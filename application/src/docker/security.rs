use std::collections::HashMap;

use bollard::models::{HostConfig, PortBinding};

/// Build a hardened default `HostConfig` for site containers.
pub fn secure_host_config(
    network_mode: &str,
    container_pid_limit: i64,
    memory_mb: Option<u64>,
    cpu_quota: Option<i64>,
    port_bindings: HashMap<String, Option<Vec<PortBinding>>>,
) -> HostConfig {
    let memory = memory_mb.map(|mb| mb.saturating_mul(1024 * 1024) as i64);
    let memory_swap = memory;

    HostConfig {
        network_mode: Some(network_mode.to_string()),
        port_bindings: Some(port_bindings),
        memory,
        memory_swap,
        cpu_quota,
        pids_limit: Some(container_pid_limit),
        privileged: Some(false),
        publish_all_ports: Some(false),
        auto_remove: Some(false),
        ..Default::default()
    }
}
