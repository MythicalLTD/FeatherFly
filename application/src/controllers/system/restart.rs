use crate::{
    routes::GetState,
    utils::{
        plugin_events::{self, RestartScheduledPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema, Deserialize)]
pub struct Payload {
    #[serde(default)]
    delay_ms: u64,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(ToSchema, Serialize)]
struct Response {
    scheduled: bool,
    delay_ms: u64,
}

#[utoipa::path(
    post,
    path = "/",
    operation_id = "post_system_restart",
    security(("bearer_auth" = [])),
    request_body = inline(Payload),
    responses(
        (status = ACCEPTED, body = inline(Response)),
        (status = FORBIDDEN, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn post(state: GetState, axum::Json(body): axum::Json<Payload>) -> ApiResponseResult {
    if !state.config.load().remote.restart {
        return ApiResponse::error("remote restart is disabled")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    if let Some(reason) = &body.reason {
        tracing::info!(reason = %reason, delay_ms = body.delay_ms, "remote restart requested");
    } else {
        tracing::info!(delay_ms = body.delay_ms, "remote restart requested");
    }

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::RestartScheduled,
        &RestartScheduledPayload {
            delay_ms: body.delay_ms,
            reason: body.reason.clone(),
        },
    );

    crate::daemon_control::schedule_restart(body.delay_ms);

    ApiResponse::new_serialized(Response {
        scheduled: true,
        delay_ms: body.delay_ms,
    })
    .with_status(StatusCode::ACCEPTED)
    .ok()
}
