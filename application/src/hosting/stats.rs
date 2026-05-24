use anyhow::Context;
use featherfly_plugin_sdk::PluginEvent;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    routes::State,
    utils::plugin_events::{self, SiteStatsCollectedPayload},
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SiteStats {
    pub site_id: String,
    pub container_count: usize,
    pub cpu_usage_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_limit_bytes: Option<u64>,
    pub disk_used_bytes: Option<u64>,
    pub disk_quota_mb: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
}

pub struct SiteStatsService;

impl SiteStatsService {
    pub async fn collect(state: &State, site_id: &str) -> Result<SiteStats, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let docker = state.docker.as_ref().context("docker unavailable")?;

        let mut container_count = 0usize;
        let mut mem_used = None;
        let mut mem_limit = None;

        if let Some(cid) = &site.container_id {
            container_count += 1;
            if let Ok(stats) = docker.container_stats_snapshot(cid).await {
                mem_used = stats.memory_stats.usage;
                mem_limit = stats.memory_stats.limit;
            }
        }

        container_count += cache.list_databases(site_id).await?.len();
        if cache.get_mail_meta(site_id).await?.is_some() {
            container_count += 1;
        }
        if cache.get_ftp_meta(site_id).await?.is_some() {
            container_count += 1;
        }

        let disk_used = if let Some(vol) = site.volume.as_deref() {
            volume_usage_bytes(docker.as_ref(), vol).await.ok()
        } else {
            None
        };

        plugin_events::emit_state_event(
            state,
            PluginEvent::SiteStatsCollected,
            &SiteStatsCollectedPayload { site_id },
        );

        Ok(SiteStats {
            site_id: site_id.to_string(),
            container_count,
            cpu_usage_percent: None,
            memory_used_bytes: mem_used,
            memory_limit_bytes: mem_limit,
            disk_used_bytes: disk_used,
            disk_quota_mb: site.disk_quota_mb,
            bandwidth_mbps: site.bandwidth_mbps,
        })
    }
}

async fn volume_usage_bytes(
    docker: &crate::docker::DockerManager,
    volume: &str,
) -> Result<u64, anyhow::Error> {
    let out = crate::docker::volume_run(
        docker,
        volume,
        "du -sb /data 2>/dev/null | awk '{print $1}'",
    )
    .await?;
    Ok(out.trim().parse().unwrap_or(0))
}
