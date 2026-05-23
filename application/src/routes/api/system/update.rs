use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::GetState,
        update::UpdateStatus,
    };

    #[utoipa::path(
        get,
        path = "/",
        operation_id = "get_system_update",
        security(("bearer_auth" = [])),
        responses((status = OK, body = inline(UpdateStatus))),
    )]
    pub async fn route(state: GetState) -> ApiResponseResult {
        let channel = state.config.load().updates.channel;
        let status = crate::update::check_update_status(channel).await;
        ApiResponse::new_serialized(status).ok()
    }
}

mod post {
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::{ApiError, GetState},
        update::{ApplyUpdateResult, UpdateChannel},
    };
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    fn default_true() -> bool {
        true
    }

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        #[serde(default = "default_true")]
        restart: bool,
        #[serde(default)]
        delay_ms: u64,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
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
        request_body = inline(Payload),
        responses(
            (status = ACCEPTED, body = inline(ApplyUpdateResult)),
            (status = BAD_REQUEST, body = inline(ApiError)),
            (status = CONFLICT, body = inline(ApiError)),
            (status = FORBIDDEN, body = inline(ApiError)),
        ),
    )]
    pub async fn apply_route(
        state: GetState,
        axum::Json(body): axum::Json<Payload>,
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
            return ApiResponse::new_serialized(Response {
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

        let preview = crate::update::check_update_status(channel).await;
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

        tokio::spawn(async move {
            match crate::update::apply_update(channel).await {
                Ok(result) => {
                    tracing::info!(
                        channel = ?result.channel,
                        version = ?result.installed_version,
                        commit = ?result.installed_commit,
                        path = %result.binary_path,
                        "update installed from GitHub release"
                    );
                    if restart {
                        crate::daemon_control::schedule_restart(delay_ms);
                    }
                }
                Err(err) => {
                    tracing::error!("failed to apply update: {err:#}");
                }
            }
        });

        ApiResponse::new_serialized(Response {
            scheduled: true,
            restart,
            delay_ms,
            channel,
            update_available: true,
        })
        .with_status(StatusCode::ACCEPTED)
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(post::apply_route))
        .with_state(state.clone())
}
