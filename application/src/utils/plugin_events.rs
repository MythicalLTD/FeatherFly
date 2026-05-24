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

#[derive(Serialize)]
pub struct NodeIdentityPayload<'a> {
    pub uuid: &'a str,
    pub token_id: &'a str,
}

#[derive(Serialize)]
pub struct DockerReadyPayload<'a> {
    pub socket: &'a str,
    pub network: &'a str,
    pub network_created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

#[derive(Serialize)]
pub struct DockerUnavailablePayload<'a> {
    pub socket: &'a str,
    pub error: String,
}

#[derive(Serialize)]
pub struct ContainerCreatedPayload<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub image: &'a str,
    pub started: bool,
}

#[derive(Serialize)]
pub struct ContainerIdPayload<'a> {
    pub id: &'a str,
}

#[derive(Serialize)]
pub struct ContainerRemovedPayload<'a> {
    pub id: &'a str,
    pub force: bool,
}

#[derive(Serialize)]
pub struct ContainerOperationFailedPayload<'a> {
    pub id: &'a str,
    pub operation: &'a str,
    pub error: String,
}

#[derive(Serialize)]
pub struct ImagePulledPayload<'a> {
    pub image: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<&'a str>,
}

#[derive(Serialize)]
pub struct ContainerLogsPayload<'a> {
    pub id: &'a str,
    pub follow: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<String>,
}

#[derive(Serialize)]
pub struct ContainerExecStartedPayload<'a> {
    pub container_id: &'a str,
    pub exec_id: &'a str,
    pub tty: bool,
}

#[derive(Serialize)]
pub struct ContainerExecEndedPayload<'a> {
    pub container_id: &'a str,
    pub exec_id: &'a str,
}

#[derive(Serialize)]
pub struct PanelConnectedPayload<'a> {
    pub remote: &'a str,
    pub uuid: &'a str,
}

#[derive(Serialize)]
pub struct PanelDisconnectedPayload<'a> {
    pub remote: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct PanelMessagePayload<'a> {
    #[serde(rename = "type")]
    pub message_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<&'a str>,
}

#[derive(Serialize)]
pub struct StatsCollectedPayload<'a> {
    pub scope: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<&'a str>,
}

#[derive(Serialize)]
pub struct SiteIdPayload<'a> {
    pub id: &'a str,
}

#[derive(Serialize)]
pub struct SiteProvisionedPayload<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub domain: &'a str,
    pub template: &'a str,
    pub container_id: &'a str,
}

#[derive(Serialize)]
pub struct SiteDestroyedPayload<'a> {
    pub id: &'a str,
}

#[derive(Serialize)]
pub struct ProxyRegisteredPayload<'a> {
    pub site_id: &'a str,
    pub domains: &'a [String],
    pub provider: &'a str,
}

#[derive(Serialize)]
pub struct SslProvisionedPayload<'a> {
    pub site_id: &'a str,
    pub domains: &'a [String],
    pub cert_resolver: &'a str,
}

#[derive(Serialize)]
pub struct VolumeCreatedPayload<'a> {
    pub name: &'a str,
}

#[derive(Serialize)]
pub struct VolumeRemovedPayload<'a> {
    pub name: &'a str,
}

#[derive(Serialize)]
pub struct DeployStartedPayload<'a> {
    pub site_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<&'a str>,
}

#[derive(Serialize)]
pub struct DeployFinishedPayload<'a> {
    pub site_id: &'a str,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct BackupStartedPayload<'a> {
    pub site_id: &'a str,
}

#[derive(Serialize)]
pub struct BackupCompletedPayload<'a> {
    pub site_id: &'a str,
    pub backup_id: &'a str,
}

#[derive(Serialize)]
pub struct WebhookReceivedPayload<'a> {
    pub site_id: &'a str,
    pub source: &'a str,
}

#[derive(Serialize)]
pub struct DatabaseCreatedPayload<'a> {
    pub site_id: &'a str,
    pub database_id: &'a str,
    pub engine: &'a str,
    pub host: &'a str,
    pub port: i64,
}

#[derive(Serialize)]
pub struct DatabaseRemovedPayload<'a> {
    pub site_id: &'a str,
    pub database_id: &'a str,
    pub engine: &'a str,
}

#[derive(Serialize)]
pub struct TaskScheduledPayload<'a> {
    pub site_id: &'a str,
    pub task_id: &'a str,
    pub kind: &'a str,
    pub action: &'a str,
}

#[derive(Serialize)]
pub struct TaskExecutedPayload<'a> {
    pub site_id: &'a str,
    pub task_id: &'a str,
    pub action: &'a str,
}

#[derive(Serialize)]
pub struct TaskFailedPayload<'a> {
    pub site_id: &'a str,
    pub task_id: &'a str,
    pub error: String,
}

#[derive(Serialize)]
pub struct DnsSyncedPayload<'a> {
    pub site_id: &'a str,
    pub record_count: usize,
}

#[derive(Serialize)]
pub struct DeployBlueGreenPayload<'a> {
    pub site_id: &'a str,
}

#[derive(Serialize)]
pub struct MailAccountCreatedPayload<'a> {
    pub site_id: &'a str,
    pub account_id: &'a str,
    pub address: &'a str,
}

#[derive(Serialize)]
pub struct MailAccountRemovedPayload<'a> {
    pub site_id: &'a str,
    pub account_id: &'a str,
    pub address: &'a str,
}

#[derive(Serialize)]
pub struct MailSyncedPayload<'a> {
    pub site_id: &'a str,
    pub account_count: usize,
    pub mail_host: &'a str,
}

#[derive(Serialize)]
pub struct RoundcubeProvisionedPayload<'a> {
    pub site_id: &'a str,
    pub webmail_host: &'a str,
}

#[derive(Serialize)]
pub struct FtpAccountCreatedPayload<'a> {
    pub site_id: &'a str,
    pub account_id: &'a str,
    pub username: &'a str,
    pub protocol: &'a str,
}

#[derive(Serialize)]
pub struct FtpAccountRemovedPayload<'a> {
    pub site_id: &'a str,
    pub account_id: &'a str,
    pub username: &'a str,
}

#[derive(Serialize)]
pub struct FtpGatewaySyncedPayload<'a> {
    pub site_id: &'a str,
    pub account_count: usize,
}

#[derive(Serialize)]
pub struct SiteExecCompletedPayload<'a> {
    pub site_id: &'a str,
    pub container_id: &'a str,
    pub exit_output_len: usize,
}

#[derive(Serialize)]
pub struct FileManagerPayload<'a> {
    pub site_id: &'a str,
    pub action: &'a str,
    pub path: &'a str,
}

#[derive(Serialize)]
pub struct SiteStatsCollectedPayload<'a> {
    pub site_id: &'a str,
}

#[derive(Serialize)]
pub struct DnsDnssecUpdatedPayload<'a> {
    pub site_id: &'a str,
    pub enabled: bool,
    pub ds_count: usize,
}

#[derive(Serialize)]
pub struct HostingLimitsUpdatedPayload<'a> {
    pub site_id: &'a str,
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

pub fn emit_state_event(
    state: &crate::routes::AppState,
    event: PluginEvent,
    payload: &impl Serialize,
) {
    emit_json(&state.plugins, event, payload);
    if let Some(panel) = state.panel.load_full().as_ref() {
        panel.fanout_lifecycle(event, payload);
    }
}

pub fn emit_container_failure(
    state: &crate::routes::AppState,
    id: &str,
    operation: &str,
    error: impl std::fmt::Display,
) {
    emit_state_event(
        state,
        PluginEvent::ContainerOperationFailed,
        &ContainerOperationFailedPayload {
            id,
            operation,
            error: error.to_string(),
        },
    );
}

pub fn ip_string(ip: std::net::IpAddr) -> String {
    ip.to_string()
}
