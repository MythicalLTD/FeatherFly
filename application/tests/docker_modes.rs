//! Integration tests for predictable Docker-enabled vs Docker-disabled API behavior.

mod fixtures;

use featherfly::controllers::docker::{get as docker_status, require_docker};

#[test]
fn docker_disabled_returns_integration_disabled_message() {
    let state = fixtures::axum_state(false);
    let Err(response) = require_docker(&state) else {
        panic!("expected docker to be unavailable when disabled");
    };

    assert_eq!(response.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let body = axum::body::to_bytes(response.body, 1024).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error"], "docker integration is disabled");
    });
}

#[test]
fn docker_enabled_but_disconnected_returns_unavailable_message() {
    let state = fixtures::axum_state(true);
    let Err(response) = require_docker(&state) else {
        panic!("expected docker to be unavailable when socket is missing");
    };

    assert_eq!(response.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let body = axum::body::to_bytes(response.body, 1024).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value["error"],
            "docker is unavailable — check socket path and daemon status"
        );
    });
}

#[test]
fn docker_status_reports_enabled_flag_from_config() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        for (enabled, expected) in [(false, false), (true, true)] {
            let state = fixtures::axum_state(enabled);
            let Ok(response) = docker_status(state).await else {
                panic!("expected docker status for enabled={enabled}");
            };
            assert_eq!(response.status, axum::http::StatusCode::OK);
            let body = axum::body::to_bytes(response.body, 4096).await.unwrap();
            let status: featherfly::controllers::docker::Response =
                serde_json::from_slice(&body).unwrap();
            assert_eq!(status.enabled, expected);
            assert!(!status.connected);
        }
    });
}

#[test]
fn feature_catalog_marks_plesk_replacement_false() {
    let response = featherfly::feature_status::catalog_response();
    assert!(!response.plesk_replacement);
    assert_eq!(response.product_readiness, "experimental");
    assert!(response.subsystems.iter().any(|s| s.id == "tenancy"));
}
