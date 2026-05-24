use crate::{
    controllers::docker::require_docker,
    docker::collect_host_stats,
    routes::GetState,
    utils::{
        plugin_events::{self, StatsCollectedPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};
use featherfly_plugin_sdk::PluginEvent;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_stats",
    security(("bearer_auth" = [])),
    responses(
        (status = OK, body = inline(crate::docker::HostStatsResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let container_count = match require_docker(&state) {
        Ok(docker) => docker.list_containers().await.map(|c| c.len()).unwrap_or(0),
        Err(_) => 0,
    };

    let stats = collect_host_stats(container_count);

    plugin_events::emit_state_event(
        &state,
        PluginEvent::StatsCollected,
        &StatsCollectedPayload {
            scope: "host",
            id: None,
        },
    );

    ApiResponse::new_serialized(stats).ok()
}
