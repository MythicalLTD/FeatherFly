use anyhow::Context;
use bollard::container::LogOutput;
use bollard::exec::{StartExecOptions, StartExecResults};
use futures_util::StreamExt;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use utoipa::ToSchema;

use super::DockerManager;

#[derive(Debug, Clone)]
pub struct ExecCreateRequest {
    pub cmd: Vec<String>,
    pub tty: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExecCreateResponse {
    pub exec_id: String,
    pub container_id: String,
    pub tty: bool,
}

impl DockerManager {
    pub async fn create_exec(
        &self,
        container_id: &str,
        request: ExecCreateRequest,
    ) -> Result<ExecCreateResponse, anyhow::Error> {
        use bollard::exec::CreateExecOptions;

        let options = CreateExecOptions {
            cmd: Some(request.cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(!request.tty),
            attach_stdin: Some(true),
            tty: Some(request.tty),
            ..Default::default()
        };

        let response = self
            .client()
            .create_exec(container_id, options)
            .await
            .with_context(|| format!("failed to create exec for container {container_id}"))?;

        Ok(ExecCreateResponse {
            exec_id: response.id,
            container_id: container_id.to_string(),
            tty: request.tty,
        })
    }

    pub async fn attach_exec(
        &self,
        exec_id: &str,
        tty: bool,
    ) -> Result<
        (
            mpsc::Sender<Vec<u8>>,
            ReceiverStream<Result<Vec<u8>, anyhow::Error>>,
        ),
        anyhow::Error,
    > {
        let start_options = StartExecOptions {
            detach: false,
            tty,
            ..Default::default()
        };

        let start_result = self
            .client()
            .start_exec(exec_id, Some(start_options))
            .await
            .with_context(|| format!("failed to start exec {exec_id}"))?;

        match start_result {
            StartExecResults::Attached {
                mut output,
                mut input,
            } => {
                let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(32);
                let (out_tx, out_rx) = mpsc::channel(64);

                tokio::spawn(async move {
                    while let Some(chunk) = stdin_rx.recv().await {
                        if input.write_all(&chunk).await.is_err() {
                            break;
                        }
                    }
                });

                tokio::spawn(async move {
                    while let Some(item) = output.next().await {
                        let chunk = item
                            .map(|frame| log_output_bytes(frame))
                            .map_err(|err| anyhow::anyhow!(err).context("exec stream error"));
                        if out_tx.send(chunk).await.is_err() {
                            break;
                        }
                    }
                });

                Ok((stdin_tx, ReceiverStream::new(out_rx)))
            }
            StartExecResults::Detached => {
                anyhow::bail!("exec started detached unexpectedly");
            }
        }
    }

    pub async fn exec_run(
        &self,
        container_id: &str,
        cmd: Vec<String>,
    ) -> Result<String, anyhow::Error> {
        let exec = self
            .create_exec(container_id, ExecCreateRequest { cmd, tty: false })
            .await?;
        let (_stdin, mut stream) = self.attach_exec(&exec.exec_id, false).await?;
        drop(_stdin);
        let mut output = String::new();
        while let Some(chunk) = stream.next().await {
            output.push_str(&String::from_utf8_lossy(&chunk?));
        }
        Ok(output)
    }
}

fn log_output_bytes(frame: LogOutput) -> Vec<u8> {
    frame.as_ref().to_vec()
}
