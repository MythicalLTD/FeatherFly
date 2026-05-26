use anyhow::Context;
use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    docker::{
        sanitize_site_path, volume_file_size, volume_read_file, volume_run, volume_usage_bytes,
        volume_write_file,
    },
    routes::State,
    utils::plugin_events::{self, FileManagerPayload},
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MoveCopyRequest {
    pub from: String,
    pub to: String,
}

pub struct FileService;

impl FileService {
    pub async fn list(
        state: &State,
        site_id: &str,
        path: &str,
    ) -> Result<Vec<FileEntry>, anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let rel = sanitize_site_path(path)?;
        let dir = if rel.is_empty() {
            ".".into()
        } else {
            rel.clone()
        };
        let out = volume_run(
            docker,
            &volume,
            &format!(
                "cd /data/{dir} 2>/dev/null || exit 1; ls -la | tail -n +2 | awk '{{print $1,$5,$9}}'"
            ),
        )
        .await?;

        let entries = out
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let perm = parts.next()?;
                let size: u64 = parts.next()?.parse().ok()?;
                let name = parts.next()?.to_string();
                if name == "." || name == ".." {
                    return None;
                }
                let is_dir = perm.starts_with('d');
                let entry_path = if rel.is_empty() {
                    name.clone()
                } else {
                    format!("{rel}/{name}")
                };
                Some(FileEntry {
                    name,
                    path: entry_path,
                    is_dir,
                    size,
                })
            })
            .collect();

        emit(state, site_id, "list", path);
        Ok(entries)
    }

    pub async fn download(
        state: &State,
        site_id: &str,
        path: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let bytes = volume_read_file(docker, &volume, path).await?;
        emit(state, site_id, "download", path);
        Ok(bytes)
    }

    pub async fn upload(
        state: &State,
        site_id: &str,
        path: &str,
        bytes: &[u8],
    ) -> Result<(), anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let volume = site.volume.context("site has no volume")?;

        enforce_disk_quota(
            docker.as_ref(),
            &volume,
            site.disk_quota_mb,
            path,
            bytes.len() as u64,
        )
        .await?;

        volume_write_file(docker.as_ref(), &volume, path, bytes).await?;
        emit(state, site_id, "upload", path);
        Ok(())
    }

    pub async fn delete(state: &State, site_id: &str, path: &str) -> Result<(), anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let rel = sanitize_site_path(path)?;
        volume_run(docker, &volume, &format!("rm -rf /data/{rel}")).await?;
        emit(state, site_id, "delete", path);
        Ok(())
    }

    pub async fn rename(
        state: &State,
        site_id: &str,
        req: RenameRequest,
    ) -> Result<(), anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let from = sanitize_site_path(&req.from)?;
        let to = sanitize_site_path(&req.to)?;
        volume_run(docker, &volume, &format!("mv /data/{from} /data/{to}")).await?;
        emit(state, site_id, "rename", &req.from);
        Ok(())
    }

    pub async fn r#move(
        state: &State,
        site_id: &str,
        req: MoveCopyRequest,
    ) -> Result<(), anyhow::Error> {
        Self::rename(
            state,
            site_id,
            RenameRequest {
                from: req.from.clone(),
                to: req.to.clone(),
            },
        )
        .await?;
        emit(state, site_id, "move", &req.from);
        Ok(())
    }

    pub async fn copy(
        state: &State,
        site_id: &str,
        req: MoveCopyRequest,
    ) -> Result<(), anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let from = sanitize_site_path(&req.from)?;
        let to = sanitize_site_path(&req.to)?;
        volume_run(docker, &volume, &format!("cp -a /data/{from} /data/{to}")).await?;
        emit(state, site_id, "copy", &req.from);
        Ok(())
    }

    pub async fn mkdir(state: &State, site_id: &str, path: &str) -> Result<(), anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let rel = sanitize_site_path(path)?;
        volume_run(docker, &volume, &format!("mkdir -p /data/{rel}")).await?;
        emit(state, site_id, "mkdir", path);
        Ok(())
    }

    pub async fn archive(
        state: &State,
        site_id: &str,
        path: &str,
    ) -> Result<String, anyhow::Error> {
        let (docker, volume) = site_volume(state, site_id).await?;
        let rel = sanitize_site_path(path)?;
        let archive_name = format!("{rel}.tar.gz").replace('/', "_");
        volume_run(
            docker,
            &volume,
            &format!("tar -czf /data/{archive_name} -C /data {rel}"),
        )
        .await?;
        emit(state, site_id, "archive", path);
        Ok(archive_name)
    }
}

async fn site_volume<'a>(
    state: &'a State,
    site_id: &str,
) -> Result<(&'a crate::docker::DockerManager, String), anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let cache = state.cache.as_ref().context("cache unavailable")?;
    let site = cache.get_site(site_id).await?.context("site not found")?;
    let volume = site.volume.context("site has no volume")?;
    Ok((docker.as_ref(), volume))
}

async fn enforce_disk_quota(
    docker: &crate::docker::DockerManager,
    volume: &str,
    disk_quota_mb: Option<i64>,
    path: &str,
    incoming_bytes: u64,
) -> Result<(), anyhow::Error> {
    let Some(quota_bytes) = disk_quota_bytes(disk_quota_mb) else {
        return Ok(());
    };

    let current_usage = volume_usage_bytes(docker, volume).await?;
    let existing_size = volume_file_size(docker, volume, path).await?.unwrap_or(0);
    let projected_usage =
        projected_disk_usage_after_write(current_usage, existing_size, incoming_bytes);

    if projected_usage > quota_bytes {
        anyhow::bail!(
            "disk quota exceeded: projected usage {projected_usage} bytes exceeds limit {quota_bytes} bytes"
        );
    }

    Ok(())
}

pub fn disk_quota_bytes(disk_quota_mb: Option<i64>) -> Option<u64> {
    disk_quota_mb
        .and_then(|mb| u64::try_from(mb).ok())
        .filter(|mb| *mb > 0)
        .map(|mb| mb.saturating_mul(1024 * 1024))
}

pub fn projected_disk_usage_after_write(
    current_usage: u64,
    existing_size: u64,
    incoming_bytes: u64,
) -> u64 {
    current_usage
        .saturating_sub(existing_size)
        .saturating_add(incoming_bytes)
}

fn emit(state: &State, site_id: &str, action: &str, path: &str) {
    plugin_events::emit_state_event(
        state,
        PluginEvent::FileManagerChanged,
        &FileManagerPayload {
            site_id,
            action,
            path,
        },
    );
}
