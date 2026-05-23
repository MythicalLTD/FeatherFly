use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::{
        plugins::PluginSummary,
        response::{ApiResponse, ApiResponseResult},
        routes::GetState,
    };
    use crate::actions::{plugins_actions, ApiAction};
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        enabled: bool,
        directory: String,
        hooks: usize,
        events: Vec<&'static str>,
        json_targets: Vec<&'static str>,
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
            plugins: state.plugins.summaries(),
            actions: plugins_actions(),
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
