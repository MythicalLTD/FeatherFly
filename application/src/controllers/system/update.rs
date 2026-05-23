use crate::{
    routes::GetState,
    utils::{
        plugin_events::{self, ErrorPayload, RestartScheduledPayload},
        response::{ApiResponse, ApiResponseResult},
        update::{ApplyUpdateResult, UpdateChannel, UpdateStatus},
    },
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_system_update",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(UpdateStatus))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let channel = state.config.load().updates.channel;
    let status = crate::utils::update::check_update_status(channel).await;
    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::UpdateChecked,
        &status,
    );
    ApiResponse::new_serialized(status).ok()
}

fn default_true() -> bool {
    true
}

#[derive(ToSchema, Deserialize)]
pub struct ApplyPayload {
    #[serde(default = "default_true")]
    restart: bool,
    #[serde(default)]
    delay_ms: u64,
}

#[derive(ToSchema, Serialize)]
struct ApplyResponse {
    scheduled: bool,
    restart: bool,
    delay_ms: u64,
    channel: UpdateChannel,
    update_available: bool,
}

#[utoipa::path(
    post,
    path = "/apply",
    operation_id = "post_system_update_apply",
    security(("bearer_auth" = [])),
    request_body = inline(ApplyPayload),
    responses(
        (status = ACCEPTED, body = inline(ApplyUpdateResult)),
        (status = BAD_REQUEST, body = inline(crate::utils::response::ApiError)),
        (status = CONFLICT, body = inline(crate::utils::response::ApiError)),
        (status = FORBIDDEN, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn apply(
    state: GetState,
    axum::Json(body): axum::Json<ApplyPayload>,
) -> ApiResponseResult {
    if !state.config.load().remote.upgrade {
        return ApiResponse::error("remote upgrade is disabled")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    if !matches!(state.container_type, crate::routes::AppContainerType::None) {
        return ApiResponse::error("upgrades are not supported in containerized environments")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    }

    if state.config.load().updates.ignore_panel_upgrades {
        return ApiResponse::new_serialized(ApplyResponse {
            scheduled: false,
            restart: false,
            delay_ms: 0,
            channel: state.config.load().updates.channel,
            update_available: false,
        })
        .ok();
    }

    let channel = state.config.load().updates.channel;
    if channel == UpdateChannel::Disabled {
        return ApiResponse::error("updates are disabled in configuration")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    }

    let preview = crate::utils::update::check_update_status(channel).await;
    if !preview.check_ok {
        return ApiResponse::error(
            preview
                .message
                .as_deref()
                .unwrap_or("failed to check for updates"),
        )
        .with_status(StatusCode::BAD_GATEWAY)
        .ok();
    }

    if !preview.update_available {
        return ApiResponse::error("FeatherFly is already up to date")
            .with_status(StatusCode::CONFLICT)
            .ok();
    }

    let restart = body.restart;
    let delay_ms = body.delay_ms;
    let plugins = state.clone();

    tokio::spawn(async move {
        match crate::utils::update::apply_update(channel).await {
            Ok(result) => {
                tracing::info!(
                    channel = ?result.channel,
                    version = ?result.installed_version,
                    commit = ?result.installed_commit,
                    path = %result.binary_path,
                    "update installed from GitHub release"
                );
                plugin_events::emit_json(
                    &plugins.plugins,
                    featherfly_plugin_sdk::PluginEvent::UpdateApplied,
                    &result,
                );
                if restart {
                    plugin_events::emit_json(
                        &plugins.plugins,
                        featherfly_plugin_sdk::PluginEvent::RestartScheduled,
                        &RestartScheduledPayload {
                            delay_ms,
                            reason: Some("update applied".into()),
                        },
                    );
                    crate::daemon_control::schedule_restart(delay_ms);
                }
            }
            Err(err) => {
                tracing::error!("failed to apply update: {err:#}");
                plugin_events::emit_json(
                    &plugins.plugins,
                    featherfly_plugin_sdk::PluginEvent::UpdateApplyFailed,
                    &ErrorPayload {
                        error: err.to_string(),
                    },
                );
            }
        }
    });

    ApiResponse::new_serialized(ApplyResponse {
        scheduled: true,
        restart,
        delay_ms,
        channel,
        update_available: true,
    })
    .with_status(StatusCode::ACCEPTED)
    .ok()
}
