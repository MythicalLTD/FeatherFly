use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::{ApiError, GetState},
    };
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use sha2::Digest;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        sha256: String,
        restart_command: String,
        #[serde(default)]
        restart_command_args: Vec<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        applied: bool,
    }

    #[utoipa::path(
        post,
        path = "/",
        operation_id = "post_system_upgrade",
        security(("bearer_auth" = [])),
        responses(
            (status = ACCEPTED, body = inline(Response)),
            (status = BAD_REQUEST, body = inline(ApiError)),
            (status = CONFLICT, body = inline(ApiError)),
        ),
        request_body = inline(Payload),
    )]
    pub async fn route(
        state: GetState,
        axum::Json(data): axum::Json<Payload>,
    ) -> ApiResponseResult {
        if !matches!(state.container_type, crate::routes::AppContainerType::None) {
            return ApiResponse::error("upgrades are not supported in containerized environments")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        if state.config.load().updates.ignore_panel_upgrades {
            return ApiResponse::new_serialized(Response { applied: false }).ok();
        }

        let current_exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(_) => {
                return ApiResponse::error("unable to locate current executable")
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let parent = match current_exe.parent() {
            Some(parent) => parent,
            None => {
                return ApiResponse::error("unable to find parent of current executable")
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let file_name = match current_exe.file_name() {
            Some(name) => name,
            None => {
                return ApiResponse::error("unable to find file name of current executable")
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let temp_path = parent.join(format!("{}.upgrade", file_name.to_string_lossy()));

        let client = match crate::update::http_client() {
            Ok(client) => client,
            Err(_) => {
                return ApiResponse::error("failed to build HTTP client")
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                    .ok();
            }
        };

        let mut request = client.get(&data.url);
        for (key, value) in &data.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                tracing::error!("failed to download upgrade binary: {err:#}");
                return ApiResponse::error("failed to download upgrade binary")
                    .with_status(StatusCode::BAD_GATEWAY)
                    .ok();
            }
        };

        let response = match response.error_for_status() {
            Ok(response) => response,
            Err(err) => {
                tracing::error!("upgrade download returned an error: {err:#}");
                return ApiResponse::error("upgrade download returned an error")
                    .with_status(StatusCode::BAD_GATEWAY)
                    .ok();
            }
        };

        let mut options = tokio::fs::OpenOptions::new();
        options.create(true).write(true).truncate(true).read(true);
        #[cfg(unix)]
        options.mode(0o755);

        let mut file = match options.open(&temp_path).await {
            Ok(file) => file,
            Err(err) => {
                tracing::error!("failed to create temporary upgrade file: {err:#}");
                return ApiResponse::error("failed to create temporary upgrade file")
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                    .ok();
            }
        };

        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::error!("failed to read upgrade download body: {err:#}");
                return ApiResponse::error("failed to read upgrade download body")
                    .with_status(StatusCode::BAD_GATEWAY)
                    .ok();
            }
        };

        if let Err(err) = file.write_all(&bytes).await {
            tracing::error!("failed to write upgrade binary: {err:#}");
            return ApiResponse::error("failed to write upgrade binary")
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .ok();
        }

        if let Err(err) = file.sync_all().await {
            tracing::error!("failed to flush upgrade binary: {err:#}");
            return ApiResponse::error("failed to flush upgrade binary")
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .ok();
        }

        if let Err(err) = file.seek(std::io::SeekFrom::Start(0)).await {
            tracing::error!("failed to rewind upgrade binary: {err:#}");
            return ApiResponse::error("failed to verify upgrade binary")
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .ok();
        }

        let mut hasher = sha2::Sha256::new();
        let mut buffer = vec![0; 64 * 1024];

        loop {
            match file.read(&mut buffer).await {
                Ok(0) => break,
                Ok(bytes_read) => hasher.update(&buffer[..bytes_read]),
                Err(err) => {
                    tracing::error!("failed to read upgrade binary for verification: {err:#}");
                    return ApiResponse::error("failed to verify upgrade binary")
                        .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                        .ok();
                }
            }
        }

        drop(file);

        if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(&data.sha256) {
            tokio::fs::remove_file(&temp_path).await.ok();
            return ApiResponse::error("downloaded file does not match provided sha256")
                .with_status(StatusCode::CONFLICT)
                .ok();
        }

        let restart_command = data.restart_command;
        let restart_command_args = data.restart_command_args;

        tokio::spawn(async move {
            if let Err(err) = apply_upgrade(
                temp_path,
                current_exe,
                restart_command,
                restart_command_args,
            )
            .await
            {
                tracing::error!("error while applying upgrade: {err:#}");
            }
        });

        ApiResponse::new_serialized(Response { applied: true })
            .with_status(StatusCode::ACCEPTED)
            .ok()
    }

    async fn apply_upgrade(
        temp_path: std::path::PathBuf,
        current_exe: std::path::PathBuf,
        restart_command: String,
        restart_command_args: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tokio::fs::rename(&temp_path, &current_exe).await?;

        #[allow(clippy::zombie_processes)]
        std::process::Command::new(restart_command)
            .args(restart_command_args)
            .spawn()?;

        Ok(())
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
