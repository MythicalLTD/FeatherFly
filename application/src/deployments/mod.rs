use std::path::Path;
use std::process::Stdio;

use anyhow::Context;
use tokio::process::Command;

use crate::cache::SiteRecord;
use crate::config::InnerConfig;

pub mod blue_green;

pub async fn run_deploy(
    inner: &InnerConfig,
    site: &SiteRecord,
    git_ref: Option<&str>,
    rebuild: bool,
    restart: bool,
) -> Result<(), anyhow::Error> {
    let repo = git_ref
        .map(str::to_string)
        .or_else(|| site.git_repo.clone())
        .context("no git repository configured for site")?;

    let deploy_dir = Path::new(&inner.system.data)
        .join("sites")
        .join(&site.id)
        .join("src");
    std::fs::create_dir_all(&deploy_dir)?;

    if deploy_dir.join(".git").exists() {
        run_git(&deploy_dir, &["pull", "--ff-only"]).await?;
    } else {
        if deploy_dir.exists() {
            std::fs::remove_dir_all(&deploy_dir).ok();
            std::fs::create_dir_all(&deploy_dir)?;
        }
        run_git(
            Path::new("."),
            &["clone", repo.as_str(), deploy_dir.to_str().unwrap()],
        )
        .await?;
    }

    let volume = site.volume.as_ref().context("site has no volume")?;

    sync_to_volume(volume, &deploy_dir).await?;

    if rebuild {
        tracing::info!(site = %site.id, "rebuild requested — restart site container to apply");
    }

    if restart && let Some(cid) = &site.container_id {
        restart_container(cid).await?;
    }

    Ok(())
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<(), anyhow::Error> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .await
        .context("failed to spawn git")?;
    if !status.success() {
        anyhow::bail!("git command failed: git {}", args.join(" "));
    }
    Ok(())
}

async fn sync_to_volume(volume: &str, src: &Path) -> Result<(), anyhow::Error> {
    let src = src
        .canonicalize()
        .with_context(|| format!("invalid src path {}", src.display()))?;
    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/target"),
            "-v",
            &format!("{}:/src:ro", src.display()),
            "alpine:3",
            "sh",
            "-c",
            "cp -a /src/. /target/",
        ])
        .status()
        .await
        .context("failed to sync files into volume")?;
    if !status.success() {
        anyhow::bail!("volume sync failed");
    }
    Ok(())
}

async fn restart_container(container_id: &str) -> Result<(), anyhow::Error> {
    let status = Command::new("docker")
        .args(["restart", container_id])
        .status()
        .await
        .context("failed to restart container")?;
    if !status.success() {
        anyhow::bail!("docker restart failed for {container_id}");
    }
    Ok(())
}
