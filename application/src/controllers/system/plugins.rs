use crate::{
    routes::GetState,
    utils::{
        plugin_events::{self, PluginReloadPayload, RestartScheduledPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::http::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
struct GetResponse {
    enabled: bool,
    directory: String,
    hooks: usize,
    schema_path: &'static str,
    events: Vec<&'static str>,
    json_targets: Vec<&'static str>,
    request_phases: Vec<&'static str>,
    cloudpanel_hooks: usize,
    plugin_routes: usize,
    load_errors: Vec<crate::plugins::PluginLoadError>,
    plugins: Vec<crate::plugins::PluginSummary>,
}

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
        cloudpanel_hooks: state.plugins.cloudpanel_hook_count(),
        plugin_routes: state.plugins.routes().len(),
        load_errors: state.plugins.load_errors(),
        plugins: state.plugins.summaries(),
    })
    .ok()
}

pub async fn schema(_state: GetState) -> ApiResponseResult {
    ApiResponse::new_serialized(crate::plugins::build_hook_schema()).ok()
}

#[derive(Serialize)]
struct ReloadResponse {
    scheduled: bool,
    delay_ms: u64,
    note: &'static str,
}

pub async fn reload(state: GetState) -> ApiResponseResult {
    let delay_ms = 750;
    let plugin_count = state.plugins.summaries().len();
    tracing::info!(
        plugins = plugin_count,
        "plugin reload requested, scheduling daemon restart"
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
