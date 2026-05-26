use axum::{Json, http::StatusCode, response::IntoResponse};
use cron::Schedule;
use serde::Serialize;
use std::str::FromStr;
use utoipa::ToSchema;

use crate::{
    proxy::traefik_runtime_status,
    routes::{GetState, State},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Unhealthy,
    Skipped,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HealthCheck {
    name: String,
    status: HealthStatus,
    required: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HealthResponse {
    status: HealthStatus,
    service: &'static str,
    version: String,
    uptime_seconds: u64,
    checks: Vec<HealthCheck>,
}

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "get_health",
    responses(
        (status = OK, body = inline(HealthResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(HealthResponse)),
    ),
)]
pub async fn get(state: GetState) -> impl IntoResponse {
    response(state.0).await
}

#[utoipa::path(
    get,
    path = "/live",
    operation_id = "get_live",
    responses((status = OK, body = inline(HealthResponse))),
)]
pub async fn live(state: GetState) -> impl IntoResponse {
    let checks = vec![HealthCheck {
        name: "process".into(),
        status: HealthStatus::Ok,
        required: true,
        message: "daemon process is responding".into(),
    }];
    let body = build_response(&state.0, checks);
    (StatusCode::OK, Json(body))
}

#[utoipa::path(
    get,
    path = "/ready",
    operation_id = "get_ready",
    responses(
        (status = OK, body = inline(HealthResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(HealthResponse)),
    ),
)]
pub async fn ready(state: GetState) -> impl IntoResponse {
    response(state.0).await
}

async fn response(state: State) -> (StatusCode, Json<HealthResponse>) {
    let checks = collect_checks(&state).await;
    let body = build_response(&state, checks);
    let status_code = if body.status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    (status_code, Json(body))
}

async fn collect_checks(state: &State) -> Vec<HealthCheck> {
    let inner = state.config.load();
    let mut checks = Vec::new();

    checks.push(HealthCheck {
        name: "config".into(),
        status: HealthStatus::Ok,
        required: true,
        message: "configuration loaded".into(),
    });

    checks.push(match &state.cache {
        Some(cache) => match cache.ping().await {
            Ok(()) => HealthCheck {
                name: "sqlite_cache".into(),
                status: HealthStatus::Ok,
                required: true,
                message: "sqlite cache is reachable".into(),
            },
            Err(err) => HealthCheck {
                name: "sqlite_cache".into(),
                status: HealthStatus::Unhealthy,
                required: true,
                message: err.to_string(),
            },
        },
        None => HealthCheck {
            name: "sqlite_cache".into(),
            status: HealthStatus::Unhealthy,
            required: true,
            message: "sqlite cache is unavailable".into(),
        },
    });

    checks.push(match &state.docker {
        Some(docker) => match docker.ping().await {
            Ok(()) => HealthCheck {
                name: "docker".into(),
                status: HealthStatus::Ok,
                required: inner.docker.enabled,
                message: format!("docker is reachable at {}", docker.socket()),
            },
            Err(err) => HealthCheck {
                name: "docker".into(),
                status: HealthStatus::Unhealthy,
                required: inner.docker.enabled,
                message: err.to_string(),
            },
        },
        None if inner.docker.enabled => HealthCheck {
            name: "docker".into(),
            status: HealthStatus::Unhealthy,
            required: true,
            message: "docker is enabled but unavailable".into(),
        },
        None => HealthCheck {
            name: "docker".into(),
            status: HealthStatus::Skipped,
            required: false,
            message: "docker integration is disabled".into(),
        },
    });

    let traefik_required =
        inner.proxy.enabled && inner.proxy.provider == "traefik" && inner.proxy.auto_provision;
    checks.push(if traefik_required {
        let runtime = traefik_runtime_status(state.docker.as_deref(), &inner).await;
        if runtime.running {
            HealthCheck {
                name: "traefik".into(),
                status: HealthStatus::Ok,
                required: true,
                message: "Traefik reverse proxy is running".into(),
            }
        } else {
            HealthCheck {
                name: "traefik".into(),
                status: HealthStatus::Unhealthy,
                required: true,
                message: "Traefik reverse proxy is not running".into(),
            }
        }
    } else {
        HealthCheck {
            name: "traefik".into(),
            status: HealthStatus::Skipped,
            required: false,
            message: "Traefik auto-provision is disabled or not selected".into(),
        }
    });

    checks.push(plugin_check(&state.plugins));

    checks.push(scheduler_check(state).await);
    checks.push(site_readiness_check(state).await);

    checks
}

fn plugin_check(registry: &crate::plugins::PluginRegistry) -> HealthCheck {
    let summaries = registry.summaries();
    let load_errors = registry.load_errors();
    let status = if load_errors.is_empty() {
        HealthStatus::Ok
    } else {
        HealthStatus::Degraded
    };
    let message = if load_errors.is_empty() {
        format!(
            "{} plugin(s), {} hook(s), {} route(s)",
            summaries.len(),
            registry.hook_count(),
            registry.routes().len()
        )
    } else {
        format!(
            "{} plugin(s), {} hook(s), {} route(s), {} load error(s)",
            summaries.len(),
            registry.hook_count(),
            registry.routes().len(),
            load_errors.len()
        )
    };

    HealthCheck {
        name: "plugins".into(),
        status,
        required: false,
        message,
    }
}

async fn scheduler_check(state: &State) -> HealthCheck {
    let Some(cache) = &state.cache else {
        return HealthCheck {
            name: "scheduler".into(),
            status: HealthStatus::Skipped,
            required: false,
            message: "scheduler readiness skipped because cache is unavailable".into(),
        };
    };

    match cache.list_enabled_tasks().await {
        Ok(tasks) => {
            let invalid_cron = tasks
                .iter()
                .filter(|task| {
                    task.kind == "cron"
                        && task
                            .schedule
                            .as_deref()
                            .is_none_or(|schedule| Schedule::from_str(schedule).is_err())
                })
                .count();
            if invalid_cron > 0 {
                HealthCheck {
                    name: "scheduler".into(),
                    status: HealthStatus::Degraded,
                    required: false,
                    message: format!(
                        "{} enabled task(s), {invalid_cron} invalid cron schedule(s)",
                        tasks.len()
                    ),
                }
            } else {
                HealthCheck {
                    name: "scheduler".into(),
                    status: HealthStatus::Ok,
                    required: false,
                    message: format!("{} enabled task(s) loaded", tasks.len()),
                }
            }
        }
        Err(err) => HealthCheck {
            name: "scheduler".into(),
            status: HealthStatus::Unhealthy,
            required: false,
            message: format!("failed to load enabled tasks: {err}"),
        },
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SiteReadinessSummary {
    total: usize,
    without_container: usize,
    running: usize,
    not_running: usize,
    inspect_failed: usize,
}

impl SiteReadinessSummary {
    fn status(self) -> HealthStatus {
        if self.total == 0 || (self.not_running == 0 && self.inspect_failed == 0) {
            HealthStatus::Ok
        } else {
            HealthStatus::Degraded
        }
    }

    fn message(self) -> String {
        format!(
            "{} site(s): {} running, {} not running, {} inspect failed, {} without container",
            self.total, self.running, self.not_running, self.inspect_failed, self.without_container
        )
    }
}

async fn site_readiness_check(state: &State) -> HealthCheck {
    let Some(cache) = &state.cache else {
        return HealthCheck {
            name: "sites".into(),
            status: HealthStatus::Skipped,
            required: false,
            message: "site readiness skipped because cache is unavailable".into(),
        };
    };

    let Ok(sites) = cache.list_sites().await else {
        return HealthCheck {
            name: "sites".into(),
            status: HealthStatus::Unhealthy,
            required: false,
            message: "failed to list sites".into(),
        };
    };

    let Some(docker) = &state.docker else {
        return HealthCheck {
            name: "sites".into(),
            status: HealthStatus::Skipped,
            required: false,
            message: format!(
                "{} site(s) tracked; container readiness skipped because Docker is unavailable",
                sites.len()
            ),
        };
    };

    let mut summary = SiteReadinessSummary {
        total: sites.len(),
        ..SiteReadinessSummary::default()
    };

    for site in sites {
        let Some(container_id) = site.container_id else {
            summary.without_container += 1;
            continue;
        };
        match docker.inspect_container(&container_id).await {
            Ok(container) => {
                if container
                    .state
                    .and_then(|state| state.running)
                    .unwrap_or(false)
                {
                    summary.running += 1;
                } else {
                    summary.not_running += 1;
                }
            }
            Err(_) => summary.inspect_failed += 1,
        }
    }

    HealthCheck {
        name: "sites".into(),
        status: summary.status(),
        required: false,
        message: summary.message(),
    }
}

fn build_response(state: &State, checks: Vec<HealthCheck>) -> HealthResponse {
    HealthResponse {
        status: overall_status(&checks),
        service: "featherfly",
        version: state.version.clone(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        checks,
    }
}

fn overall_status(checks: &[HealthCheck]) -> HealthStatus {
    if checks
        .iter()
        .any(|check| check.required && check.status == HealthStatus::Unhealthy)
    {
        return HealthStatus::Unhealthy;
    }
    if checks.iter().any(|check| {
        matches!(
            check.status,
            HealthStatus::Degraded | HealthStatus::Unhealthy
        )
    }) {
        return HealthStatus::Degraded;
    }
    HealthStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(status: HealthStatus, required: bool) -> HealthCheck {
        HealthCheck {
            name: "test".into(),
            status,
            required,
            message: String::new(),
        }
    }

    #[test]
    fn required_failure_makes_health_unhealthy() {
        let checks = [
            check(HealthStatus::Ok, true),
            check(HealthStatus::Unhealthy, true),
        ];
        assert_eq!(overall_status(&checks), HealthStatus::Unhealthy);
    }

    #[test]
    fn optional_failure_makes_health_degraded() {
        let checks = [
            check(HealthStatus::Ok, true),
            check(HealthStatus::Unhealthy, false),
        ];
        assert_eq!(overall_status(&checks), HealthStatus::Degraded);
    }

    #[test]
    fn skipped_checks_do_not_affect_health() {
        let checks = [
            check(HealthStatus::Ok, true),
            check(HealthStatus::Skipped, false),
        ];
        assert_eq!(overall_status(&checks), HealthStatus::Ok);
    }

    #[test]
    fn site_readiness_degrades_for_container_problems() {
        let summary = SiteReadinessSummary {
            total: 3,
            running: 1,
            not_running: 1,
            inspect_failed: 1,
            without_container: 0,
        };
        assert_eq!(summary.status(), HealthStatus::Degraded);
    }

    #[test]
    fn site_readiness_ok_when_no_container_problems() {
        let summary = SiteReadinessSummary {
            total: 2,
            running: 1,
            without_container: 1,
            ..SiteReadinessSummary::default()
        };
        assert_eq!(summary.status(), HealthStatus::Ok);
    }

    #[test]
    fn plugin_load_errors_degrade_health() {
        let registry = crate::plugins::PluginRegistry::with_load_error(
            "/tmp/bad.so",
            "plugin API version mismatch",
        );
        let check = plugin_check(&registry);

        assert_eq!(check.status, HealthStatus::Degraded);
        assert!(check.message.contains("1 load error"));
    }
}
