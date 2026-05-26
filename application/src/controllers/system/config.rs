use crate::{
    routes::GetState,
    utils::{
        audit::AuditEvent,
        plugin_events::{self, ConfigExportedPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize)]
struct GetResponse {
    path: String,
    yaml: String,
    editable: bool,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_system_config",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(GetResponse)), (status = FORBIDDEN, body = inline(crate::utils::response::ApiError))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let inner = state.config.load();
    if !inner.management.config_edit {
        return ApiResponse::error("remote config read is disabled")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    let yaml = match state.config.export_yaml_redacted() {
        Ok(yaml) => yaml,
        Err(err) => {
            return ApiResponse::error(&format!("failed to export config: {err:#}"))
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .ok();
        }
    };

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::ConfigExported,
        &ConfigExportedPayload {
            path: state.config.path.clone(),
        },
    );

    ApiResponse::new_serialized(GetResponse {
        path: state.config.path.clone(),
        yaml,
        editable: inner.management.config_edit,
    })
    .ok()
}

#[derive(ToSchema, serde::Deserialize)]
pub struct PutPayload {
    yaml: String,
    #[serde(default)]
    restart_if_required: bool,
}

#[derive(ToSchema, serde::Serialize)]
struct PutResponse {
    applied: bool,
    requires_restart: bool,
    restart_reasons: Vec<String>,
    restart_scheduled: bool,
}

#[utoipa::path(
    put,
    path = "/",
    operation_id = "put_system_config",
    security(("bearer_auth" = [])),
    request_body = inline(PutPayload),
    responses(
        (status = OK, body = inline(PutResponse)),
        (status = FORBIDDEN, body = inline(crate::utils::response::ApiError)),
        (status = BAD_REQUEST, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn put(state: GetState, axum::Json(body): axum::Json<PutPayload>) -> ApiResponseResult {
    if !state.config.load().management.config_edit {
        return ApiResponse::error("remote config edit is disabled")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    let crate::config::ConfigApplyResult {
        requires_restart,
        restart_reasons,
    } = match state.config.apply_yaml(&body.yaml) {
        Ok(result) => result,
        Err(err) => {
            return ApiResponse::error(&format!("invalid configuration: {err:#}"))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }
    };

    let restart_scheduled =
        body.restart_if_required && requires_restart && state.config.load().management.restart;

    if restart_scheduled {
        plugin_events::emit_json(
            &state.plugins,
            featherfly_plugin_sdk::PluginEvent::RestartScheduled,
            &plugin_events::RestartScheduledPayload {
                delay_ms: 750,
                reason: Some("config apply requires restart".into()),
            },
        );
        crate::daemon_control::schedule_restart(750);
    }

    plugin_events::emit_json(
        &state.plugins,
        featherfly_plugin_sdk::PluginEvent::ConfigApplied,
        &plugin_events::ConfigAppliedPayload {
            path: state.config.path.clone(),
            requires_restart,
            restart_reasons: restart_reasons.clone(),
            restart_scheduled,
        },
    );

    tracing::info!(
        requires_restart,
        restart_scheduled,
        reasons = ?restart_reasons,
        "configuration updated via API"
    );

    crate::utils::audit::record(
        &state,
        AuditEvent::action("config.applied")
            .with_resource("config", state.config.path.clone())
            .with_metadata(serde_json::json!({
                "requires_restart": requires_restart,
                "restart_scheduled": restart_scheduled,
                "restart_reasons": &restart_reasons,
            })),
    )
    .await;

    ApiResponse::new_serialized(PutResponse {
        applied: true,
        requires_restart,
        restart_reasons,
        restart_scheduled,
    })
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_yaml(base: &std::path::Path, config_edit: bool) -> String {
        let data = base.join("data");
        let logs = base.join("logs");
        let tmp = base.join("tmp");
        let plugins = data.join("plugins");
        let volumes = data.join("volumes");
        let archives = data.join("archives");
        let backups = data.join("backups");
        let eggs = data.join("eggs");
        let pid = base.join("featherfly.pid");
        format!(
            r#"debug: true
uuid: 00000000-0000-0000-0000-000000000001
token_id: test-token-id
token: test-token
api:
  host: 127.0.0.1
  port: 9090
management:
  config_edit: {config_edit}
  restart: true
system:
  root_directory: {data}
  log_directory: {logs}
  tmp_directory: {tmp}
  pid_file: {pid}
  plugins_directory: {plugins}
  data: {volumes}
  archive_directory: {archives}
  backup_directory: {backups}
hosting:
  eggs_directory: {eggs}
"#,
            data = data.display(),
            logs = logs.display(),
            tmp = tmp.display(),
            pid = pid.display(),
            plugins = plugins.display(),
            volumes = volumes.display(),
            archives = archives.display(),
            backups = backups.display(),
            eggs = eggs.display(),
        )
    }

    async fn test_state(
        base: &std::path::Path,
    ) -> (
        crate::routes::State,
        tempfile::TempDir,
        crate::config::ConfigGuard,
    ) {
        let config_path = base.join("config.yml");
        std::fs::write(&config_path, test_yaml(base, true)).unwrap();
        let inner =
            crate::config::Config::parse_preview(&std::fs::read(&config_path).unwrap()).unwrap();
        let (config, guard, _) = crate::config::Config::open_from_inner(
            inner,
            config_path.to_str().unwrap(),
            true,
            true,
        )
        .unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        let cache = crate::cache::Cache::open(cache_dir.path().to_str().unwrap())
            .await
            .unwrap();
        let inner = config.load();
        let jwt = Arc::new(crate::remote::jwt::JwtClient::new(
            &inner.token,
            inner.api.max_jwt_uses,
        ));
        let state = Arc::new(crate::routes::AppState {
            start_time: std::time::Instant::now(),
            container_type: crate::routes::AppContainerType::None,
            version: crate::full_version(),
            config,
            plugins: crate::plugins::PluginRegistry::empty(),
            probe_guard: crate::middlewares::probe::ProbeGuard::new(),
            docker: None,
            panel: arc_swap::ArcSwapOption::from(None),
            cache: Some(Arc::new(cache)),
            jwt,
        });
        (state, cache_dir, guard)
    }

    #[test]
    fn config_apply_writes_audit_event() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let (state, _cache_dir, _guard) = test_state(dir.path()).await;

            let yaml = test_yaml(dir.path(), true);
            let response = put(
                axum::extract::State(Arc::clone(&state)),
                axum::Json(PutPayload {
                    yaml,
                    restart_if_required: false,
                }),
            )
            .await;
            assert!(response.is_ok());

            let events = state
                .cache
                .as_ref()
                .unwrap()
                .list_audit_events(10)
                .await
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].action, "config.applied");
            assert_eq!(events[0].resource_type.as_deref(), Some("config"));
            assert_eq!(events[0].success, 1);
        });
    }
}
