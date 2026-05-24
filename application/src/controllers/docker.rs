use crate::{
    docker::DockerManager,
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize)]
pub struct Response {
    pub connected: bool,
    pub enabled: bool,
    pub socket: String,
    pub version: Option<String>,
    pub api_version: Option<String>,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_docker_status",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(Response))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let inner = state.config.load();
    let enabled = inner.docker.enabled;
    let socket = inner.docker.socket.clone();

    let Some(docker) = state.docker.as_deref() else {
        return ApiResponse::new_serialized(Response {
            connected: false,
            enabled,
            socket,
            version: None,
            api_version: None,
        })
        .ok();
    };

    let version_info = docker.version().await.ok();

    ApiResponse::new_serialized(Response {
        connected: true,
        enabled,
        socket: docker.socket().to_string(),
        version: version_info.as_ref().and_then(|v| v.version.clone()),
        api_version: version_info.and_then(|v| v.api_version),
    })
    .ok()
}

pub fn require_docker(state: &GetState) -> Result<&DockerManager, ApiResponse> {
    state.docker.as_deref().ok_or_else(|| {
        ApiResponse::error("docker is unavailable — check docker.enabled and socket path")
            .with_status(StatusCode::SERVICE_UNAVAILABLE)
    })
}
