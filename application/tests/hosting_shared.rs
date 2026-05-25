//! Shared hosting stack — config + naming conventions.

use featherfly::config::Config;
use featherfly::databases::DatabaseEngine;
use featherfly::hosting::shared_db::{
    SHARED_MONGODB, SHARED_MYSQL, SHARED_POSTGRES, SHARED_REDIS, shared_db_host,
    shared_db_logical_name, shared_db_user,
};

#[test]
fn shared_database_containers_have_stable_names() {
    assert_eq!(SHARED_MYSQL, "featherfly-mysql");
    assert_eq!(SHARED_POSTGRES, "featherfly-postgres");
    assert_eq!(SHARED_REDIS, "featherfly-redis");
    assert_eq!(SHARED_MONGODB, "featherfly-mongodb");
}

#[test]
fn shared_database_hosts_match_container_names() {
    assert_eq!(shared_db_host(DatabaseEngine::Mysql), SHARED_MYSQL);
    assert_eq!(shared_db_host(DatabaseEngine::Postgres), SHARED_POSTGRES);
    assert_eq!(shared_db_host(DatabaseEngine::Redis), SHARED_REDIS);
    assert_eq!(shared_db_host(DatabaseEngine::Mongodb), SHARED_MONGODB);
}

#[test]
fn shared_database_records_are_site_scoped() {
    assert_eq!(
        shared_db_logical_name("abc123", "db7", Some("wordpress")),
        "abc123__wordpress"
    );
    assert_eq!(shared_db_user("abc123", Some("wp"), "db7"), "abc123__wp");
}

#[test]
fn shared_services_config_defaults_to_enabled() {
    let inner = Config::parse_preview(
        br#"
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#,
    )
    .unwrap();

    assert!(inner.hosting.shared_services);
    assert!(inner.hosting.auto_update_images);
    assert!(inner.hosting.reconcile_on_startup);
}

#[test]
fn hosting_images_include_shared_database_stack() {
    let inner = Config::parse_preview(
        br#"
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#,
    )
    .unwrap();

    let images = featherfly::docker::hosting_images(&inner);
    assert!(images.contains(&inner.hosting.mysql_image));
    assert!(images.contains(&inner.hosting.postgres_image));
    assert!(images.contains(&inner.hosting.redis_image));
    assert!(images.contains(&inner.hosting.mongodb_image));
    assert!(images.contains(&inner.hosting.phpmyadmin_image));
}

#[test]
fn config_open_meta_tracks_upgrade_flag() {
    use featherfly::config::{Config, ConfigOpenMeta};
    assert_eq!(
        ConfigOpenMeta {
            identity_generated: false,
            config_upgraded: true,
        },
        ConfigOpenMeta {
            identity_generated: false,
            config_upgraded: true,
        }
    );
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    let sys = format!(
        r"
system:
  root_directory: {}/data
  log_directory: {}/logs
  tmp_directory: {}/tmp
  plugins_directory: {}/plugins
  data: {}/volumes
  archive_directory: {}/archives
  backup_directory: {}/backups
  pid_file: {}/featherfly.pid
hosting:
  templates_directory: {}/templates
",
        base.display(),
        base.display(),
        base.display(),
        base.display(),
        base.display(),
        base.display(),
        base.display(),
        base.display(),
        base.display()
    );
    let config_path = base.join("config.yml");
    std::fs::write(
        &config_path,
        format!(
            "debug: true\napi:\n  port: 9090\nuuid: 11111111-1111-1111-1111-111111111111\ntoken_id: abcdefghijklmnop\ntoken: test-token\n{sys}"
        ),
    )
    .unwrap();
    let (_cfg, _guard, meta) = Config::open(config_path.to_str().unwrap(), true, false).unwrap();
    assert!(meta.config_upgraded);
}
