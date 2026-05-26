use std::sync::Arc;

use axum::{
    extract::{
        Path, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::RwLock;

use crate::{
    routes::{GetState, State as AppState},
    site_websocket::{
        PERMISSION_WS_CONNECT, SiteWebsocketEvent, SiteWebsocketJwtPayload, SiteWebsocketMessage,
        has_permission,
    },
};

type SocketJwt = Arc<RwLock<Option<SiteWebsocketJwtPayload>>>;

pub async fn handle_site_ws(
    ws: WebSocketUpgrade,
    state: GetState,
    Path(site_id): Path<String>,
) -> Response {
    if !site_exists(&state, &site_id).await {
        return (axum::http::StatusCode::NOT_FOUND, "site not found").into_response();
    }

    ws.on_upgrade(move |socket| handle_connection(state.0, site_id, socket))
}

async fn site_exists(state: &AppState, site_id: &str) -> bool {
    let Some(cache) = &state.cache else {
        return false;
    };
    matches!(cache.get_site(site_id).await, Ok(Some(_)))
}

async fn handle_connection(state: AppState, site_id: String, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let socket_jwt: SocketJwt = Arc::new(RwLock::new(None));

    while let Some(message) = receiver.next().await {
        let Ok(message) = message else {
            break;
        };

        if matches!(message, Message::Ping(_)) {
            let _ = sender.send(Message::Pong(vec![].into())).await;
            continue;
        }
        if matches!(message, Message::Close(_)) {
            break;
        }

        let text = match message {
            Message::Text(text) => text.to_string(),
            Message::Binary(bytes) => match String::from_utf8(bytes.to_vec()) {
                Ok(text) => text,
                Err(_) => continue,
            },
            _ => continue,
        };

        let Ok(inbound) = serde_json::from_str::<SiteWebsocketMessage>(&text) else {
            continue;
        };

        match inbound.event {
            SiteWebsocketEvent::Authentication => {
                let token = inbound.args.first().map(String::as_str).unwrap_or("");
                match authenticate(&state, &site_id, token).await {
                    Ok(jwt) => {
                        let permissions = jwt.permissions.clone();
                        *socket_jwt.write().await = Some(jwt);
                        if send_json(
                            &mut sender,
                            SiteWebsocketMessage::new(
                                SiteWebsocketEvent::AuthenticationSuccess,
                                permissions,
                            ),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    Err(reason) => {
                        let _ = send_json(
                            &mut sender,
                            SiteWebsocketMessage::new(SiteWebsocketEvent::JwtError, vec![reason]),
                        )
                        .await;
                        break;
                    }
                }
            }
            SiteWebsocketEvent::Ping => {
                if !session_valid(&state, &socket_jwt).await {
                    let _ = send_json(
                        &mut sender,
                        SiteWebsocketMessage::new(
                            SiteWebsocketEvent::JwtError,
                            vec!["authentication required".into()],
                        ),
                    )
                    .await;
                    break;
                }
                if send_json(
                    &mut sender,
                    SiteWebsocketMessage::new(SiteWebsocketEvent::Pong, vec![]),
                )
                .await
                .is_err()
                {
                    break;
                }
            }
            _ => {
                if !session_valid(&state, &socket_jwt).await {
                    let _ = send_json(
                        &mut sender,
                        SiteWebsocketMessage::new(
                            SiteWebsocketEvent::JwtError,
                            vec!["authentication required".into()],
                        ),
                    )
                    .await;
                    break;
                }
            }
        }
    }
}

async fn authenticate(
    state: &AppState,
    site_id: &str,
    token: &str,
) -> Result<SiteWebsocketJwtPayload, String> {
    let jwt: SiteWebsocketJwtPayload = state
        .jwt
        .verify(token)
        .map_err(|_| "invalid token".to_string())?;

    jwt.base
        .validate(&state.jwt)
        .await
        .map_err(|err| format!("invalid token: {err}"))?;

    if jwt.site_id != site_id {
        return Err("token site mismatch".into());
    }

    if !has_permission(&jwt.permissions, PERMISSION_WS_CONNECT) {
        return Err("missing websocket.connect permission".into());
    }

    Ok(jwt)
}

async fn session_valid(state: &AppState, socket_jwt: &SocketJwt) -> bool {
    let guard = socket_jwt.read().await;
    let Some(jwt) = guard.as_ref() else {
        return false;
    };
    jwt.base.validate(&state.jwt).await.is_ok()
        && has_permission(&jwt.permissions, PERMISSION_WS_CONNECT)
}

async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: SiteWebsocketMessage,
) -> Result<(), ()> {
    let text = serde_json::to_string(&message).map_err(|_| ())?;
    sender
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn authenticate_rejects_site_mismatch() {
        let config = crate::config::Config::for_openapi_docs("FeatherFly");
        let inner = config.load();
        let jwt_client = Arc::new(crate::remote::jwt::JwtClient::new(
            &inner.token,
            inner.api.max_jwt_uses,
        ));
        let now = chrono::Utc::now().timestamp();
        let token = jwt_client
            .create(&SiteWebsocketJwtPayload {
                base: crate::remote::jwt::BasePayload {
                    issuer: "panel".into(),
                    subject: None,
                    audience: vec![],
                    expiration_time: Some(now + 300),
                    not_before: None,
                    issued_at: Some(now),
                    jwt_id: "jti-test".into(),
                },
                site_id: "other".into(),
                user_uuid: None,
                permissions: vec![PERMISSION_WS_CONNECT.into()],
            })
            .unwrap();

        let state = Arc::new(crate::routes::AppState {
            start_time: std::time::Instant::now(),
            container_type: crate::routes::AppContainerType::None,
            version: crate::full_version(),
            config,
            plugins: crate::plugins::PluginRegistry::empty(),
            probe_guard: crate::middlewares::probe::ProbeGuard::new(),
            docker: None,
            panel: arc_swap::ArcSwapOption::from(None),
            cache: None,
            jwt: jwt_client,
        });

        let err = authenticate(&state, "demo", &token).await.unwrap_err();
        assert_eq!(err, "token site mismatch");
    }
}
