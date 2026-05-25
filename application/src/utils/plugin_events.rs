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
pub struct PluginConfigMutatedPayload {
    pub hook_count: usize,
    pub input_len: usize,
    pub output_len: usize,
}

#[derive(Serialize)]
pub struct PluginRequestShortCircuitedPayload<'a> {
    pub phase: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
}

#[derive(Serialize)]
pub struct PluginJsonMutatedPayload<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub input_len: usize,
    pub output_len: usize,
}

#[derive(Serialize)]
pub struct PluginRouteInvokedPayload<'a> {
    pub plugin: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
}

#[derive(Serialize)]
pub struct PluginRouteFailedPayload<'a> {
    pub plugin: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub error: &'a str,
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

#[derive(Serialize)]
pub struct ConfigUpgradedPayload<'a> {
    pub path: &'a str,
}

#[derive(Serialize)]
pub struct HostingReconciledPayload {
    pub sites_total: usize,
    pub sites_started: usize,
    pub mail_synced: usize,
    pub ftp_synced: usize,
    pub shared_stack: bool,
}

#[derive(Serialize)]
pub struct SharedStackSyncedPayload {
    pub mail: bool,
    pub roundcube: bool,
    pub sftp: bool,
    pub ftp: bool,
    pub phpmyadmin: bool,
    pub database_servers: bool,
}

#[derive(Serialize)]
pub struct SharedDatabaseServersReadyPayload {
    pub mysql: bool,
    pub postgres: bool,
    pub redis: bool,
    pub mongodb: bool,
    pub site_networks_connected: usize,
}

#[derive(Serialize)]
pub struct PhpMyAdminProvisionedPayload<'a> {
    pub container_id: &'a str,
    pub site_count: usize,
}

#[derive(Serialize)]
pub struct ErrorPagesReadyPayload<'a> {
    pub pages_directory: &'a str,
    pub container_id: &'a str,
}

#[derive(Serialize)]
pub struct TraefikProvisionedPayload<'a> {
    pub result: &'a str,
    pub image: &'a str,
    pub network: &'a str,
    pub tls_enabled: bool,
}

#[derive(Serialize)]
pub struct HostingImagesPulledPayload {
    pub image_count: usize,
    pub images: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use featherfly_plugin_sdk::PluginEvent;

    #[test]
    fn payload_types_for_new_events_serialize() {
        let payloads: Vec<serde_json::Value> = vec![
            serde_json::to_value(ConfigUpgradedPayload {
                path: "/tmp/config.yml",
            })
            .unwrap(),
            serde_json::to_value(HostingReconciledPayload {
                sites_total: 1,
                sites_started: 1,
                mail_synced: 0,
                ftp_synced: 0,
                shared_stack: true,
            })
            .unwrap(),
            serde_json::to_value(SharedStackSyncedPayload {
                mail: true,
                roundcube: false,
                sftp: false,
                ftp: false,
                phpmyadmin: true,
                database_servers: true,
            })
            .unwrap(),
            serde_json::to_value(PluginConfigMutatedPayload {
                hook_count: 1,
                input_len: 10,
                output_len: 12,
            })
            .unwrap(),
            serde_json::to_value(PluginRequestShortCircuitedPayload {
                phase: "request.intercept",
                method: "GET",
                path: "/api/system",
                status: 403,
            })
            .unwrap(),
            serde_json::to_value(PluginJsonMutatedPayload {
                method: "GET",
                path: "/api/system",
                input_len: 10,
                output_len: 20,
            })
            .unwrap(),
        ];
        assert_eq!(payloads.len(), 6);
        assert_eq!(PluginEvent::ConfigUpgraded.name(), "config.upgraded");
        assert_eq!(
            PluginEvent::PluginRequestShortCircuited.name(),
            "plugins.request_short_circuited"
        );
    }
}
