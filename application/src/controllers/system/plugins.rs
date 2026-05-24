use crate::{
    routes::GetState,
    utils::{
        actions::{ApiAction, plugins_actions},
        plugin_events::{self, PluginReloadPayload, RestartScheduledPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize)]
struct GetResponse {
    enabled: bool,
    directory: String,
    hooks: usize,
    /// Full hook catalog: GET /api/system/plugins/schema
    pub schema_path: &'static str,
    events: Vec<&'static str>,
    json_targets: Vec<&'static str>,
    request_phases: Vec<&'static str>,
    plugin_routes: usize,
    plugins: Vec<crate::plugins::PluginSummary>,
    actions: Vec<ApiAction>,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_system_plugins",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(GetResponse))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let inner = state.config.load();

    ApiResponse::new_serialized(GetResponse {
        enabled: inner.plugins.enabled,
        directory: inner.system.plugins_directory.clone(),
        hooks: state.plugins.hook_count(),
        schema_path: "/api/system/plugins/schema",
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

#[utoipa::path(
    get,
    path = "/schema",
    operation_id = "get_system_plugins_schema",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(crate::plugins::PluginHookSchema))),
)]
pub async fn schema(_state: GetState) -> ApiResponseResult {
    ApiResponse::new_serialized(crate::plugins::build_hook_schema()).ok()
}

#[derive(ToSchema, Serialize)]
struct ReloadResponse {
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
        (status = ACCEPTED, body = inline(ReloadResponse)),
        (status = FORBIDDEN, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn reload(state: GetState) -> ApiResponseResult {
    if !state.config.load().management.restart {
        return ApiResponse::error("remote restart is disabled (required for plugin reload)")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    let delay_ms = 750;
    let plugin_count = state.plugins.summaries().len();
    tracing::info!(
        plugins = plugin_count,
        "plugin reload requested — scheduling daemon restart"
    );

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::PluginReloadRequested,
        &PluginReloadPayload {
            plugin_count,
            delay_ms,
        },
    );

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::RestartScheduled,
        &RestartScheduledPayload {
            delay_ms,
            reason: Some("plugin reload".into()),
        },
    );

    crate::daemon_control::schedule_restart(delay_ms);

    ApiResponse::new_serialized(ReloadResponse {
        scheduled: true,
        delay_ms,
        note: "plugins are loaded at startup; a daemon restart is required to pick up changes",
    })
    .with_status(StatusCode::ACCEPTED)
    .ok()
}
