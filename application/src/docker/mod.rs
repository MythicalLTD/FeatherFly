mod bootstrap;
mod containers;
mod exec;
mod images;
mod install;
mod logs;
mod network;
mod paths;
mod security;
mod stats;
mod volume_shell;
mod volumes;

use anyhow::Context;
use bollard::Docker;

use crate::config::DockerConfig;

pub use bootstrap::{DockerBootstrapResult, SystemBootstrapSummary, bootstrap_system};
pub use containers::{ContainerSummary, CreateContainerRequest, CreateContainerResponse};
pub use exec::{ExecCreateRequest, ExecCreateResponse};
pub use images::{error_pages_image, hosting_images, pull_hosting_images};
pub use install::ensure_docker_engine;
pub use logs::LogsRequest;
pub use network::{
    NetworkEnsureResult, connect_container_network, ensure_bridge_network,
    ensure_named_bridge_network, ensure_site_network, site_network_name,
};
pub use paths::absolute_bind_path;
pub use security::secure_host_config;
pub use stats::{HostStatsResponse, collect_host_stats};
pub use volume_shell::{sanitize_site_path, volume_read_file, volume_run, volume_write_file};

pub struct DockerManager {
    client: Docker,
    socket: String,
    config: DockerConfig,
}

impl DockerManager {
    pub async fn connect(config: &DockerConfig) -> Result<Self, anyhow::Error> {
        let client = if config.socket == "/var/run/docker.sock" {
            Docker::connect_with_socket_defaults()
        } else {
            Docker::connect_with_unix(&config.socket, 120, bollard::API_DEFAULT_VERSION)
        }
        .with_context(|| format!("failed to connect to docker at {}", config.socket))?;

        Ok(Self {
            client,
            socket: config.socket.clone(),
            config: config.clone(),
        })
    }

    pub fn client(&self) -> &Docker {
        &self.client
    }

    pub fn socket(&self) -> &str {
        &self.socket
    }

    pub fn config(&self) -> &DockerConfig {
        &self.config
    }

    pub async fn ping(&self) -> Result<(), anyhow::Error> {
        self.client.ping().await.context("docker ping failed")?;
        Ok(())
    }

    pub async fn prepare(&self) -> Result<NetworkEnsureResult, anyhow::Error> {
        self.ping().await?;
        network::ensure_bridge_network(&self.client, &self.config.network).await
    }

    pub async fn version(&self) -> Result<bollard::system::Version, anyhow::Error> {
        self.client
            .version()
            .await
            .context("failed to read docker version")
    }
}
