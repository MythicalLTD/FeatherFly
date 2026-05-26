use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::routes::{GetState, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    name: String,
    status: HealthStatus,
    required: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    status: HealthStatus,
    service: &'static str,
    version: String,
    uptime_seconds: u64,
    checks: Vec<HealthCheck>,
}

pub async fn get(state: GetState) -> impl IntoResponse {
    response(state.0)
}

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

pub async fn ready(state: GetState) -> impl IntoResponse {
    response(state.0)
}

fn response(state: State) -> (StatusCode, Json<HealthResponse>) {
    let checks = collect_checks();
    let body = build_response(&state, checks);
    let status_code = if body.status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    (status_code, Json(body))
}

fn collect_checks() -> Vec<HealthCheck> {
    vec![
        HealthCheck {
            name: "process".into(),
            status: HealthStatus::Ok,
            required: true,
            message: "daemon process is responding".into(),
        },
        HealthCheck {
            name: "config".into(),
            status: HealthStatus::Ok,
            required: true,
            message: "configuration loaded".into(),
        },
    ]
}

fn build_response(state: &State, checks: Vec<HealthCheck>) -> HealthResponse {
    let status = if checks
        .iter()
        .any(|check| check.required && check.status == HealthStatus::Unhealthy)
    {
        HealthStatus::Unhealthy
    } else {
        HealthStatus::Ok
    };

    HealthResponse {
        status,
        service: "featherfly",
        version: state.version.clone(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        checks,
    }
}
