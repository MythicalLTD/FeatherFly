use crate::{
    routes::GetState,
    utils::{
        plugin_events::{self, ConfigExportedPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize)]
struct GetResponse {
    path: String,
    yaml: String,
    editable: bool,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_system_config",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(GetResponse)), (status = FORBIDDEN, body = inline(crate::utils::response::ApiError))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let inner = state.config.load();
    if !inner.remote.config_edit {
        return ApiResponse::error("remote config read is disabled")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    let yaml = match state.config.export_yaml_redacted() {
        Ok(yaml) => yaml,
        Err(err) => {
            return ApiResponse::error(&format!("failed to export config: {err:#}"))
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .ok();
        }
    };

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::ConfigExported,
        &ConfigExportedPayload {
            path: state.config.path.clone(),
        },
    );

    ApiResponse::new_serialized(GetResponse {
        path: state.config.path.clone(),
        yaml,
        editable: inner.remote.config_edit,
    })
    .ok()
}

#[derive(ToSchema, serde::Deserialize)]
pub struct PutPayload {
    yaml: String,
    #[serde(default)]
    restart_if_required: bool,
}

#[derive(ToSchema, serde::Serialize)]
struct PutResponse {
    applied: bool,
    requires_restart: bool,
    restart_reasons: Vec<String>,
    restart_scheduled: bool,
}

#[utoipa::path(
    put,
    path = "/",
    operation_id = "put_system_config",
    security(("bearer_auth" = [])),
    request_body = inline(PutPayload),
    responses(
        (status = OK, body = inline(PutResponse)),
        (status = FORBIDDEN, body = inline(crate::utils::response::ApiError)),
        (status = BAD_REQUEST, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn put(state: GetState, axum::Json(body): axum::Json<PutPayload>) -> ApiResponseResult {
    if !state.config.load().remote.config_edit {
        return ApiResponse::error("remote config edit is disabled")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    let crate::config::ConfigApplyResult {
        requires_restart,
        restart_reasons,
    } = match state.config.apply_yaml(&body.yaml) {
        Ok(result) => result,
        Err(err) => {
            return ApiResponse::error(&format!("invalid configuration: {err:#}"))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }
    };

    let restart_scheduled =
        body.restart_if_required && requires_restart && state.config.load().remote.restart;

    if restart_scheduled {
        plugin_events::emit_json(
            &state.plugins,
            featherfly_plugin_sdk::PluginEvent::RestartScheduled,
            &plugin_events::RestartScheduledPayload {
                delay_ms: 750,
                reason: Some("config apply requires restart".into()),
            },
        );
        crate::daemon_control::schedule_restart(750);
    }

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::ConfigApplied,
        &plugin_events::ConfigAppliedPayload {
            path: state.config.path.clone(),
            requires_restart,
            restart_reasons: restart_reasons.clone(),
            restart_scheduled,
        },
    );

    tracing::info!(
        requires_restart,
        restart_scheduled,
        reasons = ?restart_reasons,
        "configuration updated via API"
    );

    ApiResponse::new_serialized(PutResponse {
        applied: true,
        requires_restart,
        restart_reasons,
        restart_scheduled,
    })
    .ok()
}
