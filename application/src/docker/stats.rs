use anyhow::Context;
use bollard::container::Stats;
use bollard::container::StatsOptions;
use futures_util::StreamExt;
use serde::Serialize;
use utoipa::ToSchema;

use super::DockerManager;

impl DockerManager {
    pub async fn container_stats_snapshot(&self, id: &str) -> Result<Stats, anyhow::Error> {
        let options = Some(StatsOptions {
            stream: false,
            one_shot: true,
        });

        let mut stream = self.client().stats(id, options);
        let stats = stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("no stats returned for container {id}"))?
            .with_context(|| format!("failed to read stats for container {id}"))?;

        Ok(stats)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HostStatsResponse {
    pub cpu_usage_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub container_count: usize,
}

pub fn collect_host_stats(container_count: usize) -> HostStatsResponse {
    use sysinfo::{Disks, System};

    let mut system = System::new_all();
    system.refresh_all();

    let memory_total = system.total_memory();
    let memory_used = system.used_memory();
    let cpu_usage = system.global_cpu_usage();

    let disks = Disks::new_with_refreshed_list();
    let (disk_total, disk_used) = disks
        .list()
        .iter()
        .fold((0u64, 0u64), |(total, used), disk| {
            (
                total.saturating_add(disk.total_space()),
                used.saturating_add(disk.total_space().saturating_sub(disk.available_space())),
            )
        });

    HostStatsResponse {
        cpu_usage_percent: cpu_usage,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        container_count,
    }
}
