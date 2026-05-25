use crate::{
    backups,
    cache::BackupRecord,
    controllers::docker::require_docker,
    databases::{CreateDatabaseRequest, DatabaseService},
    dns::{DnsService, DnsZoneStatus, SyncDnsRequest},
    files::{FileEntry, FileService, MoveCopyRequest, RenameRequest},
    ftp::{CreateFtpAccountRequest, FtpService, FtpSyncRequest},
    hosting::{HostingOverview, HostingService, SiteStats, SiteStatsService, UpdateLimitsRequest},
    mail::{CreateMailAccountRequest, MailService, MailSyncRequest},
    routes::GetState,
    scheduler::{CreateTaskRequest, SchedulerService},
    sites::{DeployRequest, ProvisionSiteRequest, SiteExecRequest, SiteService},
    utils::{
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
        Ok(site) => ApiResponse::new_serialized(site)
            .with_status(StatusCode::CREATED)
            .ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok(),
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
    match SiteService::deploy(&state, &id, body).await {
        Ok(()) => ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse { id, ok: true }).ok(),
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
            plugin_events::emit_state_event(
                &state,
                PluginEvent::BackupCompleted,
                &BackupCompletedPayload {
                    site_id: &id,
                    backup_id: &result.id,
                },
            );
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

#[utoipa::path(
    get,
    path = "/templates",
    operation_id = "get_site_templates",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(Vec<String>))),
)]
pub async fn templates_list(state: GetState) -> ApiResponseResult {
    let dir = state.config.load().hosting.templates_directory.clone();
    match crate::templates::list_templates(&dir) {
        Ok(names) => ApiResponse::new_serialized(names).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list templates: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(ToSchema, Deserialize)]
pub struct SiteLogsQuery {
    pub tail: Option<String>,
    pub grep: Option<String>,
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
        Ok(db) => ApiResponse::new_serialized(db)
            .with_status(StatusCode::CREATED)
            .ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: db_id,
            ok: true,
        })
        .ok(),
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
        Ok(task) => ApiResponse::new_serialized(task)
            .with_status(StatusCode::CREATED)
            .ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: task_id,
            ok: true,
        })
        .ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: task_id,
            ok: true,
        })
        .ok(),
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
    match DnsService::sync(&state, &id, body).await {
        Ok(records) => ApiResponse::new_serialized(records).ok(),
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
        Ok(account) => ApiResponse::new_serialized(account)
            .with_status(StatusCode::CREATED)
            .ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: account_id,
            ok: true,
        })
        .ok(),
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
    match MailService::sync(&state, &id, body).await {
        Ok(result) => ApiResponse::new_serialized(result).ok(),
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
        Ok(account) => ApiResponse::new_serialized(account)
            .with_status(StatusCode::CREATED)
            .ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: account_id,
            ok: true,
        })
        .ok(),
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
    match FtpService::sync(&state, &id, body).await {
        Ok(result) => ApiResponse::new_serialized(result).ok(),
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
    match SiteService::exec(&state, &id, body).await {
        Ok(result) => ApiResponse::new_serialized(result).ok(),
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
    match HostingService::update_limits(&state, &id, body).await {
        Ok(limits) => ApiResponse::new_serialized(limits).ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse { id: path, ok: true }).ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse { id: path, ok: true }).ok(),
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
        Ok(()) => ApiResponse::new_serialized(ActionResponse { id: path, ok: true }).ok(),
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
    match FileService::rename(&state, &id, body.clone()).await {
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: body.from,
            ok: true,
        })
        .ok(),
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
    match FileService::r#move(&state, &id, body.clone()).await {
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: body.from,
            ok: true,
        })
        .ok(),
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
    match FileService::copy(&state, &id, body.clone()).await {
        Ok(()) => ApiResponse::new_serialized(ActionResponse {
            id: body.from,
            ok: true,
        })
        .ok(),
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
        Ok(archive) => ApiResponse::new_serialized(ArchiveResponse { archive }).ok(),
        Err(err) => ApiResponse::error(&format!("archive failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArchiveResponse {
    pub archive: String,
}
