use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::actions::{ApiAction, plugins_actions};
    use crate::{
        plugins::PluginSummary,
        response::{ApiResponse, ApiResponseResult},
        routes::GetState,
    };
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        enabled: bool,
        directory: String,
        hooks: usize,
        events: Vec<&'static str>,
        json_targets: Vec<&'static str>,
        request_phases: Vec<&'static str>,
        plugin_routes: usize,
        plugins: Vec<PluginSummary>,
        actions: Vec<ApiAction>,
    }

    #[utoipa::path(
        get,
        path = "/",
        operation_id = "get_system_plugins",
        security(("bearer_auth" = [])),
        responses((status = OK, body = inline(Response))),
    )]
    pub async fn route(state: GetState) -> ApiResponseResult {
        let inner = state.config.load();

        ApiResponse::new_serialized(Response {
            enabled: inner.plugins.enabled,
            directory: inner.system.plugins_directory.clone(),
            hooks: state.plugins.hook_count(),
            events: featherfly_plugin_sdk::PluginEvent::all_names(),
            json_targets: featherfly_plugin_sdk::JsonMutateTarget::all_names(),
            request_phases: vec![
                featherfly_plugin_sdk::RequestHookPhase::Intercept.name(),
                featherfly_plugin_sdk::RequestHookPhase::Middleware.name(),
            ],
            plugin_routes: state.plugins.routes().len(),
            plugins: state.plugins.summaries(),
            actions: plugins_actions(),
        })
        .ok()
    }
}

mod post {
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::{ApiError, GetState},
    };
    use axum::http::StatusCode;
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        scheduled: bool,
        delay_ms: u64,
        note: &'static str,
    }

    #[utoipa::path(
        post,
        path = "/reload",
        operation_id = "post_system_plugins_reload",
        security(("bearer_auth" = [])),
        responses(
            (status = ACCEPTED, body = inline(Response)),
            (status = FORBIDDEN, body = inline(ApiError)),
        ),
    )]
    pub async fn route(state: GetState) -> ApiResponseResult {
        if !state.config.load().remote.restart {
            return ApiResponse::error("remote restart is disabled (required for plugin reload)")
                .with_status(StatusCode::FORBIDDEN)
                .ok();
        }

        let delay_ms = 750;
        tracing::info!(
            plugins = state.plugins.summaries().len(),
            "plugin reload requested — scheduling daemon restart"
        );

        crate::daemon_control::schedule_restart(delay_ms);

        ApiResponse::new_serialized(Response {
            scheduled: true,
            delay_ms,
            note: "plugins are loaded at startup; a daemon restart is required to pick up changes",
        })
        .with_status(StatusCode::ACCEPTED)
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(post::route))
        .with_state(state.clone())
}
