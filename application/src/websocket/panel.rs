use std::time::Duration;

use anyhow::Context;
use featherfly_plugin_sdk::PluginEvent;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};

use crate::{
    remote::{apply_custom_headers, panel_bearer_token, panel_ws_url},
    routes::State,
    utils::plugin_events::{
        self, PanelConnectedPayload, PanelDisconnectedPayload, PanelMessagePayload,
    },
    websocket::{
        dispatcher,
        messages::{OutboundMessage, PanelMessage},
    },
};

#[derive(Clone)]
pub struct PanelHandle {
    outbound: mpsc::UnboundedSender<OutboundMessage>,
}

impl PanelHandle {
    pub fn send(&self, message: OutboundMessage) {
        let _ = self.outbound.send(message);
    }

    pub fn fanout_lifecycle(&self, event: PluginEvent, payload: &impl Serialize) {
        if let Some(message) = OutboundMessage::from_lifecycle(event.name(), payload) {
            self.send(message);
        }
    }
}

pub fn spawn_panel_client(state: State) -> PanelHandle {
    let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
    let handle = PanelHandle {
        outbound: outbound_tx,
    };

    tokio::spawn(run_panel_client(state, outbound_rx));

    handle
}

#[allow(clippy::collapsible_match)]
async fn run_panel_client(state: State, mut outbound_rx: mpsc::UnboundedReceiver<OutboundMessage>) {
    let mut backoff = Duration::from_secs(1);
    let mut connect_attempts = 0u32;

    loop {
        let inner = state.config.load();
        let remote = inner.remote.trim().to_string();
        if remote.is_empty() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        let retry_limit = inner.remote_query.retry_limit;
        let custom_headers = inner.remote_query.custom_headers.clone();

        let ws_url = match panel_ws_url(&remote) {
            Ok(url) => url,
            Err(err) => {
                tracing::warn!(%err, %remote, "invalid panel remote URL");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
                continue;
            }
        };

        let uuid = inner.uuid.clone();
        let token_id = inner.token_id.clone();
        let token = inner.token.clone();
        drop(inner);

        match connect_panel(&ws_url, &uuid, &token_id, &token, &custom_headers).await {
            Ok((mut write, mut read)) => {
                plugin_events::emit_state_event(
                    &state,
                    PluginEvent::PanelConnected,
                    &PanelConnectedPayload {
                        remote: &remote,
                        uuid: &uuid,
                    },
                );
                backoff = Duration::from_secs(1);
                connect_attempts = 0;

                loop {
                    tokio::select! {
                        outbound = outbound_rx.recv() => {
                            let Some(message) = outbound else {
                                return;
                            };
                            let (message_type, event_name) = match &message {
                                OutboundMessage::Event { event, .. } => ("event", Some(event.as_str())),
                                OutboundMessage::Response(response) => {
                                    (response.message_type.as_str(), None)
                                }
                            };
                            plugin_events::emit_json(
                                &state.plugins,
                                PluginEvent::PanelMessageSent,
                                &PanelMessagePayload {
                                    message_type,
                                    correlation_id: None,
                                    event: event_name,
                                },
                            );
                            let Ok(text) = serde_json::to_string(&message) else {
                                continue;
                            };
                            if write.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                        incoming = read.next() => {
                            let Some(incoming) = incoming else {
                                break;
                            };
                            match incoming {
                                Ok(Message::Text(text)) => {
                                    if let Ok(message) =
                                        serde_json::from_str::<PanelMessage>(&text)
                                    {
                                        let response =
                                            dispatcher::dispatch(&state, message).await;
                                        plugin_events::emit_json(
                                            &state.plugins,
                                            PluginEvent::PanelMessageSent,
                                            &PanelMessagePayload {
                                                message_type: "response",
                                                correlation_id: response.correlation_id.as_deref(),
                                                event: None,
                                            },
                                        );
                                        let outbound =
                                            OutboundMessage::Response(response);
                                        if write
                                            .send(Message::Text(
                                                serde_json::to_string(&outbound)
                                                    .unwrap_or_default()
                                                    .into(),
                                            ))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                }
                                Ok(Message::Ping(payload)) => {
                                    if write.send(Message::Pong(payload)).await.is_err() {
                                        break;
                                    }
                                }
                                Ok(Message::Close(_)) | Err(_) => break,
                                _ => {}
                            }
                        }
                    }
                }

                plugin_events::emit_state_event(
                    &state,
                    PluginEvent::PanelDisconnected,
                    &PanelDisconnectedPayload {
                        remote: &remote,
                        reason: Some("connection closed".into()),
                    },
                );
            }
            Err(err) => {
                tracing::debug!(%err, %remote, "panel websocket connect failed");
                plugin_events::emit_state_event(
                    &state,
                    PluginEvent::PanelDisconnected,
                    &PanelDisconnectedPayload {
                        remote: &remote,
                        reason: Some(err.to_string()),
                    },
                );
            }
        }

        connect_attempts += 1;
        if retry_limit > 0 && connect_attempts >= retry_limit {
            tracing::warn!(
                attempts = connect_attempts,
                %remote,
                "panel websocket reconnect limit reached — waiting before retry cycle resets"
            );
            connect_attempts = 0;
            backoff = Duration::from_secs(1);
            tokio::time::sleep(Duration::from_secs(30)).await;
            continue;
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

async fn connect_panel(
    ws_url: &str,
    uuid: &str,
    token_id: &str,
    token: &str,
    custom_headers: &std::collections::HashMap<String, String>,
) -> Result<
    (
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ),
    anyhow::Error,
> {
    let mut request = ws_url
        .into_client_request()
        .context("failed to build websocket request")?;

    let bearer = panel_bearer_token(token_id, token);
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {bearer}"))
            .context("invalid authorization token for websocket")?,
    );
    request.headers_mut().insert(
        "X-Node-Uuid",
        HeaderValue::from_str(uuid).context("invalid node uuid for websocket")?,
    );
    apply_custom_headers(request.headers_mut(), custom_headers)?;

    let (stream, _) = connect_async(request)
        .await
        .with_context(|| format!("failed to connect to panel websocket at {ws_url}"))?;

    Ok(stream.split())
}
