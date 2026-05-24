use std::sync::Arc;

use crate::config::InnerConfig;
use crate::proxy::TraefikEnsureResult;

use super::{
    DockerManager, NetworkEnsureResult, ensure_docker_engine, ensure_named_bridge_network,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemBootstrapSummary {
    pub docker_network: NetworkEnsureResult,
    pub proxy_network: Option<NetworkEnsureResult>,
    pub traefik: TraefikEnsureResult,
    pub docker_installed: bool,
}

pub struct DockerBootstrapResult {
    pub manager: Arc<DockerManager>,
    pub summary: SystemBootstrapSummary,
}

async fn connect_manager(
    config: &crate::config::DockerConfig,
) -> Result<DockerManager, anyhow::Error> {
    DockerManager::connect(config).await
}

/// Connect to Docker and ensure **system-only** infrastructure at boot:
/// bridge network(s) + Traefik when `proxy.auto_provision` is enabled.
///
/// Site containers (web, mail, FTP, databases) are **not** started here — only via
/// provision/sync API calls so boot stays predictable.
pub async fn bootstrap_system(
    inner: &InnerConfig,
) -> Result<Option<DockerBootstrapResult>, anyhow::Error> {
    if !inner.docker.enabled {
        tracing::debug!("docker integration disabled in configuration");
        return Ok(None);
    }

    let socket = inner.docker.socket.clone();
    tracing::info!(socket = %socket, "checking docker engine");

    let mut docker_installed = false;
    let manager = match connect_manager(&inner.docker).await {
        Ok(m) => m,
        Err(connect_err) => {
            tracing::warn!(%connect_err, "docker not reachable — attempting auto-install if enabled");
            match ensure_docker_engine(&socket, inner.docker.auto_install).await {
                Ok(true) => docker_installed = true,
                Ok(false) => {}
                Err(install_err) => {
                    tracing::warn!(%install_err, "docker auto-install failed");
                }
            }
            match connect_manager(&inner.docker).await {
                Ok(m) => m,
                Err(err) => {
                    tracing::warn!(%err, "docker unavailable — container APIs disabled");
                    return Ok(None);
                }
            }
        }
    };

    manager.ping().await.map_err(|err| {
        tracing::warn!(%err, "docker ping failed — container APIs disabled");
        err
    })?;

    tracing::info!("docker engine reachable — ensuring system containers only");

    let docker_network = manager.prepare().await.map_err(|err| {
        tracing::warn!(%err, "docker network setup failed — container APIs disabled");
        err
    })?;

    let proxy_network = if inner.proxy.enabled && inner.proxy.network != inner.docker.network.name {
        Some(
            ensure_named_bridge_network(manager.client(), &inner.proxy.network)
                .await
                .map_err(|err| {
                    tracing::warn!(%err, "proxy network setup failed");
                    err
                })?,
        )
    } else {
        None
    };

    let traefik = match crate::proxy::ensure_traefik(&manager, inner).await {
        Ok(result) => {
            match result {
                TraefikEnsureResult::Created => {
                    tracing::info!("auto-provisioned Traefik reverse proxy");
                }
                TraefikEnsureResult::Restarted => {
                    tracing::info!("restarted Traefik reverse proxy");
                }
                TraefikEnsureResult::AlreadyRunning => {
                    tracing::debug!("Traefik reverse proxy already running");
                }
                TraefikEnsureResult::Skipped => {
                    tracing::debug!("Traefik auto-provision skipped by configuration");
                }
            }
            result
        }
        Err(err) => {
            tracing::warn!(%err, "Traefik auto-provision failed");
            TraefikEnsureResult::Skipped
        }
    };

    tracing::debug!("site containers reconcile via hosting.reconcile_on_startup");

    Ok(Some(DockerBootstrapResult {
        manager: Arc::new(manager),
        summary: SystemBootstrapSummary {
            docker_network,
            proxy_network,
            traefik,
            docker_installed,
        },
    }))
}
