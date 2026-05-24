use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use tokio::process::Command;
use tokio::time::sleep;

const INSTALL_SCRIPT_URL: &str = "https://get.docker.com/";

pub async fn ensure_docker_engine(socket: &str, auto_install: bool) -> Result<bool, anyhow::Error> {
    if docker_reachable(socket).await {
        return Ok(false);
    }

    if !auto_install {
        tracing::debug!("docker auto_install disabled — skipping engine setup");
        return Ok(false);
    }

    let binary_present = docker_binary_present().await?;

    if !binary_present {
        tracing::info!("docker not found — installing via get.docker.com (CHANNEL=stable)");
        install_docker_engine().await?;
    } else {
        tracing::info!("docker binary present but engine unreachable — starting docker service");
    }

    enable_docker_service().await?;
    wait_for_docker_socket(socket).await?;

    tracing::info!("docker engine is up after auto-install/bootstrap");
    Ok(true)
}

async fn docker_reachable(socket: &str) -> bool {
    if !Path::new(socket).exists() {
        return false;
    }
    Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn docker_binary_present() -> Result<bool, anyhow::Error> {
    let output = Command::new("sh")
        .args(["-c", "command -v docker >/dev/null 2>&1"])
        .status()
        .await
        .context("failed to check for docker binary")?;
    Ok(output.success())
}

async fn install_docker_engine() -> Result<(), anyhow::Error> {
    let script = format!("curl -fsSL {INSTALL_SCRIPT_URL} | CHANNEL=stable bash");
    run_privileged_shell(&script)
        .await
        .context("docker install script failed")?;
    Ok(())
}

async fn enable_docker_service() -> Result<(), anyhow::Error> {
    run_privileged("systemctl", &["enable", "--now", "docker"])
        .await
        .context("failed to enable docker systemd unit")?;
    Ok(())
}

async fn wait_for_docker_socket(socket: &str) -> Result<(), anyhow::Error> {
    for attempt in 1..=30 {
        if docker_reachable(socket).await {
            return Ok(());
        }
        tracing::debug!(
            attempt,
            socket = %socket,
            "waiting for docker socket after install"
        );
        sleep(Duration::from_secs(2)).await;
    }
    anyhow::bail!("docker socket {socket} did not become ready after install")
}

async fn run_privileged_shell(script: &str) -> Result<(), anyhow::Error> {
    if is_root().await? {
        let status = Command::new("bash")
            .arg("-c")
            .arg(script)
            .status()
            .await
            .context("failed to run shell command")?;
        if !status.success() {
            anyhow::bail!("command failed with status {status}");
        }
        return Ok(());
    }

    let status = Command::new("sudo")
        .args(["-n", "bash", "-c", script])
        .status()
        .await
        .context("failed to run privileged shell command via sudo")?;
    if !status.success() {
        anyhow::bail!("privileged command failed (is passwordless sudo configured?): {status}");
    }
    Ok(())
}

async fn run_privileged(program: &str, args: &[&str]) -> Result<(), anyhow::Error> {
    if is_root().await? {
        let status = Command::new(program)
            .args(args)
            .status()
            .await
            .with_context(|| format!("failed to run {program}"))?;
        if !status.success() {
            anyhow::bail!("{program} failed with status {status}");
        }
        return Ok(());
    }

    let mut sudo_args = vec!["-n", program];
    sudo_args.extend(args);
    let status = Command::new("sudo")
        .args(sudo_args)
        .status()
        .await
        .context("failed to run command via sudo")?;
    if !status.success() {
        anyhow::bail!("sudo {program} failed with status {status}");
    }
    Ok(())
}

async fn is_root() -> Result<bool, anyhow::Error> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .await
        .context("failed to check current user id")?;
    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "0")
}
