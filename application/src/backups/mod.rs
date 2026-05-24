use std::path::{Path, PathBuf};

use anyhow::Context;
use tokio::process::Command;
use uuid::Uuid;

use crate::cache::{BackupRecord, Cache, SiteRecord};
use crate::config::InnerConfig;
use crate::docker::DockerManager;

pub struct BackupResult {
    pub id: String,
    pub path: PathBuf,
}

pub async fn create_backup(
    inner: &InnerConfig,
    docker: &DockerManager,
    cache: &Cache,
    site: &SiteRecord,
) -> Result<BackupResult, anyhow::Error> {
    let volume = site.volume.as_ref().context("site has no volume")?;

    if let Some(cid) = &site.container_id {
        docker.stop_container(cid).await.ok();
    }

    let backup_id = Uuid::new_v4().to_string();
    let site_backup_dir = Path::new(&inner.system.backup_directory).join(&site.id);
    std::fs::create_dir_all(&site_backup_dir)?;
    let backup_path = site_backup_dir.join(format!("{backup_id}.tar.gz"));

    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/data:ro"),
            "-v",
            &format!("{}:/backup", site_backup_dir.display()),
            "alpine:3",
            "tar",
            "-czf",
            &format!("/backup/{backup_id}.tar.gz"),
            "-C",
            "/data",
            ".",
        ])
        .status()
        .await
        .context("failed to create backup archive")?;
    if !status.success() {
        anyhow::bail!("backup tar failed");
    }

    if let Some(cid) = &site.container_id {
        docker.start_container(cid).await.ok();
    }

    let record = BackupRecord {
        id: backup_id.clone(),
        site_id: site.id.clone(),
        path: backup_path.display().to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    cache.save_backup(&record).await?;

    Ok(BackupResult {
        id: backup_id,
        path: backup_path,
    })
}

pub async fn restore_backup(
    docker: &DockerManager,
    site: &SiteRecord,
    backup_path: &Path,
) -> Result<(), anyhow::Error> {
    let volume = site.volume.as_ref().context("site has no volume")?;

    if let Some(cid) = &site.container_id {
        docker.stop_container(cid).await.ok();
    }

    let parent = backup_path.parent().context("backup path has no parent")?;
    let file_name = backup_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("invalid backup file name")?;

    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/data"),
            "-v",
            &format!("{}:/backup:ro", parent.display()),
            "alpine:3",
            "sh",
            "-c",
            &format!("rm -rf /data/* /data/.[!.]* /data/..?* 2>/dev/null; tar -xzf /backup/{file_name} -C /data"),
        ])
        .status()
        .await
        .context("failed to restore backup")?;
    if !status.success() {
        anyhow::bail!("backup restore failed");
    }

    if let Some(cid) = &site.container_id {
        docker.start_container(cid).await?;
    }

    Ok(())
}
