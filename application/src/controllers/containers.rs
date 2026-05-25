use crate::{
    controllers::docker::require_docker,
    docker::{CreateContainerRequest, ExecCreateRequest, LogsRequest},
    routes::GetState,
    utils::{
        plugin_events::{
            self, ContainerCreatedPayload, ContainerExecEndedPayload, ContainerExecStartedPayload,
            ContainerIdPayload, ContainerLogsPayload, ContainerRemovedPayload, ImagePulledPayload,
            StatsCollectedPayload,
        },
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use featherfly_plugin_sdk::PluginEvent;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_containers",
    security(("bearer_auth" = [])),
    responses(
        (status = OK, body = inline(Vec<crate::docker::ContainerSummary>)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn list(state: GetState) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.list_containers().await {
        Ok(containers) => ApiResponse::new_serialized(containers).ok(),
        Err(err) => {
            plugin_events::emit_container_failure(&state, "-", "list", &err);
            ApiResponse::error(&format!("failed to list containers: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[derive(ToSchema, Deserialize)]
pub struct CreatePayload {
    #[serde(flatten)]
    pub spec: CreateContainerRequest,
    #[serde(default = "default_true")]
    pub start: bool,
}

fn default_true() -> bool {
    true
}

#[utoipa::path(
    post,
    path = "/create",
    operation_id = "post_containers_create",
    security(("bearer_auth" = [])),
    request_body = inline(CreatePayload),
    responses(
        (status = CREATED, body = inline(crate::docker::CreateContainerResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
        (status = BAD_GATEWAY, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn create(
    state: GetState,
    axum::Json(body): axum::Json<CreatePayload>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.create_container(&body.spec, body.start).await {
        Ok(response) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerCreated,
                &ContainerCreatedPayload {
                    id: &response.id,
                    name: &response.name,
                    image: &body.spec.image,
                    started: response.started,
                },
            );
            if response.started {
                plugin_events::emit_state_event(
                    &state,
                    PluginEvent::ContainerStarted,
                    &ContainerIdPayload { id: &response.id },
                );
            }
            ApiResponse::new_serialized(response)
                .with_status(StatusCode::CREATED)
                .ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &body.spec.name, "create", &err);
            ApiResponse::error(&format!("failed to create container: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "get_containers_inspect",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = OK, body = inline(serde_json::Value)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn inspect(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.inspect_container(&id).await {
        Ok(inspect) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerInspected,
                &ContainerIdPayload { id: &id },
            );
            match serde_json::to_value(inspect) {
                Ok(value) => ApiResponse::new_serialized(value).ok(),
                Err(err) => ApiResponse::error(&format!("failed to serialize inspect: {err}"))
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                    .ok(),
            }
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "inspect", &err);
            ApiResponse::error(&format!("failed to inspect container: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[utoipa::path(
    post,
    path = "/{id}/restart",
    operation_id = "post_containers_restart",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = OK, body = inline(ActionResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn restart(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.restart_container(&id).await {
        Ok(()) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerRestarted,
                &ContainerIdPayload { id: &id },
            );
            ApiResponse::new_serialized(ActionResponse {
                id,
                action: "restart",
                ok: true,
            })
            .ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "restart", &err);
            ApiResponse::error(&format!("failed to restart container: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[derive(ToSchema, Deserialize)]
pub struct PullPayload {
    pub image: Option<String>,
}

#[utoipa::path(
    post,
    path = "/{id}/pull",
    operation_id = "post_containers_pull",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    request_body = inline(PullPayload),
    responses(
        (status = OK, body = inline(ActionResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn pull(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<PullPayload>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    let image = match body.image {
        Some(image) => image,
        None => {
            let inspect = match docker.inspect_container(&id).await {
                Ok(inspect) => inspect,
                Err(err) => {
                    plugin_events::emit_container_failure(&state, &id, "pull", &err);
                    return ApiResponse::error(&format!("failed to inspect container: {err:#}"))
                        .with_status(StatusCode::BAD_GATEWAY)
                        .ok();
                }
            };
            inspect
                .config
                .and_then(|config| config.image)
                .unwrap_or_default()
        }
    };

    if image.is_empty() {
        return ApiResponse::error("no image specified and container has no configured image")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    }

    match docker.pull_image(&image).await {
        Ok(()) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ImagePulled,
                &ImagePulledPayload {
                    image: &image,
                    container_id: Some(&id),
                },
            );
            ApiResponse::new_serialized(ActionResponse {
                id,
                action: "pull",
                ok: true,
            })
            .ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "pull", &err);
            ApiResponse::error(&format!("failed to pull image: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[derive(ToSchema, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub follow: bool,
    #[serde(default = "default_true")]
    pub stdout: bool,
    #[serde(default = "default_true")]
    pub stderr: bool,
    pub tail: Option<String>,
}

#[utoipa::path(
    get,
    path = "/{id}/logs",
    operation_id = "get_containers_logs",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Container ID or name"),
        ("follow" = bool, Query, description = "Follow log output"),
        ("stdout" = bool, Query, description = "Include stdout"),
        ("stderr" = bool, Query, description = "Include stderr"),
        ("tail" = String, Query, description = "Number of lines from tail"),
    ),
    responses(
        (status = OK, description = "Log stream"),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn logs(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<LogsQuery>,
) -> Result<Response, ApiResponse> {
    let docker = require_docker(&state)?;

    let request = LogsRequest {
        follow: query.follow,
        stdout: query.stdout,
        stderr: query.stderr,
        tail: query.tail.clone(),
    };

    let stream = match docker.container_logs_stream(&id, request.clone()) {
        Ok(stream) => stream,
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "logs", &err);
            return Err(ApiResponse::error(&format!("failed to open logs: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY));
        }
    };

    plugin_events::emit_state_event(
        &state,
        PluginEvent::ContainerLogsStreamed,
        &ContainerLogsPayload {
            id: &id,
            follow: query.follow,
            tail: query.tail,
        },
    );

    let body_stream = stream.map(|result| {
        result
            .map(axum::body::Bytes::from)
            .map_err(std::io::Error::other)
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .body(Body::from_stream(body_stream))
        .unwrap())
}

#[derive(ToSchema, Deserialize)]
pub struct ExecPayload {
    #[serde(default = "default_shell_cmd")]
    pub cmd: Vec<String>,
    #[serde(default = "default_true")]
    pub tty: bool,
}

fn default_shell_cmd() -> Vec<String> {
    vec!["/bin/sh".into()]
}

#[utoipa::path(
    post,
    path = "/{id}/exec",
    operation_id = "post_containers_exec",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    request_body = inline(ExecPayload),
    responses(
        (status = OK, body = inline(crate::docker::ExecCreateResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn exec_create(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<ExecPayload>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    let request = ExecCreateRequest {
        cmd: body.cmd,
        tty: body.tty,
    };

    match docker.create_exec(&id, request).await {
        Ok(response) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerExecStarted,
                &ContainerExecStartedPayload {
                    container_id: &response.container_id,
                    exec_id: &response.exec_id,
                    tty: response.tty,
                },
            );
            ApiResponse::new_serialized(response).ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "exec", &err);
            ApiResponse::error(&format!("failed to create exec: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[derive(ToSchema, Deserialize)]
pub struct ExecWsQuery {
    pub exec_id: String,
    #[serde(default = "default_true")]
    pub tty: bool,
}

#[utoipa::path(
    get,
    path = "/{id}/exec/ws",
    operation_id = "get_containers_exec_ws",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Container ID or name"),
        ("exec_id" = String, Query, description = "Exec session ID from POST /api/containers/{id}/exec"),
        ("tty" = bool, Query, description = "Attach in TTY mode"),
    ),
    responses(
        (status = 101, description = "WebSocket upgraded"),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn exec_ws(
    ws: WebSocketUpgrade,
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<ExecWsQuery>,
) -> Result<impl IntoResponse, ApiResponse> {
    require_docker(&state)?;
    let docker = state.docker.clone().ok_or_else(|| {
        ApiResponse::error("docker is unavailable — check docker.enabled and socket path")
            .with_status(StatusCode::SERVICE_UNAVAILABLE)
    })?;
    let exec_id = query.exec_id.clone();
    let tty = query.tty;
    let state = state.0.clone();

    Ok(ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_exec_ws(socket, docker.as_ref(), &id, &exec_id, tty, &state).await
        {
            tracing::debug!(%err, container_id = %id, exec_id = %exec_id, "exec websocket ended");
        }
        plugin_events::emit_state_event(
            &state,
            PluginEvent::ContainerExecEnded,
            &ContainerExecEndedPayload {
                container_id: &id,
                exec_id: &exec_id,
            },
        );
    }))
}

async fn handle_exec_ws(
    socket: WebSocket,
    docker: &crate::docker::DockerManager,
    container_id: &str,
    exec_id: &str,
    tty: bool,
    state: &crate::routes::AppState,
) -> Result<(), anyhow::Error> {
    let (stdin_tx, mut output) = docker.attach_exec(exec_id, tty).await?;
    let (mut ws_tx, mut ws_rx) = socket.split();

    let stdin_handle = {
        let stdin_tx = stdin_tx.clone();
        tokio::spawn(async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                match message {
                    Message::Text(text) => {
                        let _ = stdin_tx.send(text.as_bytes().to_vec()).await;
                    }
                    Message::Binary(bytes) => {
                        let _ = stdin_tx.send(bytes.to_vec()).await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        })
    };

    while let Some(chunk) = output.next().await {
        match chunk {
            Ok(bytes) => {
                if ws_tx.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            Err(err) => {
                plugin_events::emit_container_failure(state, container_id, "exec", &err);
                break;
            }
        }
    }

    stdin_handle.abort();
    Ok(())
}

#[utoipa::path(
    get,
    path = "/{id}/stats",
    operation_id = "get_containers_stats",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = OK, body = inline(serde_json::Value)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn container_stats(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.container_stats_snapshot(&id).await {
        Ok(stats) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::StatsCollected,
                &StatsCollectedPayload {
                    scope: "container",
                    id: Some(&id),
                },
            );
            match serde_json::to_value(stats) {
                Ok(value) => ApiResponse::new_serialized(value).ok(),
                Err(err) => ApiResponse::error(&format!("failed to serialize stats: {err}"))
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                    .ok(),
            }
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "stats", &err);
            ApiResponse::error(&format!("failed to read container stats: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[utoipa::path(
    post,
    path = "/{id}/start",
    operation_id = "post_containers_start",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = OK, body = inline(ActionResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn start(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.start_container(&id).await {
        Ok(()) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerStarted,
                &ContainerIdPayload { id: &id },
            );
            ApiResponse::new_serialized(ActionResponse {
                id,
                action: "start",
                ok: true,
            })
            .ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "start", &err);
            ApiResponse::error(&format!("failed to start container: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[utoipa::path(
    post,
    path = "/{id}/stop",
    operation_id = "post_containers_stop",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = OK, body = inline(ActionResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn stop(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.stop_container(&id).await {
        Ok(()) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerStopped,
                &ContainerIdPayload { id: &id },
            );
            ApiResponse::new_serialized(ActionResponse {
                id,
                action: "stop",
                ok: true,
            })
            .ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "stop", &err);
            ApiResponse::error(&format!("failed to stop container: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ActionResponse {
    pub id: String,
    pub action: &'static str,
    pub ok: bool,
}

#[derive(ToSchema, Deserialize)]
pub struct RemoveQuery {
    #[serde(default)]
    pub force: bool,
}

#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "delete_containers",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Container ID or name"),
        ("force" = bool, Query, description = "Force removal"),
    ),
    responses(
        (status = OK, body = inline(ActionResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn remove(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<RemoveQuery>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(docker) => docker,
        Err(response) => return Err(response),
    };

    match docker.remove_container(&id, query.force).await {
        Ok(()) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ContainerRemoved,
                &ContainerRemovedPayload {
                    id: &id,
                    force: query.force,
                },
            );
            ApiResponse::new_serialized(ActionResponse {
                id,
                action: "remove",
                ok: true,
            })
            .ok()
        }
        Err(err) => {
            plugin_events::emit_container_failure(&state, &id, "remove", &err);
            ApiResponse::error(&format!("failed to remove container: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}
