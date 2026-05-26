use crate::{
    docker::DockerManager,
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema, Deserialize, Serialize)]
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
        ApiResponse::error(docker_unavailable_message(
            state.config.load().docker.enabled,
        ))
        .with_status(StatusCode::SERVICE_UNAVAILABLE)
    })
}

fn docker_unavailable_message(enabled: bool) -> &'static str {
    if enabled {
        "docker is unavailable — check socket path and daemon status"
    } else {
        "docker integration is disabled"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_state(docker_enabled: bool) -> crate::routes::State {
        let config = crate::config::Config::for_openapi_docs("FeatherFly");
        let mut inner = config.load().as_ref().clone();
        inner.docker.enabled = docker_enabled;
        config.inner.store(Arc::new(inner.clone()));
        let jwt = Arc::new(crate::remote::jwt::JwtClient::new(
            &inner.token,
            inner.api.max_jwt_uses,
        ));

        Arc::new(crate::routes::AppState {
            start_time: std::time::Instant::now(),
            container_type: crate::routes::AppContainerType::None,
            version: crate::full_version(),
            config,
            plugins: crate::plugins::PluginRegistry::empty(),
            probe_guard: crate::middlewares::probe::ProbeGuard::new(),
            docker: None,
            panel: arc_swap::ArcSwapOption::from(None),
            cache: None,
            jwt,
        })
    }

    #[test]
    fn require_docker_reports_disabled_configuration() {
        let state = axum::extract::State(test_state(false));
        let Err(response) = require_docker(&state) else {
            panic!("expected docker to be unavailable");
        };

        assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            docker_unavailable_message(state.config.load().docker.enabled),
            "docker integration is disabled"
        );
    }

    #[test]
    fn require_docker_reports_enabled_but_unavailable() {
        let state = axum::extract::State(test_state(true));
        let Err(response) = require_docker(&state) else {
            panic!("expected docker to be unavailable");
        };

        assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            docker_unavailable_message(state.config.load().docker.enabled),
            "docker is unavailable — check socket path and daemon status"
        );
    }

    #[test]
    fn docker_status_reports_disabled_and_disconnected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let state = axum::extract::State(test_state(false));
            let Ok(response) = get(state).await else {
                panic!("expected docker status response");
            };

            assert_eq!(response.status, StatusCode::OK);
            let body = axum::body::to_bytes(response.body, 1024).await.unwrap();
            let status: Response = serde_json::from_slice(&body).unwrap();
            assert!(!status.connected);
            assert!(!status.enabled);
            assert_eq!(status.version, None);
            assert_eq!(status.api_version, None);
        });
    }
}
