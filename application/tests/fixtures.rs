//! Shared test fixtures for integration tests (Docker-enabled/disabled modes).

use featherfly::routes::{AppContainerType, AppState, State};
use std::sync::Arc;

const BASE_CONFIG: &str = r#"
api:
  host: 127.0.0.1
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#;

pub fn config_with_docker(enabled: bool) -> Arc<featherfly::config::Config> {
    let yaml =
        format!("{BASE_CONFIG}\ndocker:\n  enabled: {enabled}\n  socket: /var/run/docker.sock\n");
    let inner = featherfly::config::Config::parse_preview(yaml.as_bytes()).unwrap();
    let (config, _guard, _meta) = featherfly::config::Config::open_from_inner(
        inner,
        "/tmp/featherfly-test-config.yml",
        false,
        false,
    )
    .unwrap();
    config
}

pub fn app_state(docker_enabled: bool) -> State {
    let config = config_with_docker(docker_enabled);
    let inner = config.load();
    let jwt = Arc::new(featherfly::remote::jwt::JwtClient::new(
        &inner.token,
        inner.api.max_jwt_uses,
    ));
    let docker = None;

    Arc::new(AppState {
        start_time: std::time::Instant::now(),
        container_type: AppContainerType::None,
        version: featherfly::full_version(),
        config,
        plugins: featherfly::plugins::PluginRegistry::empty(),
        probe_guard: featherfly::middlewares::probe::ProbeGuard::new(),
        docker,
        panel: arc_swap::ArcSwapOption::from(None),
        cache: None,
        jwt,
    })
}

pub fn axum_state(docker_enabled: bool) -> axum::extract::State<State> {
    axum::extract::State(app_state(docker_enabled))
}
