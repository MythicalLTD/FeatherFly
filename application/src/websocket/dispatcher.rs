use featherfly_plugin_sdk::PluginEvent;

use crate::{
    routes::State,
    utils::plugin_events::{self, PanelMessagePayload},
    websocket::messages::{PanelMessage, PanelResponse},
};

pub async fn dispatch(state: &State, message: PanelMessage) -> PanelResponse {
    let correlation_id = message.correlation_id.clone();
    let message_type = message.message_type.clone();

    if let Some(ref id) = correlation_id
        && let Some(cache) = &state.cache
    {
        match cache.panel_message_seen(id).await {
            Ok(true) => {
                return PanelResponse {
                    message_type: format!("{message_type}.result"),
                    correlation_id,
                    ok: true,
                    error: Some("duplicate correlation_id — already handled".into()),
                };
            }
            Ok(false) => {}
            Err(err) => {
                tracing::warn!(%err, "panel cache lookup failed");
            }
        }
    }

    plugin_events::emit_state_event(
        state,
        PluginEvent::PanelMessageReceived,
        &PanelMessagePayload {
            message_type: &message_type,
            correlation_id: correlation_id.as_deref(),
            event: None,
        },
    );

    let result = match message_type.as_str() {
        "container.start" => handle_container_start(state, &message).await,
        "container.stop" => handle_container_stop(state, &message).await,
        "container.restart" => handle_container_restart(state, &message).await,
        "site.provision" => handle_site_provision(state, &message).await,
        "site.destroy" => handle_site_destroy(state, &message).await,
        "site.deploy" => handle_site_deploy(state, &message).await,
        "deploy" => Ok(()),
        other => Err(format!("unknown panel instruction: {other}")),
    };

    if let Some(ref id) = correlation_id
        && result.is_ok()
        && let Some(cache) = &state.cache
        && let Err(err) = cache.mark_panel_message_handled(id).await
    {
        tracing::warn!(%err, "failed to mark panel message handled");
    }

    match result {
        Ok(()) => PanelResponse {
            message_type: format!("{message_type}.result"),
            correlation_id,
            ok: true,
            error: None,
        },
        Err(error) => PanelResponse {
            message_type: format!("{message_type}.result"),
            correlation_id,
            ok: false,
            error: Some(error),
        },
    }
}

async fn container_id(message: &PanelMessage) -> Result<String, String> {
    message.id.clone().ok_or_else(|| "missing id field".into())
}

async fn handle_container_start(state: &State, message: &PanelMessage) -> Result<(), String> {
    let id = container_id(message).await?;
    let docker = state.docker.as_ref().ok_or("docker unavailable")?;
    docker
        .start_container(&id)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(cache) = &state.cache {
        let _ = cache
            .record_container_seen(&id, message.correlation_id.as_deref())
            .await;
    }
    plugin_events::emit_state_event(
        state,
        PluginEvent::ContainerStarted,
        &plugin_events::ContainerIdPayload { id: &id },
    );
    Ok(())
}

async fn handle_container_stop(state: &State, message: &PanelMessage) -> Result<(), String> {
    let id = container_id(message).await?;
    let docker = state.docker.as_ref().ok_or("docker unavailable")?;
    docker
        .stop_container(&id)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(cache) = &state.cache {
        let _ = cache
            .record_container_seen(&id, message.correlation_id.as_deref())
            .await;
    }
    plugin_events::emit_state_event(
        state,
        PluginEvent::ContainerStopped,
        &plugin_events::ContainerIdPayload { id: &id },
    );
    Ok(())
}

async fn handle_container_restart(state: &State, message: &PanelMessage) -> Result<(), String> {
    let id = container_id(message).await?;
    let docker = state.docker.as_ref().ok_or("docker unavailable")?;
    docker
        .restart_container(&id)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(cache) = &state.cache {
        let _ = cache
            .record_container_seen(&id, message.correlation_id.as_deref())
            .await;
    }
    plugin_events::emit_state_event(
        state,
        PluginEvent::ContainerRestarted,
        &plugin_events::ContainerIdPayload { id: &id },
    );
    Ok(())
}

async fn handle_site_provision(state: &State, message: &PanelMessage) -> Result<(), String> {
    let payload = message.payload.as_ref().ok_or("missing payload")?;
    let req: crate::sites::ProvisionSiteRequest =
        serde_json::from_value(payload.clone()).map_err(|e| e.to_string())?;
    crate::sites::SiteService::provision(state, req)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn handle_site_destroy(state: &State, message: &PanelMessage) -> Result<(), String> {
    let id = message.id.clone().ok_or("missing id")?;
    crate::sites::SiteService::destroy(state, &id, false)
        .await
        .map_err(|e| e.to_string())
}

async fn handle_site_deploy(state: &State, message: &PanelMessage) -> Result<(), String> {
    let id = message.id.clone().ok_or("missing id")?;
    crate::sites::SiteService::deploy(
        state,
        &id,
        crate::sites::DeployRequest {
            git_ref: message
                .payload
                .as_ref()
                .and_then(|p| p.get("git_ref").and_then(|v| v.as_str().map(String::from))),
            rebuild: false,
            zero_downtime: message
                .payload
                .as_ref()
                .and_then(|p| p.get("zero_downtime").and_then(|v| v.as_bool()))
                .unwrap_or(false),
        },
    )
    .await
    .map_err(|e| e.to_string())
}
