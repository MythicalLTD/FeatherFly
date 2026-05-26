//! Pure-function unit tests across core modules (no Docker/SQLite required).

use featherfly::config::Config;
use featherfly::databases::DatabaseEngine;
use featherfly::docker::{
    absolute_bind_path, error_pages_image, hosting_images, memory_limit_bytes,
};
use featherfly::files::{
    DiskQuotaExceeded, disk_quota_bytes, is_disk_quota_error, projected_disk_usage_after_write,
};
use featherfly::hosting::shared::shared_username;
use featherfly::hosting::{ReconcileSummary, mysql_port_bindings};
use featherfly::proxy::{TraefikEnsureResult, traefik_result_label};

#[test]
fn database_engine_ports_and_images() {
    assert_eq!(DatabaseEngine::Mysql.default_port(), 3306);
    assert_eq!(DatabaseEngine::Postgres.default_port(), 5432);
    assert_eq!(DatabaseEngine::Redis.default_port(), 6379);
    assert_eq!(DatabaseEngine::Mongodb.default_port(), 27017);
    assert_eq!(DatabaseEngine::Mysql.as_str(), "mysql");
}

#[test]
fn shared_username_joins_site_and_user() {
    assert_eq!(shared_username("abc", "mail"), "abc__mail");
}

#[test]
fn absolute_bind_path_resolves_relative_directory() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("pages");
    let abs = absolute_bind_path(&sub).unwrap();
    assert!(abs.starts_with('/'));
    assert!(std::path::Path::new(&abs).exists());
}

#[test]
fn hosting_images_includes_traefik_when_auto_provision() {
    let inner = Config::parse_preview(
        br#"
api:
  port: 9090
proxy:
  enabled: true
  auto_provision: true
  traefik_image: traefik:v3.3
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#,
    )
    .unwrap();
    let images = hosting_images(&inner);
    assert!(images.iter().any(|i| i.contains("traefik")));
    assert!(images.iter().any(|i| i == &inner.hosting.mysql_image));
}

#[test]
fn error_pages_image_is_nginx_alpine() {
    assert_eq!(error_pages_image(), "nginx:alpine");
}

#[test]
fn traefik_result_labels_are_stable() {
    assert_eq!(
        traefik_result_label(TraefikEnsureResult::Created),
        "created"
    );
    assert_eq!(
        traefik_result_label(TraefikEnsureResult::AlreadyRunning),
        "already_running"
    );
}

#[test]
fn reconcile_summary_defaults_to_zero() {
    let s = ReconcileSummary::default();
    assert_eq!(s.sites_total, 0);
    assert!(!s.shared_stack);
}

#[test]
fn mail_port_bindings_respect_custom_host_port() {
    let ports = featherfly::config::HostingPortsConfig {
        smtp: 2525,
        ..Default::default()
    };
    let bindings = featherfly::hosting::mail_port_bindings(&ports);
    let binding = bindings
        .get("25/tcp")
        .and_then(|b| b.as_ref())
        .and_then(|v| v.first())
        .and_then(|b| b.host_port.as_deref());
    assert_eq!(binding, Some("2525"));
}

#[test]
fn mysql_port_bindings_use_configured_host_port() {
    let ports = featherfly::config::HostingPortsConfig {
        mysql: 13306,
        ..Default::default()
    };
    let bindings = mysql_port_bindings(&ports);
    assert!(bindings.contains_key("3306/tcp"));
}

#[test]
fn memory_limit_bytes_converts_mb_for_docker() {
    assert_eq!(memory_limit_bytes(Some(256)), Some(268_435_456));
    assert_eq!(memory_limit_bytes(None), None);
}

#[test]
fn disk_quota_bytes_treats_non_positive_as_unlimited() {
    assert_eq!(disk_quota_bytes(Some(10)), Some(10_485_760));
    assert_eq!(disk_quota_bytes(Some(0)), None);
    assert_eq!(disk_quota_bytes(Some(-1)), None);
    assert_eq!(disk_quota_bytes(None), None);
}

#[test]
fn projected_disk_usage_accounts_for_overwrite() {
    assert_eq!(projected_disk_usage_after_write(1_000, 200, 500), 1_300);
    assert_eq!(projected_disk_usage_after_write(100, 200, 50), 50);
}

#[test]
fn disk_quota_error_is_typed_for_api_mapping() {
    let err: anyhow::Error = DiskQuotaExceeded {
        projected_bytes: 11,
        quota_bytes: 10,
    }
    .into();

    assert!(is_disk_quota_error(&err));
    assert!(err.to_string().contains("disk quota exceeded"));
}
