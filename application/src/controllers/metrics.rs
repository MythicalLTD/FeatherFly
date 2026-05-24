use crate::{docker::collect_host_stats, routes::GetState, utils::response::ApiResponse};
use axum::response::IntoResponse;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_metrics",
    security(("bearer_auth" = [])),
    responses((status = OK, description = "Prometheus text metrics")),
)]
pub async fn get(state: GetState) -> axum::response::Response {
    let container_count = if let Some(docker) = state.docker.as_ref() {
        docker.list_containers().await.map(|c| c.len()).unwrap_or(0)
    } else {
        0
    };
    let site_count = if let Some(cache) = state.cache.as_ref() {
        cache.list_sites().await.map(|s| s.len()).unwrap_or(0)
    } else {
        0
    };

    let stats = collect_host_stats(container_count);
    let body = format!(
        "# HELP featherfly_cpu_usage_percent Host CPU usage percent\n\
         # TYPE featherfly_cpu_usage_percent gauge\n\
         featherfly_cpu_usage_percent {}\n\
         # HELP featherfly_memory_used_bytes Host memory used\n\
         # TYPE featherfly_memory_used_bytes gauge\n\
         featherfly_memory_used_bytes {}\n\
         # HELP featherfly_memory_total_bytes Host memory total\n\
         # TYPE featherfly_memory_total_bytes gauge\n\
         featherfly_memory_total_bytes {}\n\
         # HELP featherfly_container_count Running containers\n\
         # TYPE featherfly_container_count gauge\n\
         featherfly_container_count {}\n\
         # HELP featherfly_site_count Provisioned sites\n\
         # TYPE featherfly_site_count gauge\n\
         featherfly_site_count {}\n",
        stats.cpu_usage_percent,
        stats.memory_used_bytes,
        stats.memory_total_bytes,
        container_count,
        site_count,
    );

    ApiResponse::new(axum::body::Body::from(body))
        .with_header("Content-Type", "text/plain; version=0.0.4")
        .into_response()
}
