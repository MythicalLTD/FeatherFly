use std::process::Stdio;

use anyhow::Context;
use tokio::process::Command;

use crate::docker::DockerManager;

use super::definition::{EggInstallScript, substitute_placeholders};

pub async fn run_install_script(
    _docker: &DockerManager,
    network: &str,
    volume: &str,
    site_id: &str,
    script: &EggInstallScript,
    env: &std::collections::HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    let tmp_dir = std::env::temp_dir().join(format!("featherfly-install-{site_id}"));
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .context("failed to create install script directory")?;

    let resolved_script = substitute_placeholders(&script.script, env);
    let install_path = tmp_dir.join("install.sh");
    tokio::fs::write(&install_path, resolved_script)
        .await
        .context("failed to write install script")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&install_path, std::fs::Permissions::from_mode(0o755))
            .await
            .ok();
    }

    let mut docker_env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
    for (k, v) in &script.environment {
        docker_env.push(format!("{k}={v}"));
    }

    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            network,
            "--name",
            &format!("{site_id}_installer"),
            "-v",
            &format!("{volume}:/mnt/server"),
            "-v",
            &format!("{}:/mnt/install:ro", tmp_dir.display()),
        ])
        .args(
            docker_env
                .iter()
                .flat_map(|pair| ["-e".to_string(), pair.clone()]),
        )
        .arg(&script.container_image)
        .arg(&script.entrypoint)
        .arg("/mnt/install/install.sh")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .await
        .context("failed to run egg install container")?;

    tokio::fs::remove_dir_all(&tmp_dir).await.ok();

    if status.success() {
        tracing::info!(site_id, "egg install script completed");
        Ok(())
    } else {
        anyhow::bail!("egg install script exited with status {status}");
    }
}
