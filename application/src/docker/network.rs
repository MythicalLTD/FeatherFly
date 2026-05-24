use std::collections::HashMap;

use anyhow::Context;
use bollard::models::{Ipam, IpamConfig};
use bollard::network::{CreateNetworkOptions, ListNetworksOptions};

use crate::config::DockerNetworkConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEnsureResult {
    AlreadyExists,
    Created,
}

pub async fn ensure_bridge_network(
    client: &bollard::Docker,
    cfg: &DockerNetworkConfig,
) -> Result<NetworkEnsureResult, anyhow::Error> {
    let mut filters = HashMap::new();
    filters.insert("name", vec![cfg.name.as_str()]);

    let existing = client
        .list_networks(Some(ListNetworksOptions { filters }))
        .await
        .context("failed to list docker networks")?;

    if existing.iter().any(|network| {
        network.name.as_deref() == Some(cfg.name.as_str())
            || network
                .id
                .as_deref()
                .is_some_and(|id| id == cfg.name || id.starts_with(&cfg.name))
    }) {
        tracing::debug!(network = %cfg.name, "docker bridge network already exists");
        return Ok(NetworkEnsureResult::AlreadyExists);
    }

    let ipam_config = IpamConfig {
        subnet: Some(cfg.subnet.clone()),
        gateway: Some(cfg.interface.clone()),
        ..Default::default()
    };

    let options = CreateNetworkOptions {
        name: cfg.name.as_str(),
        check_duplicate: true,
        driver: cfg.driver.as_str(),
        internal: cfg.is_internal,
        enable_ipv6: false,
        ipam: Ipam {
            config: Some(vec![ipam_config]),
            ..Default::default()
        },
        options: if cfg.enable_icc {
            HashMap::new()
        } else {
            HashMap::from([("com.docker.network.bridge.enable_icc", "false")])
        },
        ..Default::default()
    };

    client
        .create_network(options)
        .await
        .with_context(|| format!("failed to create docker network {}", cfg.name))?;

    tracing::info!(
        network = %cfg.name,
        driver = %cfg.driver,
        subnet = %cfg.subnet,
        gateway = %cfg.interface,
        "created docker bridge network"
    );

    Ok(NetworkEnsureResult::Created)
}

pub async fn ensure_site_network(
    client: &bollard::Docker,
    site_id: &str,
) -> Result<(String, NetworkEnsureResult), anyhow::Error> {
    use bollard::network::CreateNetworkOptions;

    let name = site_network_name(site_id);
    let mut filters = HashMap::new();
    filters.insert("name", vec![name.as_str()]);

    let existing = client
        .list_networks(Some(ListNetworksOptions { filters }))
        .await
        .context("failed to list docker networks")?;

    if existing
        .iter()
        .any(|n| n.name.as_deref() == Some(name.as_str()))
    {
        return Ok((name, NetworkEnsureResult::AlreadyExists));
    }

    let options = CreateNetworkOptions {
        name: name.as_str(),
        check_duplicate: true,
        driver: "bridge",
        internal: false,
        enable_ipv6: false,
        ..Default::default()
    };

    client
        .create_network(options)
        .await
        .with_context(|| format!("failed to create site network {name}"))?;

    Ok((name, NetworkEnsureResult::Created))
}

/// Lightweight bridge network for proxy overlays when it differs from `docker.network`.
pub async fn ensure_named_bridge_network(
    client: &bollard::Docker,
    name: &str,
) -> Result<NetworkEnsureResult, anyhow::Error> {
    use bollard::network::CreateNetworkOptions;

    let mut filters = HashMap::new();
    filters.insert("name", vec![name]);

    let existing = client
        .list_networks(Some(ListNetworksOptions { filters }))
        .await
        .context("failed to list docker networks")?;

    if existing.iter().any(|n| n.name.as_deref() == Some(name)) {
        tracing::debug!(network = %name, "docker bridge network already exists");
        return Ok(NetworkEnsureResult::AlreadyExists);
    }

    let options = CreateNetworkOptions {
        name,
        check_duplicate: true,
        driver: "bridge",
        internal: false,
        enable_ipv6: false,
        ..Default::default()
    };

    client
        .create_network(options)
        .await
        .with_context(|| format!("failed to create docker network {name}"))?;

    tracing::info!(network = %name, "created docker bridge network");
    Ok(NetworkEnsureResult::Created)
}

pub fn site_network_name(site_id: &str) -> String {
    format!("site-nw-{site_id}")
}

pub async fn connect_container_network(
    client: &bollard::Docker,
    network: &str,
    container_id: &str,
) -> Result<(), anyhow::Error> {
    use bollard::network::ConnectNetworkOptions;

    client
        .connect_network(
            network,
            ConnectNetworkOptions {
                container: container_id.to_string(),
                ..Default::default()
            },
        )
        .await
        .with_context(|| format!("failed to connect {container_id} to network {network}"))?;
    Ok(())
}
