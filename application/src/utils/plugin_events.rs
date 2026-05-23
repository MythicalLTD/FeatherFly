use featherfly_plugin_sdk::PluginEvent;
use serde::Serialize;

use crate::plugins::PluginRegistry;

#[derive(Serialize)]
pub struct RequestPayload<'a> {
    pub client_ip: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub query: &'a str,
}

#[derive(Serialize)]
pub struct RequestCompletedPayload<'a> {
    pub client_ip: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub query: &'a str,
    pub status: u16,
    pub duration_ms: u64,
}

#[derive(Serialize)]
pub struct ProbeBlockedPayload<'a> {
    pub client_ip: &'a str,
    pub unknown_hits: u32,
    pub block_secs: u32,
}

#[derive(Serialize)]
pub struct RestartScheduledPayload {
    pub delay_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct PluginReloadPayload {
    pub plugin_count: usize,
    pub delay_ms: u64,
}

#[derive(Serialize)]
pub struct ConfigAppliedPayload {
    pub path: String,
    pub requires_restart: bool,
    pub restart_reasons: Vec<String>,
    pub restart_scheduled: bool,
}

#[derive(Serialize)]
pub struct ConfigExportedPayload {
    pub path: String,
}

#[derive(Serialize)]
pub struct ErrorPayload {
    pub error: String,
}

#[derive(Serialize)]
pub struct UpgradeStartedPayload {
    pub url: String,
}

pub fn emit_json(registry: &PluginRegistry, event: PluginEvent, payload: &impl Serialize) {
    if !registry.has_event_hooks(event) {
        return;
    }

    let Ok(bytes) = serde_json::to_vec(payload) else {
        tracing::warn!(
            event = event.name(),
            "failed to serialize plugin event payload"
        );
        return;
    };

    registry.emit(event, &bytes);
}

pub fn ip_string(ip: std::net::IpAddr) -> String {
    ip.to_string()
}
