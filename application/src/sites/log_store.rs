use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogArchiveEntry {
    pub name: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

pub fn site_logs_dir(data_root: &str, site_id: &str) -> PathBuf {
    Path::new(data_root)
        .join("sites")
        .join(site_id)
        .join("logs")
}

pub fn ensure_logs_dir(data_root: &str, site_id: &str) -> Result<PathBuf, anyhow::Error> {
    let dir = site_logs_dir(data_root, site_id);
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    Ok(dir)
}

pub fn archive_log(
    data_root: &str,
    site_id: &str,
    content: &str,
) -> Result<LogArchiveEntry, anyhow::Error> {
    let dir = ensure_logs_dir(data_root, site_id)?;
    let ts = Utc::now().timestamp();
    let name = format!("{ts}-{}.log", uuid::Uuid::new_v4().simple());
    let path = dir.join(&name);
    std::fs::write(&path, content.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    let meta = std::fs::metadata(&path)?;
    Ok(LogArchiveEntry {
        name,
        size_bytes: meta.len(),
        created_at: ts,
    })
}

pub fn list_archives(
    data_root: &str,
    site_id: &str,
) -> Result<Vec<LogArchiveEntry>, anyhow::Error> {
    let dir = site_logs_dir(data_root, site_id);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in
        std::fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let meta = entry.metadata()?;
        let created_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        entries.push(LogArchiveEntry {
            name,
            size_bytes: meta.len(),
            created_at,
        });
    }
    entries.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    Ok(entries)
}

pub fn read_archive(data_root: &str, site_id: &str, name: &str) -> Result<Vec<u8>, anyhow::Error> {
    if name.contains('/') || name.contains('\\') || !name.ends_with(".log") {
        anyhow::bail!("invalid log archive name");
    }
    let path = site_logs_dir(data_root, site_id).join(name);
    std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))
}

pub fn prune_archives(data_root: &str, site_id: &str, keep: u32) -> Result<usize, anyhow::Error> {
    let mut archives = list_archives(data_root, site_id)?;
    if archives.len() <= keep as usize {
        return Ok(0);
    }
    archives.sort_by_key(|a| a.created_at);
    let remove_count = archives.len() - keep as usize;
    let dir = site_logs_dir(data_root, site_id);
    let mut removed = 0;
    for entry in archives.into_iter().take(remove_count) {
        std::fs::remove_file(dir.join(&entry.name)).ok();
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_roundtrip_and_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_str().unwrap();
        let entry = archive_log(root, "site-a", "line1\nline2").unwrap();
        assert!(entry.name.ends_with(".log"));
        let listed = list_archives(root, "site-a").unwrap();
        assert_eq!(listed.len(), 1);
        let bytes = read_archive(root, "site-a", &entry.name).unwrap();
        assert_eq!(bytes, b"line1\nline2");
        archive_log(root, "site-a", "second").unwrap();
        let removed = prune_archives(root, "site-a", 1).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(list_archives(root, "site-a").unwrap().len(), 1);
    }
}
