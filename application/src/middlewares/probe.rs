use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use featherfly_plugin_sdk::PluginEvent;
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::{
    config::ProbeProtectionConfig,
    routes::State as AppStateArc,
    utils::{
        plugin_events::{self, ProbeBlockedPayload, RequestCompletedPayload, RequestPayload},
        response::ApiResponse,
    },
};

pub struct ProbeGuard {
    clients: Mutex<HashMap<IpAddr, ClientRecord>>,
}

struct ClientRecord {
    window_start: Instant,
    unknown_hits: u32,
    blocked_until: Option<Instant>,
}

pub struct BlockEvent {
    pub unknown_hits: u32,
}

impl ProbeGuard {
    #[must_use]
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_blocked(&self, ip: IpAddr, config: &ProbeProtectionConfig) -> bool {
        if !config.enabled {
            return false;
        }

        let Ok(mut clients) = self.clients.lock() else {
            return false;
        };

        Self::prune(&mut clients);
        Self::blocked_now(clients.get(&ip), Instant::now())
    }

    pub fn record_unknown_route(
        &self,
        ip: IpAddr,
        config: &ProbeProtectionConfig,
    ) -> Option<BlockEvent> {
        if !config.enabled {
            return None;
        }

        let Ok(mut clients) = self.clients.lock() else {
            return None;
        };

        let now = Instant::now();
        Self::prune(&mut clients);

        let record = clients.entry(ip).or_insert_with(|| ClientRecord {
            window_start: now,
            unknown_hits: 0,
            blocked_until: None,
        });

        if Self::blocked_now(Some(record), now) {
            return None;
        }

        let window = Duration::from_secs(u64::from(config.window_secs));
        if now.duration_since(record.window_start) > window {
            record.window_start = now;
            record.unknown_hits = 0;
        }

        record.unknown_hits = record.unknown_hits.saturating_add(1);

        if record.unknown_hits >= config.max_404s {
            record.blocked_until = Some(now + Duration::from_secs(u64::from(config.block_secs)));
            return Some(BlockEvent {
                unknown_hits: record.unknown_hits,
            });
        }

        None
    }

    fn blocked_now(record: Option<&ClientRecord>, now: Instant) -> bool {
        record.is_some_and(|record| record.blocked_until.is_some_and(|until| now < until))
    }

    fn prune(clients: &mut HashMap<IpAddr, ClientRecord>) {
        let now = Instant::now();
        clients.retain(|_, record| {
            if Self::blocked_now(Some(record), now) {
                return true;
            }

            record.blocked_until.is_some()
                || now.duration_since(record.window_start) < Duration::from_secs(3600)
        });
    }
}

impl Default for ProbeGuard {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn middleware(
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    State(state): State<AppStateArc>,
    req: Request,
    next: Next,
) -> Response {
    let config = state.config.load().security.probe_protection.clone();
    let ip = peer.ip();
    let client_ip = ip.to_string();
    let request_id = crate::middlewares::request_id::current_request_id(&req).to_string();
    let started = Instant::now();

    if config.enabled && state.probe_guard.is_blocked(ip, &config) {
        if config.log_blocked {
            tracing::warn!(
                request_id = %request_id,
                client_ip = %client_ip,
                "rejected request from blocked probe client"
            );
        }
        plugin_events::emit_state_event(
            &state,
            PluginEvent::RequestBlocked,
            &RequestPayload {
                request_id: &request_id,
                client_ip: &client_ip,
                method: req.method().as_str(),
                path: req.uri().path(),
                query: req.uri().query().unwrap_or(""),
            },
        );
        return ApiResponse::error("too many unknown route requests")
            .with_status(StatusCode::TOO_MANY_REQUESTS)
            .into_response();
    }

    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let uri = if query.is_empty() {
        path.clone()
    } else {
        format!("{path}?{query}")
    };

    plugin_events::emit_state_event(
        &state,
        PluginEvent::RequestReceived,
        &RequestPayload {
            request_id: &request_id,
            client_ip: &client_ip,
            method: &method,
            path: &path,
            query: &query,
        },
    );

    let response = next.run(req).await;
    let status = response.status();
    let duration_ms = started.elapsed().as_millis() as u64;

    if config.enabled && status == StatusCode::NOT_FOUND {
        if let Some(event) = state.probe_guard.record_unknown_route(ip, &config) {
            tracing::warn!(
                request_id = %request_id,
                client_ip = %client_ip,
                unknown_hits = event.unknown_hits,
                window_secs = config.window_secs,
                block_secs = config.block_secs,
                "blocking probe client after repeated unknown routes"
            );
            plugin_events::emit_state_event(
                &state,
                PluginEvent::ProbeClientBlocked,
                &ProbeBlockedPayload {
                    request_id: &request_id,
                    client_ip: &client_ip,
                    unknown_hits: event.unknown_hits,
                    block_secs: config.block_secs,
                },
            );
        } else if config.log_unknown_routes {
            tracing::debug!(
                request_id = %request_id,
                client_ip = %client_ip,
                status = status.as_u16(),
                duration_ms,
                "unknown route {method} from {client_ip} {uri}"
            );
        }
        plugin_events::emit_state_event(
            &state,
            PluginEvent::RequestNotFound,
            &RequestPayload {
                request_id: &request_id,
                client_ip: &client_ip,
                method: &method,
                path: &path,
                query: &query,
            },
        );
    } else if config.log_requests {
        tracing::debug!(
            request_id = %request_id,
            client_ip = %client_ip,
            status = status.as_u16(),
            duration_ms,
            "http {method} from {client_ip} {uri}"
        );
    }

    plugin_events::emit_state_event(
        &state,
        PluginEvent::RequestCompleted,
        &RequestCompletedPayload {
            request_id: &request_id,
            client_ip: &client_ip,
            method: &method,
            path: &path,
            query: &query,
            status: status.as_u16(),
            duration_ms,
        },
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(max_404s: u32) -> ProbeProtectionConfig {
        ProbeProtectionConfig {
            enabled: true,
            max_404s,
            window_secs: 60,
            block_secs: 120,
            log_requests: false,
            log_unknown_routes: false,
            log_blocked: false,
        }
    }

    #[test]
    fn blocks_after_unknown_route_threshold() {
        let guard = ProbeGuard::new();
        let config = test_config(3);
        let ip = IpAddr::from([203, 0, 113, 10]);

        assert!(!guard.is_blocked(ip, &config));

        for _ in 0..2 {
            assert!(guard.record_unknown_route(ip, &config).is_none());
            assert!(!guard.is_blocked(ip, &config));
        }

        assert!(guard.record_unknown_route(ip, &config).is_some());
        assert!(guard.is_blocked(ip, &config));
    }

    #[test]
    fn ignores_when_disabled() {
        let guard = ProbeGuard::new();
        let mut config = test_config(1);
        config.enabled = false;
        let ip = IpAddr::from([10, 0, 0, 1]);

        assert!(guard.record_unknown_route(ip, &config).is_none());
        assert!(!guard.is_blocked(ip, &config));
    }
}
