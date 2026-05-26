use std::time::Duration;

use anyhow::Context;
use tokio::process::Command;
use tokio::time::sleep;

use bollard::models::ContainerStateStatusEnum;

use crate::docker::DockerManager;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub async fn wait_for_site_ready(
    docker: &DockerManager,
    container_id: &str,
    network: &str,
    port: u16,
    health_path: Option<&str>,
) -> Result<(), anyhow::Error> {
    wait_for_site_ready_with_timeout(
        docker,
        container_id,
        network,
        port,
        health_path,
        DEFAULT_TIMEOUT,
    )
    .await
}

pub async fn wait_for_site_ready_with_timeout(
    docker: &DockerManager,
    container_id: &str,
    network: &str,
    port: u16,
    health_path: Option<&str>,
    timeout: Duration,
) -> Result<(), anyhow::Error> {
    let path = health_path.filter(|p| !p.is_empty()).unwrap_or("/");
    let deadline = tokio::time::Instant::now() + timeout;
    let mut last_err = String::from("container not ready");

    while tokio::time::Instant::now() < deadline {
        if !container_is_running(docker, container_id).await? {
            last_err = "container is not running".into();
            sleep(POLL_INTERVAL).await;
            continue;
        }

        match container_ip_on_network(docker, container_id, network).await {
            Ok(ip) => {
                if probe_http(network, &ip, port, path).await.is_ok() {
                    tracing::info!(container_id, %ip, port, "site container passed health check");
                    return Ok(());
                }
                last_err = format!("HTTP probe failed for {ip}:{port}{path}");
            }
            Err(err) => last_err = err.to_string(),
        }

        sleep(POLL_INTERVAL).await;
    }

    anyhow::bail!("site health check timed out: {last_err}")
}

async fn container_is_running(
    docker: &DockerManager,
    container_id: &str,
) -> Result<bool, anyhow::Error> {
    let inspect = docker.inspect_container(container_id).await?;
    Ok(inspect
        .state
        .and_then(|s| s.status)
        .is_some_and(|status| status == ContainerStateStatusEnum::RUNNING))
}

async fn container_ip_on_network(
    docker: &DockerManager,
    container_id: &str,
    network: &str,
) -> Result<String, anyhow::Error> {
    let inspect = docker.inspect_container(container_id).await?;
    let ip = inspect
        .network_settings
        .and_then(|settings| settings.networks)
        .and_then(|networks| networks.get(network).cloned())
        .and_then(|network| network.ip_address)
        .filter(|ip| !ip.is_empty())
        .context("container has no IP on site network")?;
    Ok(ip)
}

async fn probe_http(network: &str, ip: &str, port: u16, path: &str) -> Result<(), anyhow::Error> {
    let url = format!("http://{ip}:{port}{path}");
    let script = format!("wget -q -O /dev/null --timeout=5 '{url}'");
    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            network,
            "alpine:3",
            "sh",
            "-c",
            &script,
        ])
        .status()
        .await
        .context("failed to run health probe container")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("wget probe failed for {url}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timeout_is_two_minutes() {
        assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(120));
    }
}
