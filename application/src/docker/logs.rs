use bollard::container::LogOutput;
use bollard::container::LogsOptions;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::DockerManager;

#[derive(Debug, Clone)]
pub struct LogsRequest {
    pub follow: bool,
    pub stdout: bool,
    pub stderr: bool,
    pub tail: Option<String>,
}

impl Default for LogsRequest {
    fn default() -> Self {
        Self {
            follow: false,
            stdout: true,
            stderr: true,
            tail: Some("100".into()),
        }
    }
}

impl DockerManager {
    pub fn container_logs_stream(
        &self,
        id: &str,
        request: LogsRequest,
    ) -> Result<ReceiverStream<Result<Vec<u8>, anyhow::Error>>, anyhow::Error> {
        let options = Some(LogsOptions::<String> {
            follow: request.follow,
            stdout: request.stdout,
            stderr: request.stderr,
            tail: request.tail.unwrap_or_else(|| "100".into()),
            timestamps: false,
            ..Default::default()
        });

        let mut stream = self.client().logs(id, options);
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                let chunk = match item {
                    Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) => {
                        Ok(message.to_vec())
                    }
                    Ok(LogOutput::StdIn { .. } | LogOutput::Console { .. }) => continue,
                    Err(err) => Err(anyhow::anyhow!(err).context("log stream error")),
                };

                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}
