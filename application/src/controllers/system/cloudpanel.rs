use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

use axum::{Json, body::Body, extract::Path, http::StatusCode};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::{
    plugins::PluginRegistry,
    routes::GetState,
    utils::{
        plugin_events::{self, CloudPanelCommandPayload},
        response::{ApiResponse, ApiResponseResult},
    },
};

const DEFAULT_CLPCTL: &str = "clpctl";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetResponse {
    binary: String,
    compatibility: &'static str,
    routes: Vec<RouteSpec>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteSpec {
    method: &'static str,
    path: &'static str,
    command: &'static str,
    summary: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliArg {
    name: String,
    value: Option<String>,
    sensitive: bool,
}

struct CommandEvent<'a> {
    phase: CommandPhase,
    operation: &'a str,
    command: &'a str,
    args: &'a [CliArg],
    status: Option<i32>,
    duration_ms: u64,
    error: Option<&'a str>,
    hook_handlers: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandPhase {
    Requested,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloudPanelLifecycle {
    SiteCreate,
    SiteDelete,
    DatabaseCreate,
    DatabaseDelete,
    DatabaseExport,
    UserPasswordReset,
    UserMfaDisable,
    CertificateInstall,
    VhostTemplatesImport,
}

impl CloudPanelLifecycle {
    fn from_operation(operation: &str) -> Option<Self> {
        match operation {
            "add_static_site"
            | "add_nodejs_site"
            | "add_python_site"
            | "add_reverse_proxy_site"
            | "add_php_site" => Some(Self::SiteCreate),
            "delete_site" => Some(Self::SiteDelete),
            "add_database" => Some(Self::DatabaseCreate),
            "delete_database" => Some(Self::DatabaseDelete),
            "export_database" => Some(Self::DatabaseExport),
            "reset_user_password" => Some(Self::UserPasswordReset),
            "disable_user_mfa" => Some(Self::UserMfaDisable),
            "install_lets_encrypt_certificate" => Some(Self::CertificateInstall),
            "import_vhost_templates" => Some(Self::VhostTemplatesImport),
            _ => None,
        }
    }

    fn site_type(operation: &str) -> Option<&'static str> {
        match operation {
            "add_static_site" => Some("static"),
            "add_nodejs_site" => Some("nodejs"),
            "add_python_site" => Some("python"),
            "add_reverse_proxy_site" => Some("reverse_proxy"),
            "add_php_site" => Some("php"),
            _ => None,
        }
    }

    fn event(self, phase: CommandPhase) -> featherfly_plugin_sdk::PluginEvent {
        use featherfly_plugin_sdk::PluginEvent as E;
        match (self, phase) {
            (Self::SiteCreate, CommandPhase::Requested) => E::CloudPanelSiteCreateRequested,
            (Self::SiteCreate, CommandPhase::Succeeded) => E::CloudPanelSiteCreated,
            (Self::SiteCreate, CommandPhase::Failed) => E::CloudPanelSiteCreateFailed,
            (Self::SiteDelete, CommandPhase::Requested) => E::CloudPanelSiteDeleteRequested,
            (Self::SiteDelete, CommandPhase::Succeeded) => E::CloudPanelSiteDeleted,
            (Self::SiteDelete, CommandPhase::Failed) => E::CloudPanelSiteDeleteFailed,
            (Self::DatabaseCreate, CommandPhase::Requested) => E::CloudPanelDatabaseCreateRequested,
            (Self::DatabaseCreate, CommandPhase::Succeeded) => E::CloudPanelDatabaseCreated,
            (Self::DatabaseCreate, CommandPhase::Failed) => E::CloudPanelDatabaseCreateFailed,
            (Self::DatabaseDelete, CommandPhase::Requested) => E::CloudPanelDatabaseDeleteRequested,
            (Self::DatabaseDelete, CommandPhase::Succeeded) => E::CloudPanelDatabaseDeleted,
            (Self::DatabaseDelete, CommandPhase::Failed) => E::CloudPanelDatabaseDeleteFailed,
            (Self::DatabaseExport, CommandPhase::Requested) => E::CloudPanelDatabaseExportRequested,
            (Self::DatabaseExport, CommandPhase::Succeeded) => E::CloudPanelDatabaseExported,
            (Self::DatabaseExport, CommandPhase::Failed) => E::CloudPanelDatabaseExportFailed,
            (Self::UserPasswordReset, CommandPhase::Requested) => {
                E::CloudPanelUserPasswordResetRequested
            }
            (Self::UserPasswordReset, CommandPhase::Succeeded) => E::CloudPanelUserPasswordReset,
            (Self::UserPasswordReset, CommandPhase::Failed) => E::CloudPanelUserPasswordResetFailed,
            (Self::UserMfaDisable, CommandPhase::Requested) => E::CloudPanelUserMfaDisableRequested,
            (Self::UserMfaDisable, CommandPhase::Succeeded) => E::CloudPanelUserMfaDisabled,
            (Self::UserMfaDisable, CommandPhase::Failed) => E::CloudPanelUserMfaDisableFailed,
            (Self::CertificateInstall, CommandPhase::Requested) => {
                E::CloudPanelCertificateInstallRequested
            }
            (Self::CertificateInstall, CommandPhase::Succeeded) => {
                E::CloudPanelCertificateInstalled
            }
            (Self::CertificateInstall, CommandPhase::Failed) => {
                E::CloudPanelCertificateInstallFailed
            }
            (Self::VhostTemplatesImport, CommandPhase::Requested) => {
                E::CloudPanelVhostTemplatesImportRequested
            }
            (Self::VhostTemplatesImport, CommandPhase::Succeeded) => {
                E::CloudPanelVhostTemplatesImported
            }
            (Self::VhostTemplatesImport, CommandPhase::Failed) => {
                E::CloudPanelVhostTemplatesImportFailed
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterCredentials {
    host: String,
    port: u16,
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VhostTemplate {
    name: String,
    root_directory: String,
    #[serde(rename = "type")]
    template_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VhostTemplateView {
    name: String,
    template: String,
}

macro_rules! arg {
    ($name:literal, $value:expr) => {
        CliArg::value($name, $value, false)
    };
}

macro_rules! secret_arg {
    ($name:literal, $value:expr) => {
        CliArg::value($name, $value, true)
    };
}

pub async fn get(_state: GetState) -> ApiResponseResult {
    ApiResponse::new_serialized(GetResponse {
        binary: clpctl_binary(),
        compatibility: "CloudPanelApi",
        routes: route_specs(),
    })
    .ok()
}

pub async fn add_static_site(
    state: GetState,
    Json(request): Json<AddStaticSiteRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "add_static_site",
        "site:add:static",
        vec![
            arg!("domainName", request.domain_name),
            arg!("siteUser", request.site_user),
            secret_arg!("siteUserPassword", request.site_user_password),
        ],
    )
    .await
}

pub async fn add_nodejs_site(
    state: GetState,
    Json(request): Json<AddNodejsSiteRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "add_nodejs_site",
        "site:add:nodejs",
        vec![
            arg!("domainName", request.domain_name),
            arg!("siteUser", request.site_user),
            secret_arg!("siteUserPassword", request.site_user_password),
            arg!("nodejsVersion", request.nodejs_version),
            arg!("appPort", request.app_port),
        ],
    )
    .await
}

pub async fn add_python_site(
    state: GetState,
    Json(request): Json<AddPythonSiteRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "add_python_site",
        "site:add:python",
        vec![
            arg!("domainName", request.domain_name),
            arg!("siteUser", request.site_user),
            secret_arg!("siteUserPassword", request.site_user_password),
            arg!("pythonVersion", request.python_version),
            arg!("appPort", request.app_port),
        ],
    )
    .await
}

pub async fn add_reverse_proxy_site(
    state: GetState,
    Json(request): Json<AddReverseProxySiteRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "add_reverse_proxy_site",
        "site:add:reverse-proxy",
        vec![
            arg!("domainName", request.domain_name),
            arg!("siteUser", request.site_user),
            secret_arg!("siteUserPassword", request.site_user_password),
            arg!("reverseProxyUrl", request.reverse_proxy_url),
        ],
    )
    .await
}

pub async fn add_php_site(
    state: GetState,
    Json(request): Json<AddPhpSiteRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "add_php_site",
        "site:add:php",
        vec![
            arg!("domainName", request.domain_name),
            arg!("siteUser", request.site_user),
            secret_arg!("siteUserPassword", request.site_user_password),
            arg!("phpVersion", request.php_version),
            arg!("vhostTemplate", request.vhost_template),
        ],
    )
    .await
}

pub async fn delete_site(state: GetState, Path(domain_name): Path<String>) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "delete_site",
        "site:delete",
        vec![arg!("domainName", domain_name), CliArg::flag("force")],
    )
    .await
}

pub async fn get_master_credentials(state: GetState) -> ApiResponseResult {
    let output = execute(
        &state.plugins,
        "get_master_credentials",
        "db:show:master-credentials",
        Vec::new(),
    )
    .await?;
    let credentials = match parse_master_credentials(&output) {
        Ok(credentials) => credentials,
        Err(err) => return Err(ApiResponse::error(&err).with_status(StatusCode::BAD_GATEWAY)),
    };

    ApiResponse::new_serialized(credentials).ok()
}

pub async fn add_database(
    state: GetState,
    Json(request): Json<AddDatabaseRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "add_database",
        "db:add",
        vec![
            arg!("domainName", request.domain_name),
            arg!("databaseName", request.database_name),
            arg!("databaseUserName", request.database_user_name),
            secret_arg!("databaseUserPassword", request.database_user_password),
        ],
    )
    .await
}

pub async fn delete_database(
    state: GetState,
    Path(database_name): Path<String>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "delete_database",
        "db:delete",
        vec![arg!("databaseName", database_name), CliArg::flag("force")],
    )
    .await
}

pub async fn reset_user_password(
    state: GetState,
    Path(username): Path<String>,
    Json(request): Json<ResetPasswordRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "reset_user_password",
        "user:reset:password",
        vec![
            arg!("userName", username),
            secret_arg!("password", request.password),
        ],
    )
    .await
}

pub async fn disable_user_mfa(state: GetState, Path(username): Path<String>) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "disable_user_mfa",
        "user:disable:mfa",
        vec![arg!("userName", username)],
    )
    .await
}

pub async fn install_lets_encrypt_certificate(
    state: GetState,
    Json(request): Json<InstallCertificateRequest>,
) -> ApiResponseResult {
    let mut args = vec![arg!("domainName", request.domain_name)];
    if !request.subject_alternative_name.is_empty() {
        args.push(arg!(
            "subjectAlternativeName",
            request.subject_alternative_name.join(",")
        ));
    }

    execute_no_content(
        &state.plugins,
        "install_lets_encrypt_certificate",
        "lets-encrypt:install:certificate",
        args,
    )
    .await
}

pub async fn get_vhost_templates(state: GetState) -> ApiResponseResult {
    let output = execute(
        &state.plugins,
        "get_vhost_templates",
        "vhost-templates:list",
        Vec::new(),
    )
    .await?;
    let templates = parse_vhost_templates(&output);

    ApiResponse::new_serialized(templates).ok()
}

pub async fn import_vhost_templates(state: GetState) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "import_vhost_templates",
        "vhost-templates:import",
        Vec::new(),
    )
    .await
}

pub async fn list_users(state: GetState) -> ApiResponseResult {
    let output = execute(&state.plugins, "list_users", "user:list", Vec::new()).await?;
    ApiResponse::new_serialized(parse_table_records(&output)).ok()
}

pub async fn export_database(
    state: GetState,
    Path(database_name): Path<String>,
    Json(request): Json<ExportDatabaseRequest>,
) -> ApiResponseResult {
    execute_no_content(
        &state.plugins,
        "export_database",
        "db:export",
        vec![
            arg!("databaseName", database_name),
            arg!("file", request.file),
        ],
    )
    .await
}

pub async fn view_vhost_template(state: GetState, Path(name): Path<String>) -> ApiResponseResult {
    let output = execute(
        &state.plugins,
        "view_vhost_template",
        "vhost-template:view",
        vec![arg!("name", name.clone())],
    )
    .await?;

    ApiResponse::new_serialized(VhostTemplateView {
        name,
        template: output,
    })
    .ok()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddStaticSiteRequest {
    domain_name: String,
    site_user: String,
    site_user_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNodejsSiteRequest {
    domain_name: String,
    site_user: String,
    site_user_password: String,
    nodejs_version: u16,
    app_port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPythonSiteRequest {
    domain_name: String,
    site_user: String,
    site_user_password: String,
    python_version: String,
    app_port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddReverseProxySiteRequest {
    domain_name: String,
    site_user: String,
    site_user_password: String,
    reverse_proxy_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPhpSiteRequest {
    domain_name: String,
    site_user: String,
    site_user_password: String,
    vhost_template: String,
    php_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDatabaseRequest {
    domain_name: String,
    database_name: String,
    database_user_name: String,
    database_user_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallCertificateRequest {
    domain_name: String,
    #[serde(default)]
    subject_alternative_name: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDatabaseRequest {
    file: String,
}

impl CliArg {
    fn value(name: impl Into<String>, value: impl ToString, sensitive: bool) -> Self {
        Self {
            name: name.into(),
            value: Some(value.to_string()),
            sensitive,
        }
    }

    fn flag(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
            sensitive: false,
        }
    }

    fn raw(&self) -> Result<String, String> {
        let Some(value) = &self.value else {
            return Ok(format!("--{}", self.name));
        };

        validate_value(&self.name, value)?;
        Ok(format!("--{}={}", self.name, value))
    }

    fn redacted(&self) -> String {
        match (&self.value, self.sensitive) {
            (Some(_), true) => format!("--{}=********", self.name),
            (Some(value), false) => format!("--{}={}", self.name, value),
            (None, _) => format!("--{}", self.name),
        }
    }
}

async fn execute_no_content(
    plugins: &PluginRegistry,
    operation: &'static str,
    command: &'static str,
    args: Vec<CliArg>,
) -> ApiResponseResult {
    execute(plugins, operation, command, args).await?;
    ApiResponse::new(Body::empty())
        .with_status(StatusCode::NO_CONTENT)
        .ok()
}

async fn execute(
    plugins: &PluginRegistry,
    operation: &'static str,
    command: &'static str,
    args: Vec<CliArg>,
) -> Result<String, ApiResponse> {
    let start = Instant::now();
    let mut args = args;

    emit_command_event(
        plugins,
        CommandEvent {
            phase: CommandPhase::Requested,
            operation,
            command,
            args: &args,
            status: None,
            duration_ms: 0,
            error: None,
            hook_handlers: 0,
        },
    );

    let hook_input = match serde_json::to_vec(&args) {
        Ok(input) => input,
        Err(err) => {
            tracing::error!(operation, command, error = ?err, "failed to serialize CloudPanel args");
            return Err(ApiResponse::error("failed to serialize CloudPanel args")
                .with_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };
    let hook_outcome = plugins.run_cloudpanel_hooks(operation, command, &hook_input);
    if hook_outcome.cancelled {
        let reason = if hook_outcome.cancel_reason.trim().is_empty() {
            "CloudPanel command cancelled by plugin"
        } else {
            hook_outcome.cancel_reason.trim()
        };
        emit_command_event(
            plugins,
            CommandEvent {
                phase: CommandPhase::Failed,
                operation,
                command,
                args: &args,
                status: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(reason),
                hook_handlers: hook_outcome.handlers_run,
            },
        );
        return Err(ApiResponse::error(reason).with_status(StatusCode::BAD_REQUEST));
    }
    if hook_outcome.mutated {
        args = normalize_hook_args(&args, &hook_outcome.args_json)?;
    }

    let raw_args = match args.iter().map(CliArg::raw).collect::<Result<Vec<_>, _>>() {
        Ok(args) => args,
        Err(err) => return Err(ApiResponse::error(&err).with_status(StatusCode::BAD_REQUEST)),
    };

    tracing::info!(
        command,
        argv = ?redacted_argv(command, &args),
        "running CloudPanel CLI command"
    );

    let output = match Command::new(clpctl_binary())
        .arg(command)
        .args(&raw_args)
        .output()
        .await
    {
        Ok(output) => output,
        Err(err) => {
            tracing::error!(command, error = ?err, "failed to run CloudPanel CLI");
            let message = "failed to run CloudPanel CLI";
            emit_command_event(
                plugins,
                CommandEvent {
                    phase: CommandPhase::Failed,
                    operation,
                    command,
                    args: &args,
                    status: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(message),
                    hook_handlers: hook_outcome.handlers_run,
                },
            );
            return Err(ApiResponse::error("failed to run CloudPanel CLI")
                .with_status(StatusCode::SERVICE_UNAVAILABLE));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let duration_ms = start.elapsed().as_millis() as u64;

    if output.status.success() {
        emit_command_event(
            plugins,
            CommandEvent {
                phase: CommandPhase::Succeeded,
                operation,
                command,
                args: &args,
                status: output.status.code(),
                duration_ms,
                error: None,
                hook_handlers: hook_outcome.handlers_run,
            },
        );
        return Ok(stdout);
    }

    let message = cloudpanel_error_message(&stdout, &stderr);
    emit_command_event(
        plugins,
        CommandEvent {
            phase: CommandPhase::Failed,
            operation,
            command,
            args: &args,
            status: output.status.code(),
            duration_ms,
            error: Some(&message),
            hook_handlers: hook_outcome.handlers_run,
        },
    );
    Err(ApiResponse::error(&message).with_status(StatusCode::BAD_REQUEST))
}

fn normalize_hook_args(original: &[CliArg], args_json: &[u8]) -> Result<Vec<CliArg>, ApiResponse> {
    let mut args: Vec<CliArg> = serde_json::from_slice(args_json).map_err(|err| {
        tracing::warn!(error = ?err, "CloudPanel plugin returned invalid args JSON");
        ApiResponse::error("CloudPanel plugin returned invalid args JSON")
            .with_status(StatusCode::BAD_REQUEST)
    })?;
    let allowed: BTreeMap<String, bool> = original
        .iter()
        .map(|arg| (arg.name.clone(), arg.sensitive))
        .collect();
    let mut seen = BTreeSet::new();

    for arg in &mut args {
        let Some(sensitive) = allowed.get(&arg.name) else {
            return Err(
                ApiResponse::error("CloudPanel plugin returned unsupported argument")
                    .with_status(StatusCode::BAD_REQUEST),
            );
        };
        if !seen.insert(arg.name.clone()) {
            return Err(
                ApiResponse::error("CloudPanel plugin returned duplicate argument")
                    .with_status(StatusCode::BAD_REQUEST),
            );
        }
        arg.sensitive = *sensitive;
    }

    Ok(args)
}

fn emit_command_event(plugins: &PluginRegistry, event: CommandEvent<'_>) {
    let Some(lifecycle) = CloudPanelLifecycle::from_operation(event.operation) else {
        return;
    };

    plugin_events::emit_json(
        plugins,
        lifecycle.event(event.phase),
        &CloudPanelCommandPayload {
            operation: event.operation,
            command: event.command,
            args: redacted_argv(event.command, event.args),
            site_type: CloudPanelLifecycle::site_type(event.operation),
            status: event.status,
            duration_ms: event.duration_ms,
            error: event.error,
            hook_handlers: event.hook_handlers,
        },
    );
}

fn parse_master_credentials(output: &str) -> Result<MasterCredentials, String> {
    let values = parse_key_value_table(output);

    let host = required_value(&values, "Host")?.to_string();
    let port = required_value(&values, "Port")?
        .parse::<u16>()
        .map_err(|_| "CloudPanel returned an invalid database port".to_string())?;
    let username = required_value(&values, "User Name")?.to_string();
    let password = required_value(&values, "Password")?.to_string();

    Ok(MasterCredentials {
        host,
        port,
        username,
        password,
    })
}

fn parse_vhost_templates(output: &str) -> Vec<VhostTemplate> {
    output
        .lines()
        .filter_map(|line| {
            let parts = table_parts(line);
            if parts.len() < 3 || parts[0] == "Name" {
                return None;
            }

            Some(VhostTemplate {
                name: parts[0].to_string(),
                root_directory: parts[1].to_string(),
                template_type: parts[2].to_string(),
            })
        })
        .collect()
}

fn parse_key_value_table(output: &str) -> BTreeMap<String, String> {
    output
        .lines()
        .filter_map(|line| {
            let parts = table_parts(line);
            if parts.len() < 2 {
                return None;
            }

            Some((parts[0].to_string(), parts[1].to_string()))
        })
        .collect()
}

fn parse_table_records(output: &str) -> Vec<BTreeMap<String, String>> {
    let mut headers: Vec<String> = Vec::new();
    let mut records = Vec::new();

    for line in output.lines() {
        let parts = table_parts(line);
        if parts.len() < 2 {
            continue;
        }

        if headers.is_empty() {
            headers = parts.iter().map(|part| (*part).to_string()).collect();
            continue;
        }

        if parts.len() != headers.len()
            || parts == headers.iter().map(String::as_str).collect::<Vec<_>>()
        {
            continue;
        }

        records.push(
            headers
                .iter()
                .cloned()
                .zip(parts.iter().map(|part| (*part).to_string()))
                .collect(),
        );
    }

    records
}

fn table_parts(line: &str) -> Vec<&str> {
    line.split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn required_value<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("CloudPanel response did not include {key}"))
}

fn validate_value(name: &str, value: &str) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!(
            "argument {name} contains unsupported control characters"
        ));
    }

    Ok(())
}

fn cloudpanel_error_message(stdout: &str, stderr: &str) -> String {
    let message = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };

    message
        .trim_start_matches('"')
        .replace("\"\n", "")
        .trim()
        .to_string()
}

fn redacted_argv(command: &str, args: &[CliArg]) -> Vec<String> {
    let mut argv = Vec::with_capacity(args.len() + 2);
    argv.push(clpctl_binary());
    argv.push(command.to_string());
    argv.extend(args.iter().map(CliArg::redacted));
    argv
}

fn clpctl_binary() -> String {
    std::env::var("FEATHERFLY_CLPCTL").unwrap_or_else(|_| DEFAULT_CLPCTL.to_string())
}

fn route_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/site/static",
            command: "site:add:static",
            summary: "Add a static site.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/site/nodejs",
            command: "site:add:nodejs",
            summary: "Add a Node.js site.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/site/python",
            command: "site:add:python",
            summary: "Add a Python site.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/site/reverseproxy",
            command: "site:add:reverse-proxy",
            summary: "Add a reverse proxy site.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/site/php",
            command: "site:add:php",
            summary: "Add a PHP site.",
        },
        RouteSpec {
            method: "DELETE",
            path: "/api/system/cloudpanel/site/{domainName}",
            command: "site:delete --force",
            summary: "Delete a site.",
        },
        RouteSpec {
            method: "GET",
            path: "/api/system/cloudpanel/db/master-credentials",
            command: "db:show:master-credentials",
            summary: "Get database master credentials.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/db",
            command: "db:add",
            summary: "Add a database.",
        },
        RouteSpec {
            method: "DELETE",
            path: "/api/system/cloudpanel/db/{databaseName}",
            command: "db:delete --force",
            summary: "Delete a database.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/db/{databaseName}/export",
            command: "db:export",
            summary: "Export a database dump.",
        },
        RouteSpec {
            method: "GET",
            path: "/api/system/cloudpanel/user",
            command: "user:list",
            summary: "List users.",
        },
        RouteSpec {
            method: "PUT",
            path: "/api/system/cloudpanel/user/{username}/resetpassword",
            command: "user:reset:password",
            summary: "Reset a user password.",
        },
        RouteSpec {
            method: "PUT",
            path: "/api/system/cloudpanel/user/{username}/mfa",
            command: "user:disable:mfa",
            summary: "Disable user MFA.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/letsencrypt/install/certificate",
            command: "lets-encrypt:install:certificate",
            summary: "Install a Let's Encrypt certificate.",
        },
        RouteSpec {
            method: "GET",
            path: "/api/system/cloudpanel/vhosttemplates",
            command: "vhost-templates:list",
            summary: "List vhost templates.",
        },
        RouteSpec {
            method: "POST",
            path: "/api/system/cloudpanel/vhosttemplates",
            command: "vhost-templates:import",
            summary: "Import vhost templates.",
        },
        RouteSpec {
            method: "GET",
            path: "/api/system/cloudpanel/vhosttemplate/{name}",
            command: "vhost-template:view",
            summary: "View a vhost template.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_specs_match_cloudpanel_api_surface() {
        let routes = route_specs();

        assert_eq!(routes.len(), 17);
        assert!(routes.iter().any(|route| {
            route.method == "DELETE" && route.path == "/api/system/cloudpanel/site/{domainName}"
        }));
        assert!(routes.iter().any(|route| {
            route.method == "PUT"
                && route.path == "/api/system/cloudpanel/user/{username}/resetpassword"
        }));
    }

    #[test]
    fn cli_args_support_flags_and_secret_redaction() {
        let args = [
            arg!("domainName", "example.com"),
            secret_arg!("siteUserPassword", "secret"),
            CliArg::flag("force"),
        ];

        let raw = args
            .iter()
            .map(CliArg::raw)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let redacted = args.iter().map(CliArg::redacted).collect::<Vec<_>>();

        assert_eq!(
            raw,
            vec![
                "--domainName=example.com",
                "--siteUserPassword=secret",
                "--force"
            ]
        );
        assert_eq!(
            redacted,
            vec![
                "--domainName=example.com",
                "--siteUserPassword=********",
                "--force"
            ]
        );
    }

    #[test]
    fn parses_master_credentials_table() {
        let output = r#"
+-----------+----------------+
| Key       | Value          |
+-----------+----------------+
| Host      | 127.0.0.1      |
| Port      | 3306           |
| User Name | root           |
| Password  | secret         |
+-----------+----------------+
"#;

        let credentials = parse_master_credentials(output).unwrap();

        assert_eq!(credentials.host, "127.0.0.1");
        assert_eq!(credentials.port, 3306);
        assert_eq!(credentials.username, "root");
        assert_eq!(credentials.password, "secret");
    }

    #[test]
    fn parses_vhost_template_table() {
        let output = r#"
+--------------+----------------+------+
| Name         | Root Directory | Type |
+--------------+----------------+------+
| Generic      | htdocs         | php  |
+--------------+----------------+------+
"#;

        let templates = parse_vhost_templates(output);

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Generic");
        assert_eq!(templates[0].root_directory, "htdocs");
        assert_eq!(templates[0].template_type, "php");
    }

    #[test]
    fn lifecycle_events_map_write_operations() {
        use featherfly_plugin_sdk::PluginEvent;

        let lifecycle = CloudPanelLifecycle::from_operation("delete_site").unwrap();
        assert_eq!(
            lifecycle.event(CommandPhase::Requested),
            PluginEvent::CloudPanelSiteDeleteRequested
        );
        assert_eq!(
            lifecycle.event(CommandPhase::Succeeded),
            PluginEvent::CloudPanelSiteDeleted
        );
        assert_eq!(
            lifecycle.event(CommandPhase::Failed),
            PluginEvent::CloudPanelSiteDeleteFailed
        );

        assert_eq!(CloudPanelLifecycle::site_type("add_php_site"), Some("php"));
        assert!(CloudPanelLifecycle::from_operation("list_users").is_none());
    }
}
