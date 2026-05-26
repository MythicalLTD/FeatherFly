#[test]
fn bare_config_parses_minimal_daemon_fields() {
    let yaml = br#"
app_name: FeatherFly
token: test-token
api:
  host: 127.0.0.1
  port: 9090
"#;

    let inner = featherfly::config::Config::parse_preview(yaml).unwrap();

    assert_eq!(inner.app_name, "FeatherFly");
    assert_eq!(inner.api.host, "127.0.0.1");
    assert_eq!(inner.token, "test-token");
}

#[test]
fn bearer_auth_uses_plain_node_secret() {
    let inner = featherfly::config::InnerConfig {
        token_id: "node-id".into(),
        token: "node-secret".into(),
        ..Default::default()
    };

    assert!(featherfly::utils::auth::validate_node_bearer(
        "Bearer node-secret",
        &inner
    ));
    assert!(!featherfly::utils::auth::validate_node_bearer(
        "Bearer node-id.node-secret",
        &inner
    ));
}
