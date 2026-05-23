use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::GetState,
    };
    use axum::http::StatusCode;
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        path: String,
        yaml: String,
        editable: bool,
    }

    #[utoipa::path(
        get,
        path = "/",
        operation_id = "get_system_config",
        security(("bearer_auth" = [])),
        responses((status = OK, body = inline(Response)), (status = FORBIDDEN, body = inline(crate::routes::ApiError))),
    )]
    pub async fn route(state: GetState) -> ApiResponseResult {
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

        ApiResponse::new_serialized(Response {
            path: state.config.path.clone(),
            yaml,
            editable: inner.remote.config_edit,
        })
        .ok()
    }
}

mod put {
    use crate::{
        config::ConfigApplyResult,
        response::{ApiResponse, ApiResponseResult},
        routes::{ApiError, GetState},
    };
    use axum::http::StatusCode;
    use serde::Deserialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        yaml: String,
        #[serde(default)]
        restart_if_required: bool,
    }

    #[derive(ToSchema, serde::Serialize)]
    struct Response {
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
        request_body = inline(Payload),
        responses(
            (status = OK, body = inline(Response)),
            (status = FORBIDDEN, body = inline(ApiError)),
            (status = BAD_REQUEST, body = inline(ApiError)),
        ),
    )]
    pub async fn route(
        state: GetState,
        axum::Json(body): axum::Json<Payload>,
    ) -> ApiResponseResult {
        if !state.config.load().remote.config_edit {
            return ApiResponse::error("remote config edit is disabled")
                .with_status(StatusCode::FORBIDDEN)
                .ok();
        }

        let ConfigApplyResult {
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
            crate::daemon_control::schedule_restart(750);
        }

        tracing::info!(
            requires_restart,
            restart_scheduled,
            reasons = ?restart_reasons,
            "configuration updated via API"
        );

        ApiResponse::new_serialized(Response {
            applied: true,
            requires_restart,
            restart_reasons,
            restart_scheduled,
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(put::route))
        .with_state(state.clone())
}
