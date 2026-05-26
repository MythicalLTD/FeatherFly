use crate::{
    backups,
    cache::BackupRecord,
    controllers::docker::require_docker,
    databases::{CreateDatabaseRequest, DatabaseService},
    dns::{DnsService, DnsZoneStatus, SyncDnsRequest},
    files::{FileEntry, FileService, MoveCopyRequest, RenameRequest, is_disk_quota_error},
    ftp::{CreateFtpAccountRequest, FtpService, FtpSyncRequest},
    hosting::{HostingOverview, HostingService, SiteStats, SiteStatsService, UpdateLimitsRequest},
    mail::{CreateMailAccountRequest, MailService, MailSyncRequest},
    routes::GetState,
    scheduler::{CreateTaskRequest, SchedulerService},
    sites::{DeployRequest, ProvisionSiteRequest, SiteExecRequest, SiteService, UpdateSiteRequest},
    utils::{
        audit::AuditEvent,
        plugin_events::{
            self, BackupCompletedPayload, BackupStartedPayload, WebhookReceivedPayload,
        },
        response::{ApiResponse, ApiResponseResult},
    },
};
use axum::http::StatusCode;
use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_sites",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(Vec<crate::sites::SiteSummary>))),
)]
pub async fn list(state: GetState) -> ApiResponseResult {
    match SiteService::list(&state).await {
        Ok(sites) => ApiResponse::new_serialized(sites).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list sites: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/",
    operation_id = "post_sites",
    security(("bearer_auth" = [])),
    request_body = inline(ProvisionSiteRequest),
    responses((status = CREATED, body = inline(crate::sites::SiteSummary))),
)]
pub async fn create(
    state: GetState,
    axum::Json(body): axum::Json<ProvisionSiteRequest>,
) -> ApiResponseResult {
    match SiteService::provision(&state, body).await {
        Ok(site) => {
            audit_site(
                &state,
                "site.created",
                &site.id,
                serde_json::json!({
                    "domain": site.domain,
                    "egg_id": site.egg_id,
                }),
            )
            .await;
            ApiResponse::new_serialized(site)
                .with_status(StatusCode::CREATED)
                .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to provision site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "get_sites_by_id",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(crate::sites::SiteSummary))),
)]
pub async fn get(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteService::get(&state, &id).await {
        Ok(Some(site)) => ApiResponse::new_serialized(site).ok(),
        Ok(None) => ApiResponse::error("site not found")
            .with_status(StatusCode::NOT_FOUND)
            .ok(),
        Err(err) => ApiResponse::error(&format!("failed to get site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "patch_sites",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(UpdateSiteRequest),
    responses((status = OK, body = inline(crate::sites::SiteSummary))),
)]
pub async fn update(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<UpdateSiteRequest>,
) -> ApiResponseResult {
    match SiteService::update(&state, &id, body).await {
        Ok(site) => {
            audit_site(
                &state,
                "site.updated",
                &site.id,
                serde_json::json!({
                    "domain": site.domain,
                    "egg_id": site.egg_id,
                    "status": site.status,
                }),
            )
            .await;
            ApiResponse::new_serialized(site).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to update site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/suspend",
    operation_id = "post_sites_suspend",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(crate::sites::SiteSummary))),
)]
pub async fn suspend(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteService::suspend(&state, &id).await {
        Ok(site) => {
            audit_site(
                &state,
                "site.suspended",
                &site.id,
                serde_json::json!({ "status": site.status }),
            )
            .await;
            ApiResponse::new_serialized(site).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to suspend site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/unsuspend",
    operation_id = "post_sites_unsuspend",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(crate::sites::SiteSummary))),
)]
pub async fn unsuspend(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteService::unsuspend(&state, &id).await {
        Ok(site) => {
            audit_site(
                &state,
                "site.unsuspended",
                &site.id,
                serde_json::json!({ "status": site.status }),
            )
            .await;
            ApiResponse::new_serialized(site).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to unsuspend site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(ToSchema, Deserialize)]
pub struct DestroyQuery {
    #[serde(default)]
    pub force: bool,
}

#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "delete_sites",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("force" = bool, Query)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn destroy(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<DestroyQuery>,
) -> ApiResponseResult {
    match SiteService::destroy(&state, &id, query.force).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.destroyed",
                &id,
                serde_json::json!({ "force": query.force }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to destroy site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/deploy",
    operation_id = "post_sites_deploy",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(DeployRequest),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn deploy(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<DeployRequest>,
) -> ApiResponseResult {
    let metadata = serde_json::json!({
        "git_ref": &body.git_ref,
        "rebuild": body.rebuild,
        "zero_downtime": body.zero_downtime,
    });
    match SiteService::deploy(&state, &id, body).await {
        Ok(()) => {
            audit_site(&state, "site.deployed", &id, metadata).await;
            ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("deploy failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(ToSchema, Deserialize)]
pub struct WebhookDeployBody {
    pub git_ref: Option<String>,
}

#[utoipa::path(
    post,
    path = "/{id}/deploy/webhook",
    operation_id = "post_sites_deploy_webhook",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(WebhookDeployBody),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn webhook_deploy(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<WebhookDeployBody>,
) -> ApiResponseResult {
    let git_ref = body.git_ref.clone();
    plugin_events::emit_state_event(
        &state,
        PluginEvent::WebhookReceived,
        &WebhookReceivedPayload {
            site_id: &id,
            source: "deploy_webhook",
        },
    );
    match SiteService::deploy(
        &state,
        &id,
        DeployRequest {
            git_ref: body.git_ref,
            rebuild: false,
            zero_downtime: false,
        },
    )
    .await
    {
        Ok(()) => {
            audit_site(
                &state,
                "site.webhook_deployed",
                &id,
                serde_json::json!({ "git_ref": git_ref }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("webhook deploy failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/backups",
    operation_id = "post_sites_backups",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = CREATED, body = inline(BackupRecord))),
)]
pub async fn backup_create(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(d) => d,
        Err(r) => return Err(r),
    };
    let cache = match state.cache.as_ref() {
        Some(c) => c,
        None => {
            return ApiResponse::error("cache unavailable")
                .with_status(StatusCode::SERVICE_UNAVAILABLE)
                .ok();
        }
    };
    let inner = state.config.load();
    let site = match cache.get_site(&id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return ApiResponse::error("site not found")
                .with_status(StatusCode::NOT_FOUND)
                .ok();
        }
        Err(err) => {
            return ApiResponse::error(&format!("cache error: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok();
        }
    };

    plugin_events::emit_state_event(
        &state,
        PluginEvent::BackupStarted,
        &BackupStartedPayload { site_id: &id },
    );

    match backups::create_backup(&inner, docker, cache, &site).await {
        Ok(result) => {
            let backup_id = result.id.clone();
            let backup_path = result.path.display().to_string();
            plugin_events::emit_state_event(
                &state,
                PluginEvent::BackupCompleted,
                &BackupCompletedPayload {
                    site_id: &id,
                    backup_id: &result.id,
                },
            );
            audit_site(
                &state,
                "site.backup_created",
                &id,
                serde_json::json!({
                    "backup_id": backup_id,
                    "path": backup_path,
                }),
            )
            .await;
            ApiResponse::new_serialized(BackupRecord {
                id: result.id,
                site_id: id,
                path: result.path.display().to_string(),
                created_at: chrono::Utc::now().timestamp(),
            })
            .with_status(StatusCode::CREATED)
            .ok()
        }
        Err(err) => {
            plugin_events::emit_json(
                &state.plugins,
                PluginEvent::BackupFailed,
                &serde_json::json!({ "site_id": id, "error": err.to_string() }),
            );
            ApiResponse::error(&format!("backup failed: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
                .ok()
        }
    }
}

#[utoipa::path(
    get,
    path = "/{id}/backups",
    operation_id = "get_sites_backups",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(Vec<BackupRecord>))),
)]
pub async fn backup_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let Some(cache) = state.cache.as_ref() else {
        return ApiResponse::error("cache unavailable")
            .with_status(StatusCode::SERVICE_UNAVAILABLE)
            .ok();
    };
    match cache.list_backups(&id).await {
        Ok(list) => ApiResponse::new_serialized(list).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list backups: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/backups/{backup_id}/restore",
    operation_id = "post_sites_backup_restore",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("backup_id" = String, Path)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn backup_restore(
    state: GetState,
    axum::extract::Path((id, backup_id)): axum::extract::Path<(String, String)>,
) -> ApiResponseResult {
    let docker = match require_docker(&state) {
        Ok(d) => d,
        Err(r) => return Err(r),
    };
    let cache = state.cache.as_ref().ok_or_else(|| {
        ApiResponse::error("cache unavailable").with_status(StatusCode::SERVICE_UNAVAILABLE)
    })?;
    let site = cache
        .get_site(&id)
        .await
        .map_err(|err| {
            ApiResponse::error(&format!("cache error: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
        })?
        .ok_or_else(|| ApiResponse::error("site not found").with_status(StatusCode::NOT_FOUND))?;
    let backup = cache
        .get_backup(&backup_id)
        .await
        .map_err(|err| {
            ApiResponse::error(&format!("cache error: {err:#}"))
                .with_status(StatusCode::BAD_GATEWAY)
        })?
        .ok_or_else(|| ApiResponse::error("backup not found").with_status(StatusCode::NOT_FOUND))?;

    match backups::restore_backup(docker, &site, std::path::Path::new(&backup.path)).await {
        Ok(()) => {
            plugin_events::emit_state_event(
                &state,
                PluginEvent::BackupRestored,
                &BackupCompletedPayload {
                    site_id: &id,
                    backup_id: &backup_id,
                },
            );
            audit_site(
                &state,
                "site.backup_restored",
                &id,
                serde_json::json!({ "backup_id": backup_id }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("restore failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ActionResponse {
    pub id: String,
    pub ok: bool,
}

#[derive(ToSchema, Deserialize)]
pub struct SiteLogsQuery {
    pub tail: Option<String>,
    pub grep: Option<String>,
    #[serde(default)]
    pub download: bool,
}

#[utoipa::path(
    get,
    path = "/{id}/logs",
    operation_id = "get_sites_logs",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("tail" = String, Query), ("grep" = String, Query)),
    responses((status = OK, description = "Site container logs")),
)]
pub async fn logs(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SiteLogsQuery>,
) -> ApiResponseResult {
    if query.download {
        match SiteService::logs_download(&state, &id, query.tail).await {
            Ok((filename, text)) => {
                return ApiResponse::new(axum::body::Body::from(text))
                    .with_header("content-type", "text/plain; charset=utf-8")
                    .with_header(
                        "content-disposition",
                        format!("attachment; filename=\"{filename}\""),
                    )
                    .ok();
            }
            Err(err) => {
                return ApiResponse::error(&format!("failed to download logs: {err:#}"))
                    .with_status(StatusCode::BAD_GATEWAY)
                    .ok();
            }
        }
    }

    match SiteService::logs(&state, &id, query.tail, query.grep).await {
        Ok(text) => ApiResponse::new(axum::body::Body::from(text))
            .with_header("content-type", "text/plain; charset=utf-8")
            .ok(),
        Err(err) => ApiResponse::error(&format!("failed to fetch logs: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/logs/snapshot",
    operation_id = "post_sites_logs_snapshot",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(crate::sites::LogArchiveEntry))),
)]
pub async fn logs_snapshot(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteService::snapshot_logs(&state, &id).await {
        Ok(entry) => ApiResponse::new_serialized(entry).ok(),
        Err(err) => ApiResponse::error(&format!("failed to snapshot logs: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/logs/archives",
    operation_id = "get_sites_logs_archives",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(Vec<crate::sites::LogArchiveEntry>))),
)]
pub async fn logs_archives_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteService::list_log_archives(&state, &id).await {
        Ok(entries) => ApiResponse::new_serialized(entries).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list log archives: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/logs/archives/{name}",
    operation_id = "get_sites_logs_archive",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("name" = String, Path)),
    responses((status = OK, description = "Archived log file")),
)]
pub async fn logs_archive_get(
    state: GetState,
    axum::extract::Path((id, name)): axum::extract::Path<(String, String)>,
) -> ApiResponseResult {
    match SiteService::read_log_archive(&state, &id, &name).await {
        Ok(bytes) => ApiResponse::new(axum::body::Body::from(bytes))
            .with_header("content-type", "text/plain; charset=utf-8")
            .with_header(
                "content-disposition",
                format!("attachment; filename=\"{name}\""),
            )
            .ok(),
        Err(err) => ApiResponse::error(&format!("failed to read log archive: {err:#}"))
            .with_status(StatusCode::NOT_FOUND)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/reinstall",
    operation_id = "post_sites_reinstall",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn reinstall(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteService::reinstall(&state, &id).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.reinstalled",
                &id,
                serde_json::json!({ "ok": true }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to reinstall site: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/databases",
    operation_id = "get_sites_databases",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(Vec<crate::databases::DatabaseSummary>))),
)]
pub async fn databases_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match DatabaseService::list(&state, &id).await {
        Ok(list) => ApiResponse::new_serialized(list).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list databases: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/databases",
    operation_id = "post_sites_databases",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(CreateDatabaseRequest),
    responses((status = CREATED, body = inline(crate::databases::DatabaseSummary))),
)]
pub async fn databases_create(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<CreateDatabaseRequest>,
) -> ApiResponseResult {
    match DatabaseService::create(&state, &id, body).await {
        Ok(db) => {
            audit_site(
                &state,
                "site.database_created",
                &id,
                serde_json::json!({
                    "database_id": &db.id,
                    "engine": &db.engine,
                    "host": &db.host,
                    "port": db.port,
                }),
            )
            .await;
            ApiResponse::new_serialized(db)
                .with_status(StatusCode::CREATED)
                .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to create database: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(ToSchema, Deserialize)]
pub struct DatabaseDestroyQuery {
    #[serde(default)]
    pub force: bool,
}

#[utoipa::path(
    delete,
    path = "/{id}/databases/{db_id}",
    operation_id = "delete_sites_database",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("db_id" = String, Path), ("force" = bool, Query)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn databases_destroy(
    state: GetState,
    axum::extract::Path((id, db_id)): axum::extract::Path<(String, String)>,
    axum::extract::Query(query): axum::extract::Query<DatabaseDestroyQuery>,
) -> ApiResponseResult {
    match DatabaseService::remove(&state, &id, &db_id, query.force).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.database_deleted",
                &id,
                serde_json::json!({
                    "database_id": &db_id,
                    "force": query.force,
                }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse {
                id: db_id,
                ok: true,
            })
            .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to remove database: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/tasks",
    operation_id = "get_sites_tasks",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(Vec<crate::scheduler::TaskSummary>))),
)]
pub async fn tasks_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SchedulerService::list(&state, &id).await {
        Ok(list) => ApiResponse::new_serialized(list).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list tasks: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/tasks",
    operation_id = "post_sites_tasks",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(CreateTaskRequest),
    responses((status = CREATED, body = inline(crate::scheduler::TaskSummary))),
)]
pub async fn tasks_create(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<CreateTaskRequest>,
) -> ApiResponseResult {
    match SchedulerService::create(&state, &id, body).await {
        Ok(task) => {
            audit_site(
                &state,
                "site.task_created",
                &id,
                serde_json::json!({
                    "task_id": &task.id,
                    "kind": &task.kind,
                    "action": &task.action,
                    "enabled": task.enabled,
                }),
            )
            .await;
            ApiResponse::new_serialized(task)
                .with_status(StatusCode::CREATED)
                .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to create task: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/tasks/{task_id}/run",
    operation_id = "post_sites_task_run",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("task_id" = String, Path)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn tasks_run(
    state: GetState,
    axum::extract::Path((id, task_id)): axum::extract::Path<(String, String)>,
) -> ApiResponseResult {
    match SchedulerService::run_now(&state, &id, &task_id).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.task_run",
                &id,
                serde_json::json!({ "task_id": &task_id }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse {
                id: task_id,
                ok: true,
            })
            .ok()
        }
        Err(err) => ApiResponse::error(&format!("task run failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    delete,
    path = "/{id}/tasks/{task_id}",
    operation_id = "delete_sites_task",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("task_id" = String, Path)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn tasks_destroy(
    state: GetState,
    axum::extract::Path((id, task_id)): axum::extract::Path<(String, String)>,
) -> ApiResponseResult {
    match SchedulerService::delete(&state, &id, &task_id).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.task_deleted",
                &id,
                serde_json::json!({ "task_id": &task_id }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse {
                id: task_id,
                ok: true,
            })
            .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to delete task: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/dns",
    operation_id = "get_sites_dns",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(DnsZoneStatus))),
)]
pub async fn dns_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match DnsService::list(&state, &id).await {
        Ok(records) => ApiResponse::new_serialized(records).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list dns records: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    put,
    path = "/{id}/dns",
    operation_id = "put_sites_dns",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(SyncDnsRequest),
    responses((status = OK, body = inline(DnsZoneStatus))),
)]
pub async fn dns_sync(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<SyncDnsRequest>,
) -> ApiResponseResult {
    let record_count = body.records.len();
    let dnssec_enabled = body.dnssec_enabled;
    let ds_count = body.ds_records.len();
    match DnsService::sync(&state, &id, body).await {
        Ok(records) => {
            audit_site(
                &state,
                "site.dns_synced",
                &id,
                serde_json::json!({
                    "record_count": record_count,
                    "dnssec_enabled": dnssec_enabled,
                    "ds_count": ds_count,
                }),
            )
            .await;
            ApiResponse::new_serialized(records).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to sync dns: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/mail",
    operation_id = "get_sites_mail",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(crate::mail::MailStatus))),
)]
pub async fn mail_status(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match MailService::status(&state, &id).await {
        Ok(status) => ApiResponse::new_serialized(status).ok(),
        Err(err) => ApiResponse::error(&format!("failed to get mail status: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/mail/accounts",
    operation_id = "get_sites_mail_accounts",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(Vec<crate::mail::MailAccountSummary>))),
)]
pub async fn mail_accounts_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match MailService::list_accounts(&state, &id).await {
        Ok(list) => ApiResponse::new_serialized(list).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list mail accounts: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/mail/accounts",
    operation_id = "post_sites_mail_accounts",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(CreateMailAccountRequest),
    responses((status = CREATED, body = inline(crate::mail::MailAccountSummary))),
)]
pub async fn mail_accounts_create(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<CreateMailAccountRequest>,
) -> ApiResponseResult {
    match MailService::create_account(&state, &id, body).await {
        Ok(account) => {
            audit_site(
                &state,
                "site.mail_account_created",
                &id,
                serde_json::json!({
                    "account_id": &account.id,
                    "address": &account.address,
                    "quota_mb": account.quota_mb,
                }),
            )
            .await;
            ApiResponse::new_serialized(account)
                .with_status(StatusCode::CREATED)
                .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to create mail account: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    delete,
    path = "/{id}/mail/accounts/{account_id}",
    operation_id = "delete_sites_mail_account",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("account_id" = String, Path)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn mail_accounts_destroy(
    state: GetState,
    axum::extract::Path((id, account_id)): axum::extract::Path<(String, String)>,
) -> ApiResponseResult {
    match MailService::remove_account(&state, &id, &account_id).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.mail_account_deleted",
                &id,
                serde_json::json!({ "account_id": &account_id }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse {
                id: account_id,
                ok: true,
            })
            .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to remove mail account: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    put,
    path = "/{id}/mail/sync",
    operation_id = "put_sites_mail_sync",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(MailSyncRequest),
    responses((status = OK, body = inline(crate::mail::MailSyncResult))),
)]
pub async fn mail_sync(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<MailSyncRequest>,
) -> ApiResponseResult {
    let metadata = serde_json::json!({
        "provision_mailserver": body.provision_mailserver,
        "provision_roundcube": body.provision_roundcube,
        "mail_domain": &body.mail_domain,
    });
    match MailService::sync(&state, &id, body).await {
        Ok(result) => {
            audit_site(&state, "site.mail_synced", &id, metadata).await;
            ApiResponse::new_serialized(result).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to sync mail: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/ftp",
    operation_id = "get_sites_ftp",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(crate::ftp::FtpStatus))),
)]
pub async fn ftp_status(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match FtpService::status(&state, &id).await {
        Ok(status) => ApiResponse::new_serialized(status).ok(),
        Err(err) => ApiResponse::error(&format!("failed to get ftp status: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/ftp/accounts",
    operation_id = "get_sites_ftp_accounts",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(Vec<crate::ftp::FtpAccountSummary>))),
)]
pub async fn ftp_accounts_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match FtpService::list_accounts(&state, &id).await {
        Ok(list) => ApiResponse::new_serialized(list).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list ftp accounts: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/ftp/accounts",
    operation_id = "post_sites_ftp_accounts",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(CreateFtpAccountRequest),
    responses((status = CREATED, body = inline(crate::ftp::FtpAccountSummary))),
)]
pub async fn ftp_accounts_create(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<CreateFtpAccountRequest>,
) -> ApiResponseResult {
    match FtpService::create_account(&state, &id, body).await {
        Ok(account) => {
            audit_site(
                &state,
                "site.ftp_account_created",
                &id,
                serde_json::json!({
                    "account_id": &account.id,
                    "username": &account.username,
                    "protocol": &account.protocol,
                    "home_subpath": &account.home_subpath,
                }),
            )
            .await;
            ApiResponse::new_serialized(account)
                .with_status(StatusCode::CREATED)
                .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to create ftp account: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    delete,
    path = "/{id}/ftp/accounts/{account_id}",
    operation_id = "delete_sites_ftp_account",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("account_id" = String, Path)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn ftp_accounts_destroy(
    state: GetState,
    axum::extract::Path((id, account_id)): axum::extract::Path<(String, String)>,
) -> ApiResponseResult {
    match FtpService::remove_account(&state, &id, &account_id).await {
        Ok(()) => {
            audit_site(
                &state,
                "site.ftp_account_deleted",
                &id,
                serde_json::json!({ "account_id": &account_id }),
            )
            .await;
            ApiResponse::new_serialized(ActionResponse {
                id: account_id,
                ok: true,
            })
            .ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to remove ftp account: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    put,
    path = "/{id}/ftp/sync",
    operation_id = "put_sites_ftp_sync",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(FtpSyncRequest),
    responses((status = OK, body = inline(crate::ftp::FtpSyncResult))),
)]
pub async fn ftp_sync(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<FtpSyncRequest>,
) -> ApiResponseResult {
    let metadata = serde_json::json!({ "reprovision": body.reprovision });
    match FtpService::sync(&state, &id, body).await {
        Ok(result) => {
            audit_site(&state, "site.ftp_synced", &id, metadata).await;
            ApiResponse::new_serialized(result).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to sync ftp gateway: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/exec",
    operation_id = "post_sites_exec",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(SiteExecRequest),
    responses((status = OK, body = inline(crate::sites::SiteExecResponse))),
)]
pub async fn exec(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<SiteExecRequest>,
) -> ApiResponseResult {
    let metadata = serde_json::json!({
        "shell": body.shell,
        "cmd_len": body.cmd.len(),
        "cmd_first": body.cmd.first(),
    });
    match SiteService::exec(&state, &id, body).await {
        Ok(result) => {
            audit_site(&state, "site.exec", &id, metadata).await;
            ApiResponse::new_serialized(result).ok()
        }
        Err(err) => ApiResponse::error(&format!("exec failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/hosting",
    operation_id = "get_sites_hosting",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(HostingOverview))),
)]
pub async fn hosting_overview(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match HostingService::overview(&state, &id).await {
        Ok(overview) => ApiResponse::new_serialized(overview).ok(),
        Err(err) => ApiResponse::error(&format!("failed to load hosting overview: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    put,
    path = "/{id}/hosting/limits",
    operation_id = "put_sites_hosting_limits",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(UpdateLimitsRequest),
    responses((status = OK, body = inline(crate::hosting::HostingLimits))),
)]
pub async fn hosting_limits(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<UpdateLimitsRequest>,
) -> ApiResponseResult {
    let metadata = serde_json::to_value(&body).unwrap_or_else(|_| serde_json::json!({}));
    match HostingService::update_limits(&state, &id, body).await {
        Ok(limits) => {
            audit_site(&state, "site.hosting_limits_updated", &id, metadata).await;
            ApiResponse::new_serialized(limits).ok()
        }
        Err(err) => ApiResponse::error(&format!("failed to update limits: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/stats",
    operation_id = "get_sites_stats",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(SiteStats))),
)]
pub async fn site_stats(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    match SiteStatsService::collect(&state, &id).await {
        Ok(stats) => ApiResponse::new_serialized(stats).ok(),
        Err(err) => ApiResponse::error(&format!("failed to collect site stats: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}/files",
    operation_id = "get_sites_files",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("path" = Option<String>, Query)),
    responses((status = OK, body = inline(Vec<FileEntry>))),
)]
pub async fn files_list(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<FilePathQuery>,
) -> ApiResponseResult {
    let path = query.path.unwrap_or_default();
    match FileService::list(&state, &id, &path).await {
        Ok(entries) => ApiResponse::new_serialized(entries).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list files: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FilePathQuery {
    pub path: Option<String>,
}

#[utoipa::path(
    get,
    path = "/{id}/files/download",
    operation_id = "get_sites_files_download",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("path" = String, Query)),
    responses((status = OK, description = "File bytes")),
)]
pub async fn files_download(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<FilePathQuery>,
) -> ApiResponseResult {
    let path = query.path.unwrap_or_default();
    if path.is_empty() {
        return ApiResponse::error("path query parameter required")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    }
    match FileService::download(&state, &id, &path).await {
        Ok(bytes) => ApiResponse::new(axum::body::Body::from(bytes))
            .with_header("content-type", "application/octet-stream")
            .ok(),
        Err(err) => ApiResponse::error(&format!("download failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    put,
    path = "/{id}/files",
    operation_id = "put_sites_files",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("path" = String, Query)),
    request_body(content = String, description = "Raw file bytes"),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn files_upload(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<FilePathQuery>,
    body: axum::body::Bytes,
) -> ApiResponseResult {
    let path = query.path.unwrap_or_default();
    if path.is_empty() {
        return ApiResponse::error("path query parameter required")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    }
    match FileService::upload(&state, &id, &path, &body).await {
        Ok(()) => {
            audit_file(&state, "site.file_uploaded", &id, &path).await;
            ApiResponse::new_serialized(ActionResponse { id: path, ok: true }).ok()
        }
        Err(err) if is_disk_quota_error(&err) => ApiResponse::error(&format!("{err:#}"))
            .with_status(StatusCode::PAYLOAD_TOO_LARGE)
            .ok(),
        Err(err) => ApiResponse::error(&format!("upload failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    delete,
    path = "/{id}/files",
    operation_id = "delete_sites_files",
    security(("bearer_auth" = [])),
    params(("id" = String, Path), ("path" = String, Query)),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn files_delete(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<FilePathQuery>,
) -> ApiResponseResult {
    let path = query.path.unwrap_or_default();
    match FileService::delete(&state, &id, &path).await {
        Ok(()) => {
            audit_file(&state, "site.file_deleted", &id, &path).await;
            ApiResponse::new_serialized(ActionResponse { id: path, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("delete failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/files/mkdir",
    operation_id = "post_sites_files_mkdir",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(FilePathQuery),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn files_mkdir(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<FilePathQuery>,
) -> ApiResponseResult {
    let path = body.path.unwrap_or_default();
    match FileService::mkdir(&state, &id, &path).await {
        Ok(()) => {
            audit_file(&state, "site.file_mkdir", &id, &path).await;
            ApiResponse::new_serialized(ActionResponse { id: path, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("mkdir failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/files/rename",
    operation_id = "post_sites_files_rename",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(RenameRequest),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn files_rename(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<RenameRequest>,
) -> ApiResponseResult {
    let from = body.from.clone();
    let to = body.to.clone();
    match FileService::rename(&state, &id, body.clone()).await {
        Ok(()) => {
            audit_file_move(&state, "site.file_renamed", &id, &from, &to).await;
            ApiResponse::new_serialized(ActionResponse { id: from, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("rename failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/files/move",
    operation_id = "post_sites_files_move",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(MoveCopyRequest),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn files_move(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<MoveCopyRequest>,
) -> ApiResponseResult {
    let from = body.from.clone();
    let to = body.to.clone();
    match FileService::r#move(&state, &id, body.clone()).await {
        Ok(()) => {
            audit_file_move(&state, "site.file_moved", &id, &from, &to).await;
            ApiResponse::new_serialized(ActionResponse { id: from, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("move failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/files/copy",
    operation_id = "post_sites_files_copy",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(MoveCopyRequest),
    responses((status = OK, body = inline(ActionResponse))),
)]
pub async fn files_copy(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<MoveCopyRequest>,
) -> ApiResponseResult {
    let from = body.from.clone();
    let to = body.to.clone();
    match FileService::copy(&state, &id, body.clone()).await {
        Ok(()) => {
            audit_file_move(&state, "site.file_copied", &id, &from, &to).await;
            ApiResponse::new_serialized(ActionResponse { id: from, ok: true }).ok()
        }
        Err(err) => ApiResponse::error(&format!("copy failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/files/archive",
    operation_id = "post_sites_files_archive",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    request_body = inline(FilePathQuery),
    responses((status = OK, body = inline(ArchiveResponse))),
)]
pub async fn files_archive(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<FilePathQuery>,
) -> ApiResponseResult {
    let path = body.path.unwrap_or_default();
    match FileService::archive(&state, &id, &path).await {
        Ok(archive) => {
            audit_site(
                &state,
                "site.file_archived",
                &id,
                serde_json::json!({ "path": &path, "archive": &archive }),
            )
            .await;
            ApiResponse::new_serialized(ArchiveResponse { archive }).ok()
        }
        Err(err) => ApiResponse::error(&format!("archive failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

async fn audit_site(state: &GetState, action: &str, site_id: &str, metadata: serde_json::Value) {
    crate::utils::audit::record(
        state,
        AuditEvent::action(action)
            .with_resource("site", site_id)
            .with_metadata(metadata),
    )
    .await;
}

async fn audit_file(state: &GetState, action: &str, site_id: &str, path: &str) {
    audit_site(
        state,
        action,
        site_id,
        serde_json::json!({
            "path": path,
        }),
    )
    .await;
}

async fn audit_file_move(state: &GetState, action: &str, site_id: &str, from: &str, to: &str) {
    audit_site(
        state,
        action,
        site_id,
        serde_json::json!({
            "from": from,
            "to": to,
        }),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cache::{AuditEventRecord, Cache, SiteRecord},
        routes::{AppContainerType, AppState, State},
    };
    use std::{collections::HashMap, sync::Arc, time::Instant};

    async fn test_state() -> (State, tempfile::TempDir, tempfile::TempDir) {
        let data_dir = tempfile::tempdir().unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        let config = crate::config::Config::for_openapi_docs("FeatherFly");
        let mut inner = config.load().as_ref().clone();
        inner.system.data = data_dir.path().join("data").display().to_string();
        inner.system.backup_directory = data_dir.path().join("backups").display().to_string();
        inner.system.archive_directory = data_dir.path().join("archives").display().to_string();
        inner.hosting.shared_services = false;
        config.inner.store(Arc::new(inner));

        let cache = Cache::open(cache_dir.path().to_str().unwrap())
            .await
            .unwrap();
        let inner = config.load();
        let jwt = Arc::new(crate::remote::jwt::JwtClient::new(
            &inner.token,
            inner.api.max_jwt_uses,
        ));
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            container_type: AppContainerType::None,
            version: crate::full_version(),
            config,
            plugins: crate::plugins::PluginRegistry::empty(),
            probe_guard: crate::middlewares::probe::ProbeGuard::new(),
            docker: None,
            panel: arc_swap::ArcSwapOption::from(None),
            cache: Some(Arc::new(cache)),
            jwt,
        });

        (state, data_dir, cache_dir)
    }

    async fn seed_site(state: &State, id: &str) {
        state
            .cache
            .as_ref()
            .unwrap()
            .save_site(&SiteRecord {
                id: id.into(),
                name: "Demo".into(),
                domain: "demo.example.test".into(),
                egg_id: "generic-web".into(),
                container_id: None,
                network: Some("site-demo-net".into()),
                volume: Some("site-demo-vol".into()),
                git_repo: None,
                memory_mb: None,
                cpu_quota: None,
                disk_quota_mb: None,
                bandwidth_mbps: None,
                status: crate::sites::SITE_STATUS_ACTIVE.into(),
                domains: Vec::new(),
                document_root: None,
                site_type: crate::sites::SiteType::Static.as_str().into(),
                variables: HashMap::new(),
                redirects: Vec::new(),
                preview_domain: None,
                wildcard_domain: false,
                created_at: 1,
            })
            .await
            .unwrap();
    }

    fn event<'a>(events: &'a [AuditEventRecord], action: &str) -> &'a AuditEventRecord {
        events
            .iter()
            .find(|event| event.action == action)
            .unwrap_or_else(|| panic!("missing audit action {action}"))
    }

    fn metadata(event: &AuditEventRecord) -> serde_json::Value {
        serde_json::from_str(event.metadata_json.as_deref().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn account_controller_actions_write_audit_rows() {
        let (state, _data_dir, _cache_dir) = test_state().await;
        seed_site(&state, "demo").await;

        let mail_response = mail_accounts_create(
            axum::extract::State(Arc::clone(&state)),
            axum::extract::Path("demo".to_string()),
            axum::Json(CreateMailAccountRequest {
                id: "mail-main".into(),
                address: "hello@demo.example.test".into(),
                password: "secret".into(),
                quota_mb: Some(512),
            }),
        )
        .await;
        assert!(mail_response.is_ok());

        let ftp_response = ftp_accounts_create(
            axum::extract::State(Arc::clone(&state)),
            axum::extract::Path("demo".to_string()),
            axum::Json(CreateFtpAccountRequest {
                id: "ftp-main".into(),
                username: "deploy".into(),
                password: "secret".into(),
                protocol: crate::ftp::FtpProtocol::Sftp,
                home_subpath: "public".into(),
            }),
        )
        .await;
        assert!(ftp_response.is_ok());

        let events = state
            .cache
            .as_ref()
            .unwrap()
            .list_audit_events(10)
            .await
            .unwrap();

        let mail = event(&events, "site.mail_account_created");
        assert_eq!(mail.resource_type.as_deref(), Some("site"));
        assert_eq!(mail.resource_id.as_deref(), Some("demo"));
        assert_eq!(mail.success, 1);
        assert_eq!(metadata(mail)["address"], "hello@demo.example.test");

        let ftp = event(&events, "site.ftp_account_created");
        assert_eq!(ftp.resource_type.as_deref(), Some("site"));
        assert_eq!(ftp.resource_id.as_deref(), Some("demo"));
        assert_eq!(ftp.success, 1);
        assert_eq!(metadata(ftp)["username"], "deploy");
    }

    #[tokio::test]
    async fn site_audit_helpers_write_file_backup_and_database_rows() {
        let (state, _data_dir, _cache_dir) = test_state().await;
        let get_state = axum::extract::State(Arc::clone(&state));

        audit_file(
            &get_state,
            "site.file_uploaded",
            "demo",
            "public/index.html",
        )
        .await;
        audit_file_move(
            &get_state,
            "site.file_moved",
            "demo",
            "public/index.html",
            "public/app/index.html",
        )
        .await;
        audit_site(
            &get_state,
            "site.backup_created",
            "demo",
            serde_json::json!({
                "backup_id": "backup-1",
                "path": "/tmp/demo.tar.gz",
            }),
        )
        .await;
        audit_site(
            &get_state,
            "site.database_created",
            "demo",
            serde_json::json!({
                "database_id": "main",
                "engine": "mysql",
                "host": "db-demo-main",
                "port": 3306,
            }),
        )
        .await;

        let events = state
            .cache
            .as_ref()
            .unwrap()
            .list_audit_events(10)
            .await
            .unwrap();

        let file = event(&events, "site.file_uploaded");
        assert_eq!(file.resource_id.as_deref(), Some("demo"));
        assert_eq!(metadata(file)["path"], "public/index.html");

        let moved = event(&events, "site.file_moved");
        assert_eq!(metadata(moved)["from"], "public/index.html");
        assert_eq!(metadata(moved)["to"], "public/app/index.html");

        let backup = event(&events, "site.backup_created");
        assert_eq!(backup.resource_type.as_deref(), Some("site"));
        assert_eq!(metadata(backup)["backup_id"], "backup-1");

        let database = event(&events, "site.database_created");
        assert_eq!(database.success, 1);
        assert_eq!(metadata(database)["database_id"], "main");
        assert_eq!(metadata(database)["engine"], "mysql");
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArchiveResponse {
    pub archive: String,
}
