use featherfly_plugin_sdk::PluginEvent;
use serde::Serialize;

use crate::plugins::PluginRegistry;

#[derive(Serialize)]
pub struct RequestPayload<'a> {
    #[serde(skip_serializing_if = "str::is_empty")]
    pub request_id: &'a str,
    pub client_ip: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub query: &'a str,
}

#[derive(Serialize)]
pub struct RequestCompletedPayload<'a> {
    #[serde(skip_serializing_if = "str::is_empty")]
    pub request_id: &'a str,
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
    #[serde(skip_serializing_if = "str::is_empty")]
    pub request_id: &'a str,
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
pub struct NodeIdentityPayload<'a> {
    pub uuid: &'a str,
    pub token_id: &'a str,
}

#[derive(Serialize)]
pub struct CloudPanelCommandPayload<'a> {
    pub operation: &'a str,
    pub command: &'a str,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
    pub hook_handlers: usize,
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
}

pub fn ip_string(ip: std::net::IpAddr) -> String {
    ip.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_runtime_payloads_serialize() {
        let values = [
            serde_json::to_value(PluginConfigMutatedPayload {
                hook_count: 1,
                input_len: 10,
                output_len: 20,
            })
            .unwrap(),
            serde_json::to_value(PluginRouteInvokedPayload {
                plugin: "hello",
                method: "GET",
                path: "/plugins/hello",
                status: 200,
            })
            .unwrap(),
            serde_json::to_value(NodeIdentityPayload {
                uuid: "uuid",
                token_id: "token",
            })
            .unwrap(),
            serde_json::to_value(CloudPanelCommandPayload {
                operation: "list_users",
                command: "user:list",
                args: Vec::new(),
                site_type: None,
                status: Some(0),
                duration_ms: 1,
                error: None,
                hook_handlers: 0,
            })
            .unwrap(),
        ];

        assert_eq!(values.len(), 4);
    }
}
