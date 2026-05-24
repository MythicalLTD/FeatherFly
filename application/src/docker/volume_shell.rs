use anyhow::Context;
use tokio::process::Command;

use super::DockerManager;

pub fn sanitize_site_path(path: &str) -> Result<String, anyhow::Error> {
    let path = path.trim().replace('\\', "/");
    let path = path.trim_start_matches('/');
    if path.contains("..") {
        anyhow::bail!("path traversal not allowed");
    }
    Ok(path.to_string())
}

pub async fn volume_run(
    _docker: &DockerManager,
    volume: &str,
    script: &str,
) -> Result<String, anyhow::Error> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/data"),
            "alpine:3",
            "sh",
            "-c",
            script,
        ])
        .output()
        .await
        .context("failed to run volume shell command")?;
    if !output.status.success() {
        anyhow::bail!(
            "volume command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn volume_read_file(
    _docker: &DockerManager,
    volume: &str,
    rel_path: &str,
) -> Result<Vec<u8>, anyhow::Error> {
    let rel = sanitize_site_path(rel_path)?;
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/data:ro"),
            "alpine:3",
            "cat",
            &format!("/data/{rel}"),
        ])
        .output()
        .await
        .context("failed to read file from volume")?;
    if !output.status.success() {
        anyhow::bail!("read failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output.stdout)
}

pub async fn volume_write_file(
    docker: &DockerManager,
    volume: &str,
    rel_path: &str,
    bytes: &[u8],
) -> Result<(), anyhow::Error> {
    let rel = sanitize_site_path(rel_path)?;
    let dir = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    if !dir.is_empty() {
        volume_run(docker, volume, &format!("mkdir -p /data/{dir}")).await?;
    }
    let tmp = format!("/tmp/featherfly-upload-{}", uuid::Uuid::new_v4());
    std::fs::write(&tmp, bytes)?;
    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/data"),
            "-v",
            &format!("{}:/upload:ro", tmp),
            "alpine:3",
            "sh",
            "-c",
            &format!("cp /upload /data/{rel}"),
        ])
        .status()
        .await?;
    std::fs::remove_file(&tmp).ok();
    if !status.success() {
        anyhow::bail!("upload failed");
    }
    Ok(())
}
