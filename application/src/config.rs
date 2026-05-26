use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use serde_default::DefaultFromSerde;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogRotation {
    #[default]
    Daily,
    Hourly,
    Never,
}

impl LogRotation {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Hourly => "hourly",
            Self::Never => "never",
        }
    }

    #[must_use]
    pub fn as_tracing_rotation(self) -> tracing_appender::rolling::Rotation {
        match self {
            Self::Daily => tracing_appender::rolling::Rotation::DAILY,
            Self::Hourly => tracing_appender::rolling::Rotation::HOURLY,
            Self::Never => tracing_appender::rolling::Rotation::NEVER,
        }
    }
}

fn logging_enabled_default() -> bool {
    true
}

fn log_to_stdout_default() -> bool {
    true
}

fn log_level_default() -> String {
    "info".into()
}

fn latest_file_default() -> String {
    "latest.log".into()
}

fn archive_prefix_default() -> String {
    "featherfly".into()
}

fn archive_suffix_default() -> String {
    "log".into()
}

fn archive_max_files_default() -> usize {
    30
}

fn timestamp_format_default() -> String {
    "%Y-%m-%d %H:%M:%S %z".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct LogFormatConfig {
    #[serde(default = "timestamp_format_default")]
    pub timestamp: String,

    #[serde(default)]
    pub include_target: bool,

    #[serde(default)]
    pub include_file_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct LogArchiveConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "archive_prefix_default")]
    pub prefix: String,

    #[serde(default = "archive_suffix_default")]
    pub suffix: String,

    #[serde(default)]
    pub rotation: LogRotation,

    #[serde(default = "archive_max_files_default")]
    pub max_files: usize,

    #[serde(default)]
    pub max_age_days: u32,

    #[serde(default = "default_true")]
    pub prune_on_startup: bool,
}

impl LogArchiveConfig {
    pub fn looks_like_archive(&self, filename: &str) -> bool {
        filename.starts_with(&format!("{}.", self.prefix))
            && filename.ends_with(&format!(".{}", self.suffix))
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct LoggingConfig {
    #[serde(default = "logging_enabled_default")]
    pub enabled: bool,

    #[serde(default = "log_to_stdout_default")]
    pub log_to_stdout: bool,

    #[serde(default = "log_level_default")]
    pub level: String,

    #[serde(default = "latest_file_default")]
    pub latest_file: String,

    #[serde(default)]
    pub archive: LogArchiveConfig,

    #[serde(default)]
    pub format: LogFormatConfig,
}

fn app_name() -> String {
    "FeatherFly".to_string()
}
fn api_host() -> String {
    "0.0.0.0".to_string()
}
fn api_port() -> u16 {
    9090
}

fn system_root_directory() -> String {
    "/var/lib/featherfly".to_string()
}
fn system_log_directory() -> String {
    "/var/log/featherfly".to_string()
}
fn system_tmp_directory() -> String {
    "/tmp/featherfly".to_string()
}
fn system_username() -> String {
    "featherfly".to_string()
}
fn system_pid_file() -> String {
    "/var/run/featherfly/daemon.pid".to_string()
}
fn system_plugins_directory() -> String {
    "/var/lib/featherfly/plugins".to_string()
}

fn node_id_default() -> String {
    String::new()
}

fn token_id_default() -> String {
    String::new()
}

fn docker_socket_default() -> String {
    "/var/run/docker.sock".to_string()
}

fn system_data_directory() -> String {
    "/var/lib/featherfly/volumes".to_string()
}

fn system_archive_directory() -> String {
    "/var/lib/featherfly/archives".to_string()
}

fn system_backup_directory() -> String {
    "/var/lib/featherfly/backups".to_string()
}

fn system_timezone_default() -> String {
    "UTC".to_string()
}

fn docker_network_interface_default() -> String {
    "172.18.0.1".into()
}

fn docker_network_name_default() -> String {
    "featherfly_nw".into()
}

fn docker_network_mode_default() -> String {
    "featherfly_nw".into()
}

fn docker_network_driver_default() -> String {
    "bridge".into()
}

fn docker_network_subnet_default() -> String {
    "172.18.0.0/16".into()
}

fn docker_tmpfs_size_default() -> u32 {
    100
}

fn docker_container_pid_limit_default() -> i64 {
    512
}

fn remote_query_timeout_default() -> u32 {
    30
}

fn remote_query_boot_servers_default() -> u32 {
    50
}

fn remote_query_retry_limit_default() -> u32 {
    10
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct RemoteQueryConfig {
    #[serde(default = "remote_query_timeout_default")]
    pub timeout: u32,

    #[serde(default = "remote_query_boot_servers_default")]
    pub boot_servers_per_page: u32,

    /// Max retry attempts for panel HTTP calls (exponential backoff, Wings-compatible).
    /// Set to 0 to use elapsed-time cap (~30s) instead of a fixed attempt count.
    #[serde(default = "remote_query_retry_limit_default")]
    pub retry_limit: u32,

    /// Extra query parameters appended to every panel HTTP request.
    #[serde(default)]
    pub query: HashMap<String, String>,

    /// Custom headers sent on panel HTTP and websocket requests (e.g. Cloudflare Access).
    #[serde(default)]
    pub custom_headers: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct DockerNetworkConfig {
    #[serde(default = "docker_network_interface_default")]
    pub interface: String,

    #[serde(default = "docker_network_name_default")]
    pub name: String,

    #[serde(default = "docker_network_mode_default")]
    pub network_mode: String,

    #[serde(default = "docker_network_driver_default")]
    pub driver: String,

    #[serde(default = "docker_network_subnet_default")]
    pub subnet: String,

    #[serde(default = "default_true")]
    pub enable_icc: bool,

    #[serde(default)]
    pub is_internal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct DockerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "docker_socket_default")]
    pub socket: String,

    #[serde(default)]
    pub network: DockerNetworkConfig,

    #[serde(default = "docker_tmpfs_size_default")]
    pub tmpfs_size: u32,

    #[serde(default = "docker_container_pid_limit_default")]
    pub container_pid_limit: i64,

    /// When true, install Docker via get.docker.com and enable systemd if missing at boot.
    #[serde(default = "default_true")]
    pub auto_install: bool,
}

fn proxy_provider_default() -> String {
    "traefik".into()
}

fn proxy_cert_resolver_default() -> String {
    "letsencrypt".into()
}

fn proxy_entrypoint_http_default() -> String {
    "web".into()
}

fn proxy_entrypoint_https_default() -> String {
    "websecure".into()
}

fn proxy_health_check_path_default() -> Option<String> {
    Some("/".into())
}

fn proxy_health_check_interval_default() -> String {
    "30s".into()
}

fn proxy_traefik_image_default() -> String {
    "traefik:v3.3".into()
}

fn proxy_publish_http_port_default() -> u16 {
    80
}

fn proxy_publish_https_port_default() -> u16 {
    443
}

fn hosting_eggs_directory_default() -> String {
    "/var/lib/featherfly/eggs".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct ProxyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "proxy_provider_default")]
    pub provider: String,

    #[serde(default = "proxy_cert_resolver_default")]
    pub cert_resolver: String,

    #[serde(default = "proxy_entrypoint_http_default")]
    pub entrypoint_http: String,

    #[serde(default = "proxy_entrypoint_https_default")]
    pub entrypoint_https: String,

    #[serde(default = "docker_network_name_default")]
    pub network: String,

    #[serde(default = "default_true")]
    pub tls_enabled: bool,

    #[serde(default = "default_true")]
    pub redirect_http_to_https: bool,

    #[serde(default = "default_true")]
    pub compress_responses: bool,

    #[serde(default = "default_true")]
    pub security_headers: bool,

    #[serde(default = "proxy_health_check_path_default")]
    pub health_check_path: Option<String>,

    #[serde(default = "proxy_health_check_interval_default")]
    pub health_check_interval: String,

    #[serde(default = "default_true")]
    pub include_www_alias: bool,

    #[serde(default = "default_true")]
    pub auto_provision: bool,

    #[serde(default = "proxy_traefik_image_default")]
    pub traefik_image: String,

    /// Let's Encrypt contact email. Leave empty for HTTP-only routing until you set this.
    #[serde(default)]
    pub acme_email: String,

    #[serde(default)]
    pub dashboard_enabled: bool,

    #[serde(default = "proxy_publish_http_port_default")]
    pub publish_http_port: u16,

    #[serde(default = "proxy_publish_https_port_default")]
    pub publish_https_port: u16,
}

impl ProxyConfig {
    /// True when TLS certificates can be issued (email configured).
    #[must_use]
    pub fn acme_ready(&self) -> bool {
        self.enabled && self.tls_enabled && !self.acme_email.trim().is_empty()
    }

    /// Site routers should terminate TLS when ACME is configured.
    #[must_use]
    pub fn routes_use_tls(&self) -> bool {
        self.enabled && self.provider == "traefik" && self.acme_ready()
    }

    /// Global/per-site HTTP→HTTPS redirect is only safe once certificates can be issued.
    #[must_use]
    pub fn routes_redirect_http(&self) -> bool {
        self.redirect_http_to_https && self.acme_ready()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct HostingConfig {
    #[serde(
        default = "hosting_eggs_directory_default",
        alias = "templates_directory"
    )]
    pub eggs_directory: String,

    #[serde(default = "default_true")]
    pub auto_pull_images: bool,

    #[serde(default = "default_true")]
    pub attach_proxy_network: bool,

    #[serde(default = "hosting_mailserver_image_default")]
    pub mailserver_image: String,

    #[serde(default = "hosting_roundcube_image_default")]
    pub roundcube_image: String,

    #[serde(default = "hosting_sftp_image_default")]
    pub sftp_image: String,

    #[serde(default = "hosting_ftp_image_default")]
    pub ftp_image: String,

    #[serde(default = "hosting_mail_subdomain_default")]
    pub mail_subdomain: String,

    #[serde(default = "hosting_webmail_subdomain_default")]
    pub webmail_subdomain: String,

    #[serde(default = "hosting_phpmyadmin_subdomain_default")]
    pub phpmyadmin_subdomain: String,

    #[serde(default = "hosting_phpmyadmin_image_default")]
    pub phpmyadmin_image: String,

    #[serde(default = "hosting_mysql_image_default")]
    pub mysql_image: String,

    #[serde(default = "hosting_postgres_image_default")]
    pub postgres_image: String,

    #[serde(default = "hosting_redis_image_default")]
    pub redis_image: String,

    #[serde(default = "hosting_mongodb_image_default")]
    pub mongodb_image: String,

    /// Root password for shared MySQL/Postgres/Redis/MongoDB (auto-generated if empty).
    #[serde(default)]
    pub database_root_password: String,

    #[serde(default)]
    pub ports: HostingPortsConfig,

    /// One shared mail/SFTP/FTP/Roundcube/database stack for all sites (Plesk-like).
    #[serde(default = "default_true")]
    pub shared_services: bool,

    /// Pull latest images and recreate stack containers on startup reconcile.
    #[serde(default = "default_true")]
    pub auto_update_images: bool,

    /// On boot: start site containers and sync mail/FTP/shared stack.
    #[serde(default = "default_true")]
    pub reconcile_on_startup: bool,

    /// Optional custom 404/500/502/503 HTML templates (filenames: 404.html, etc.).
    #[serde(default = "hosting_error_pages_directory_default")]
    pub error_pages_directory: String,

    /// Archived site log files retained per site (`data/sites/{id}/logs/`).
    #[serde(default = "hosting_site_log_retention_default")]
    pub site_log_retention: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct HostingPortsConfig {
    #[serde(default = "hosting_port_http_default")]
    pub http: u16,
    #[serde(default = "hosting_port_https_default")]
    pub https: u16,
    #[serde(default = "hosting_port_sftp_default")]
    pub sftp: u16,
    #[serde(default = "hosting_port_ftp_default")]
    pub ftp: u16,
    #[serde(default = "hosting_port_ftp_passive_min_default")]
    pub ftp_passive_min: u16,
    #[serde(default = "hosting_port_ftp_passive_max_default")]
    pub ftp_passive_max: u16,
    #[serde(default = "hosting_port_smtp_default")]
    pub smtp: u16,
    #[serde(default = "hosting_port_submission_default")]
    pub submission: u16,
    #[serde(default = "hosting_port_smtps_default")]
    pub smtps: u16,
    #[serde(default = "hosting_port_imap_default")]
    pub imap: u16,
    #[serde(default = "hosting_port_imaps_default")]
    pub imaps: u16,
    #[serde(default = "hosting_port_pop3_default")]
    pub pop3: u16,
    #[serde(default = "hosting_port_pop3s_default")]
    pub pop3s: u16,
    #[serde(default = "hosting_port_mysql_default")]
    pub mysql: u16,
    #[serde(default = "hosting_port_postgres_default")]
    pub postgres: u16,
    #[serde(default = "hosting_port_redis_default")]
    pub redis: u16,
    #[serde(default = "hosting_port_mongodb_default")]
    pub mongodb: u16,
    #[serde(default = "default_true")]
    pub publish_service_ports: bool,
}

impl Default for HostingPortsConfig {
    fn default() -> Self {
        Self {
            http: 80,
            https: 443,
            sftp: 2222,
            ftp: 21,
            ftp_passive_min: 40000,
            ftp_passive_max: 40100,
            smtp: 25,
            submission: 587,
            smtps: 465,
            imap: 143,
            imaps: 993,
            pop3: 110,
            pop3s: 995,
            mysql: 3306,
            postgres: 5432,
            redis: 6379,
            mongodb: 27017,
            publish_service_ports: true,
        }
    }
}

fn hosting_port_http_default() -> u16 {
    80
}
fn hosting_port_https_default() -> u16 {
    443
}
fn hosting_port_sftp_default() -> u16 {
    2222
}
fn hosting_port_ftp_default() -> u16 {
    21
}
fn hosting_port_ftp_passive_min_default() -> u16 {
    40000
}
fn hosting_port_ftp_passive_max_default() -> u16 {
    40100
}
fn hosting_port_smtp_default() -> u16 {
    25
}
fn hosting_port_submission_default() -> u16 {
    587
}
fn hosting_port_smtps_default() -> u16 {
    465
}
fn hosting_port_imap_default() -> u16 {
    143
}
fn hosting_port_imaps_default() -> u16 {
    993
}
fn hosting_port_pop3_default() -> u16 {
    110
}
fn hosting_port_pop3s_default() -> u16 {
    995
}
fn hosting_port_mysql_default() -> u16 {
    3306
}
fn hosting_port_postgres_default() -> u16 {
    5432
}
fn hosting_port_redis_default() -> u16 {
    6379
}
fn hosting_port_mongodb_default() -> u16 {
    27017
}

fn hosting_mailserver_image_default() -> String {
    "mailserver/docker-mailserver:latest".into()
}

fn hosting_roundcube_image_default() -> String {
    "roundcube/roundcubemail:latest".into()
}

fn hosting_sftp_image_default() -> String {
    "atmoz/sftp:latest".into()
}

fn hosting_ftp_image_default() -> String {
    "stilliard/pure-ftpd:hardened".into()
}

fn hosting_mail_subdomain_default() -> String {
    "mail".into()
}

fn hosting_webmail_subdomain_default() -> String {
    "webmail".into()
}

fn hosting_phpmyadmin_subdomain_default() -> String {
    "phpmyadmin".into()
}

fn hosting_phpmyadmin_image_default() -> String {
    "phpmyadmin:latest".into()
}

fn hosting_mysql_image_default() -> String {
    "mysql:8".into()
}

fn hosting_postgres_image_default() -> String {
    "postgres:16-alpine".into()
}

fn hosting_redis_image_default() -> String {
    "redis:7-alpine".into()
}

fn hosting_mongodb_image_default() -> String {
    "mongo:7".into()
}

fn hosting_site_log_retention_default() -> u32 {
    30
}

fn hosting_error_pages_directory_default() -> String {
    "/var/lib/featherfly/error-pages".into()
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct InnerConfig {
    #[serde(default = "app_name")]
    pub app_name: String,

    #[serde(default)]
    pub debug: bool,

    /// Node UUID — auto-generated on first boot if empty (Wings-compatible).
    #[serde(default = "node_id_default")]
    pub uuid: String,

    /// Panel authentication token ID — auto-generated if empty.
    #[serde(default = "token_id_default")]
    pub token_id: String,

    /// Daemon API / panel authentication token — auto-generated if empty.
    #[serde(default)]
    pub token: String,

    /// FeatherPanel base URL (Wings `remote`).
    #[serde(default)]
    pub remote: String,

    #[serde(default)]
    pub remote_query: RemoteQueryConfig,

    #[serde(default)]
    pub allowed_mounts: Vec<String>,

    #[serde(default)]
    pub allowed_origins: Vec<String>,

    #[serde(default)]
    pub api: ApiConfig,

    #[serde(default)]
    pub system: SystemConfig,

    #[serde(default)]
    pub updates: UpdatesConfig,

    #[serde(default)]
    pub plugins: PluginsConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub management: ManagementConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub docker: DockerConfig,

    #[serde(default)]
    pub proxy: ProxyConfig,

    #[serde(default)]
    pub hosting: HostingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct SecurityConfig {
    #[serde(default)]
    pub probe_protection: ProbeProtectionConfig,
}

fn probe_max_404s_default() -> u32 {
    25
}

fn probe_window_secs_default() -> u32 {
    60
}

fn probe_block_secs_default() -> u32 {
    900
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, DefaultFromSerde)]
pub struct ProbeProtectionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "probe_max_404s_default")]
    pub max_404s: u32,

    #[serde(default = "probe_window_secs_default")]
    pub window_secs: u32,

    #[serde(default = "probe_block_secs_default")]
    pub block_secs: u32,

    #[serde(default = "default_true")]
    pub log_requests: bool,

    #[serde(default = "default_true")]
    pub log_unknown_routes: bool,

    #[serde(default = "default_true")]
    pub log_blocked: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct ManagementConfig {
    #[serde(default = "default_true")]
    pub config_edit: bool,

    #[serde(default = "default_true")]
    pub restart: bool,

    #[serde(default = "default_true")]
    pub upgrade: bool,
}

pub struct ConfigApplyResult {
    pub requires_restart: bool,
    pub restart_reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct ApiConfig {
    #[serde(default = "api_host")]
    pub host: String,

    #[serde(default = "api_port")]
    pub port: u16,

    #[serde(default)]
    pub disable_openapi_docs: bool,

    /// Max uses per JWT `unique_id` for download/upload tokens (0 = unlimited). Wings-compatible.
    #[serde(default = "api_max_jwt_uses_default")]
    pub max_jwt_uses: usize,
}

fn api_max_jwt_uses_default() -> usize {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct SystemConfig {
    #[serde(default = "system_root_directory")]
    pub root_directory: String,

    #[serde(default = "system_log_directory")]
    pub log_directory: String,

    #[serde(default = "system_tmp_directory")]
    pub tmp_directory: String,

    #[serde(default = "system_username")]
    pub username: String,

    #[serde(default = "system_pid_file")]
    pub pid_file: String,

    #[serde(default = "system_plugins_directory")]
    pub plugins_directory: String,

    #[serde(default = "system_data_directory")]
    pub data: String,

    #[serde(default = "system_archive_directory")]
    pub archive_directory: String,

    #[serde(default = "system_backup_directory")]
    pub backup_directory: String,

    #[serde(default = "system_timezone_default")]
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct PluginsConfig {
    #[serde(default = "plugins_enabled_default")]
    pub enabled: bool,
}

fn plugins_enabled_default() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct UpdatesConfig {
    #[serde(default = "updates_channel_default")]
    pub channel: crate::utils::update::UpdateChannel,

    #[serde(default = "updates_check_on_startup_default")]
    pub check_on_startup: bool,

    #[serde(default)]
    pub ignore_panel_upgrades: bool,
}

fn updates_channel_default() -> crate::utils::update::UpdateChannel {
    crate::utils::update::UpdateChannel::Stable
}

fn updates_check_on_startup_default() -> bool {
    true
}

pub struct Config {
    pub inner: ArcSwap<InnerConfig>,
    pub path: String,
}

pub struct ConfigGuard(#[allow(dead_code)] crate::utils::logging::LogGuards);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ConfigOpenMeta {
    pub identity_generated: bool,
    pub config_upgraded: bool,
}

impl std::ops::Deref for Config {
    type Target = ArcSwap<InnerConfig>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Config {
    pub fn load(&self) -> arc_swap::Guard<Arc<InnerConfig>> {
        self.inner.load()
    }

    pub fn resolve_config_path(
        debug: bool,
        explicit: Option<&str>,
        from_command_line: bool,
    ) -> Result<String, anyhow::Error> {
        const DEFAULT_CONFIG_PATH: &str = "/etc/featherfly/config.yml";
        const DEBUG_CONFIG_NAME: &str = "config.yml";

        if from_command_line {
            return Ok(explicit.unwrap_or(DEFAULT_CONFIG_PATH).to_string());
        }

        if debug {
            let path = std::env::current_dir()
                .context("failed to determine current working directory")?
                .join(DEBUG_CONFIG_NAME);

            return Ok(path.to_string_lossy().into_owned());
        }

        Ok(DEFAULT_CONFIG_PATH.to_string())
    }

    pub fn open(
        path: &str,
        debug: bool,
        is_subcommand: bool,
    ) -> Result<(Arc<Self>, ConfigGuard, ConfigOpenMeta), anyhow::Error> {
        let raw = std::fs::read(path).context(format!("failed to read config file {path}"))?;
        let inner = Self::parse_preview(&raw)?;
        Self::open_from_inner(inner, path, debug, is_subcommand)
    }

    pub fn parse_preview(bytes: &[u8]) -> Result<InnerConfig, anyhow::Error> {
        let value: serde_norway::Value =
            serde_norway::from_slice(bytes).context("failed to parse config yaml")?;
        let value = migrate_legacy_config(value);
        let inner: InnerConfig =
            serde_norway::from_value(value).context("failed to parse config yaml")?;
        Ok(inner)
    }

    pub fn apply_debug_overrides(mut inner: InnerConfig, debug: bool) -> InnerConfig {
        if debug {
            inner.debug = true;
        }
        inner
    }

    pub fn ensure_directories(inner: &InnerConfig) -> Result<(), anyhow::Error> {
        for dir in [
            inner.system.root_directory.as_str(),
            inner.system.log_directory.as_str(),
            inner.system.tmp_directory.as_str(),
            inner.system.plugins_directory.as_str(),
            inner.system.data.as_str(),
            inner.system.archive_directory.as_str(),
            inner.system.backup_directory.as_str(),
            inner.hosting.eggs_directory.as_str(),
        ] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory {dir}"))?;
        }

        if let Some(parent) = PathBuf::from(&inner.system.pid_file).parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create pid directory {}", parent.display()))?;
        }

        Ok(())
    }

    pub fn init_tracing(
        inner: &InnerConfig,
        _debug: bool,
        is_subcommand: bool,
    ) -> Result<ConfigGuard, anyhow::Error> {
        crate::utils::logging::init(inner, is_subcommand).map(ConfigGuard)
    }

    pub fn open_from_inner(
        mut inner: InnerConfig,
        path: &str,
        debug: bool,
        is_subcommand: bool,
    ) -> Result<(Arc<Self>, ConfigGuard, ConfigOpenMeta), anyhow::Error> {
        inner = Self::apply_debug_overrides(inner, debug);

        let identity_changed = crate::utils::identity::ensure_node_identity(
            &mut inner.uuid,
            &mut inner.token_id,
            &mut inner.token,
        );

        if !is_subcommand {
            Self::ensure_directories(&inner)?;
        }

        let log_guards = Self::init_tracing(&inner, debug, is_subcommand)?;

        Self::validate_inner(&inner)?;

        let dev_mode = inner.debug || is_subcommand;

        if !dev_mode {
            Self::write_pid_file(&inner)?;
        }

        let config = Arc::new(Self {
            inner: ArcSwap::new(Arc::new(inner)),
            path: path.to_string(),
        });

        let config_upgraded = if !is_subcommand {
            match std::fs::read(path) {
                Ok(raw) => Self::config_needs_persist(&raw, config.load().as_ref())?,
                Err(_) => false,
            }
        } else {
            false
        };

        if (identity_changed || config_upgraded) && !is_subcommand {
            config.persist_inner()?;
            if identity_changed {
                tracing::info!(
                    uuid = %config.load().uuid,
                    token_id = %config.load().token_id,
                    "generated node identity and saved to config"
                );
            }
            if config_upgraded {
                tracing::info!(path, "updated config file with new settings");
            }
        }

        Ok((
            config,
            log_guards,
            ConfigOpenMeta {
                identity_generated: identity_changed,
                config_upgraded,
            },
        ))
    }

    /// True when the on-disk YAML is missing keys or needs a legacy migration rewrite.
    fn config_needs_persist(raw: &[u8], inner: &InnerConfig) -> Result<bool, anyhow::Error> {
        let original: serde_norway::Value =
            serde_norway::from_slice(raw).context("failed to parse config yaml")?;
        let migrated = migrate_legacy_config(original.clone());
        if migrated != original {
            return Ok(true);
        }

        let complete: serde_norway::Value =
            serde_norway::to_value(inner).context("failed to serialize config")?;
        Ok(!value_contains_all_keys(&migrated, &complete))
    }

    pub fn persist_inner(&self) -> Result<(), anyhow::Error> {
        let yaml =
            serde_norway::to_string(self.load().as_ref()).context("failed to serialize config")?;
        std::fs::write(&self.path, yaml)
            .with_context(|| format!("failed to write config file {}", self.path))?;
        Ok(())
    }

    fn validate_inner(inner: &InnerConfig) -> Result<(), anyhow::Error> {
        if inner.api.port == 0 {
            anyhow::bail!("api.port must be greater than 0");
        }

        if inner.system.root_directory.is_empty() {
            anyhow::bail!("system.root_directory must not be empty");
        }

        if inner.logging.latest_file.is_empty() {
            anyhow::bail!("logging.latest_file must not be empty");
        }

        if inner.logging.archive.prefix.is_empty() {
            anyhow::bail!("logging.archive.prefix must not be empty");
        }

        Ok(())
    }

    fn write_pid_file(inner: &InnerConfig) -> Result<(), anyhow::Error> {
        std::fs::write(&inner.system.pid_file, std::process::id().to_string())
            .with_context(|| format!("failed to write pid file {}", inner.system.pid_file))
    }

    pub fn remove_pid_file(inner: &InnerConfig) {
        if let Err(error) = std::fs::remove_file(&inner.system.pid_file) {
            tracing::warn!(
                path = %inner.system.pid_file,
                %error,
                "failed to remove pid file"
            );
        }
    }

    pub fn export_yaml_redacted(&self) -> Result<String, anyhow::Error> {
        let guard = self.load();
        let mut inner = guard.as_ref().clone();
        if !inner.token.is_empty() {
            inner.token = "***".into();
        }
        if !inner.token_id.is_empty() {
            inner.token_id = "***".into();
        }
        serde_norway::to_string(&inner).context("failed to serialize config")
    }

    pub fn apply_yaml(&self, yaml: &str) -> Result<ConfigApplyResult, anyhow::Error> {
        let mut new_inner: InnerConfig =
            serde_norway::from_str(yaml).context("failed to parse config yaml")?;

        let old = self.load();
        if new_inner.token == "***" {
            new_inner.token = old.token.clone();
        }
        if new_inner.token_id == "***" {
            new_inner.token_id = old.token_id.clone();
        }
        if new_inner.uuid.is_empty() {
            new_inner.uuid = old.uuid.clone();
        }

        Self::validate_inner(&new_inner)?;
        let restart_reasons = restart_reasons(&old, &new_inner);
        let yaml = serde_norway::to_string(&new_inner).context("failed to serialize config")?;

        std::fs::write(&self.path, yaml)
            .with_context(|| format!("failed to write config file {}", self.path))?;
        self.inner.store(Arc::new(new_inner));

        Ok(ConfigApplyResult {
            requires_restart: !restart_reasons.is_empty(),
            restart_reasons,
        })
    }

    pub fn for_openapi_docs(app_name: impl Into<String>) -> Arc<Self> {
        let inner = InnerConfig {
            app_name: app_name.into(),
            debug: true,
            uuid: "00000000-0000-0000-0000-000000000000".into(),
            token_id: "docs-token-id".into(),
            token: "docs".into(),
            remote: String::new(),
            remote_query: RemoteQueryConfig::default(),
            allowed_mounts: Vec::new(),
            allowed_origins: Vec::new(),
            api: ApiConfig {
                host: "127.0.0.1".into(),
                port: 8080,
                disable_openapi_docs: false,
                max_jwt_uses: 5,
            },
            system: SystemConfig {
                root_directory: "./data".into(),
                log_directory: "./logs".into(),
                tmp_directory: "./tmp".into(),
                username: "featherfly".into(),
                pid_file: "./featherfly.pid".into(),
                plugins_directory: "./data/plugins".into(),
                data: "./data/volumes".into(),
                archive_directory: "./data/archives".into(),
                backup_directory: "./data/backups".into(),
                timezone: "UTC".into(),
            },
            updates: UpdatesConfig::default(),
            plugins: PluginsConfig::default(),
            logging: LoggingConfig::default(),
            management: ManagementConfig::default(),
            security: SecurityConfig::default(),
            docker: DockerConfig::default(),
            proxy: ProxyConfig::default(),
            hosting: HostingConfig::default(),
        };

        Arc::new(Self {
            inner: ArcSwap::new(Arc::new(inner)),
            path: "docs".into(),
        })
    }
}

fn value_contains_all_keys(disk: &serde_norway::Value, complete: &serde_norway::Value) -> bool {
    match (disk, complete) {
        (serde_norway::Value::Mapping(disk_map), serde_norway::Value::Mapping(complete_map)) => {
            for (key, complete_value) in complete_map {
                match disk_map.get(key) {
                    None => return false,
                    Some(disk_value) => {
                        if !value_contains_all_keys(disk_value, complete_value) {
                            return false;
                        }
                    }
                }
            }
            true
        }
        _ => true,
    }
}

fn migrate_legacy_config(mut value: serde_norway::Value) -> serde_norway::Value {
    let Some(root) = value.as_mapping_mut() else {
        return value;
    };

    if let Some(remote) = root.remove("remote") {
        match remote {
            serde_norway::Value::String(url) => {
                root.insert("remote".into(), serde_norway::Value::String(url));
            }
            serde_norway::Value::Mapping(map) => {
                root.insert("remote".into(), serde_norway::Value::String(String::new()));
                let mut management = root
                    .remove("management")
                    .and_then(|v| match v {
                        serde_norway::Value::Mapping(map) => Some(map),
                        _ => None,
                    })
                    .unwrap_or_default();
                for key in ["config_edit", "restart", "upgrade"] {
                    if let Some(v) = map.get(key) {
                        management.insert(key.into(), v.clone());
                    }
                }
                if !management.is_empty() {
                    root.insert(
                        "management".into(),
                        serde_norway::Value::Mapping(management),
                    );
                }
            }
            other => {
                root.insert("remote".into(), other);
            }
        }
    }

    if let Some(panel) = root.remove("panel")
        && let Some(mapping) = panel.as_mapping()
        && root
            .get("remote")
            .and_then(|v| v.as_str())
            .is_none_or(str::is_empty)
        && let Some(url) = mapping.get("url").and_then(|v| v.as_str())
    {
        root.insert("remote".into(), serde_norway::Value::String(url.into()));
    }

    if let Some(node) = root.remove("node")
        && let Some(mapping) = node.as_mapping()
        && let Some(id) = mapping.get("id").and_then(|v| v.as_str())
        && !id.is_empty()
        && root
            .get("uuid")
            .and_then(|v| v.as_str())
            .is_none_or(str::is_empty)
    {
        root.insert("uuid".into(), serde_norway::Value::String(id.into()));
    }

    value
}

fn restart_reasons(old: &InnerConfig, new: &InnerConfig) -> Vec<String> {
    let mut reasons = Vec::new();

    if old.api.host != new.api.host || old.api.port != new.api.port {
        reasons.push("api listen address changed".into());
    }
    if old.debug != new.debug {
        reasons.push("debug flag changed".into());
    }
    if old.logging != new.logging {
        reasons.push("logging settings changed".into());
    }
    if old.system.plugins_directory != new.system.plugins_directory {
        reasons.push("plugins directory changed".into());
    }
    if old.plugins.enabled != new.plugins.enabled {
        reasons.push("plugins.enabled changed".into());
    }
    if old.system.log_directory != new.system.log_directory {
        reasons.push("log directory changed".into());
    }
    if old.docker != new.docker {
        reasons.push("docker settings changed".into());
    }
    if old.remote != new.remote {
        reasons.push("panel remote URL changed".into());
    }
    if old.remote_query != new.remote_query {
        reasons.push("remote_query settings changed".into());
    }
    if old.uuid != new.uuid {
        reasons.push("node uuid changed".into());
    }

    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn isolated_system_yaml(base: &Path) -> String {
        let base = base.to_string_lossy();
        format!(
            r"
system:
  root_directory: {base}/data
  log_directory: {base}/logs
  tmp_directory: {base}/tmp
  plugins_directory: {base}/plugins
  data: {base}/volumes
  archive_directory: {base}/archives
  backup_directory: {base}/backups
  pid_file: {base}/featherfly.pid
"
        )
    }

    fn minimal_open_yaml(base: &Path, hosting_extra: &str) -> String {
        let eggs = base.to_string_lossy();
        format!(
            r"debug: true
api:
  port: 9090
{system}
hosting:
  eggs_directory: {eggs}/eggs
{hosting_extra}",
            system = isolated_system_yaml(base)
        )
    }

    #[test]
    fn resolve_config_path_debug_uses_local_config() {
        let path = Config::resolve_config_path(true, None, false).unwrap();
        assert!(path.ends_with("config.yml"));
    }

    #[test]
    fn resolve_config_path_production_uses_etc() {
        let path = Config::resolve_config_path(false, None, false).unwrap();
        assert_eq!(path, "/etc/featherfly/config.yml");
    }

    #[test]
    fn resolve_config_path_explicit_flag_wins() {
        let path = Config::resolve_config_path(false, Some("/custom/config.yml"), true).unwrap();
        assert_eq!(path, "/custom/config.yml");
    }

    #[test]
    fn validate_inner_rejects_zero_port() {
        let inner = InnerConfig {
            app_name: "FeatherFly".into(),
            debug: false,
            uuid: String::new(),
            token_id: String::new(),
            token: String::new(),
            remote: String::new(),
            remote_query: RemoteQueryConfig::default(),
            allowed_mounts: Vec::new(),
            allowed_origins: Vec::new(),
            api: ApiConfig {
                host: "127.0.0.1".into(),
                port: 0,
                disable_openapi_docs: false,
                max_jwt_uses: 5,
            },
            system: SystemConfig {
                root_directory: "/tmp/featherfly-test".into(),
                log_directory: "/tmp/featherfly-test/logs".into(),
                tmp_directory: "/tmp/featherfly-test/tmp".into(),
                username: "featherfly".into(),
                pid_file: "/tmp/featherfly-test/featherfly.pid".into(),
                plugins_directory: "/tmp/featherfly-test/plugins".into(),
                data: "/tmp/featherfly-test/volumes".into(),
                archive_directory: "/tmp/featherfly-test/archives".into(),
                backup_directory: "/tmp/featherfly-test/backups".into(),
                timezone: "UTC".into(),
            },
            updates: UpdatesConfig::default(),
            plugins: PluginsConfig::default(),
            logging: LoggingConfig::default(),
            management: ManagementConfig::default(),
            security: SecurityConfig::default(),
            docker: DockerConfig::default(),
            proxy: ProxyConfig::default(),
            hosting: HostingConfig::default(),
        };

        let error = Config::validate_inner(&inner).unwrap_err();
        assert!(error.to_string().contains("api.port"));
    }

    #[test]
    fn loads_config_from_yaml_file() {
        let inner = Config::parse_preview(
            r#"
app_name: TestFly
debug: true
uuid: 11111111-1111-1111-1111-111111111111
token_id: testtokenid12345
token: test-token
api:
  host: 127.0.0.1
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  username: featherfly
  pid_file: ./featherfly.pid
"#
            .as_bytes(),
        )
        .unwrap();

        assert_eq!(inner.app_name, "TestFly");
        assert_eq!(inner.uuid, "11111111-1111-1111-1111-111111111111");
        assert_eq!(inner.token_id, "testtokenid12345");
        assert_eq!(inner.token, "test-token");
        assert_eq!(inner.api.port, 9090);
        assert!(inner.debug);
    }

    #[test]
    fn migrates_legacy_remote_and_node_blocks() {
        let inner = Config::parse_preview(
            r#"
debug: true
node:
  id: legacy-node-id
remote:
  config_edit: false
  restart: true
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#
            .as_bytes(),
        )
        .unwrap();

        assert_eq!(inner.uuid, "legacy-node-id");
        assert!(!inner.management.config_edit);
        assert!(inner.management.restart);
    }

    #[test]
    fn auto_generates_node_identity_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yml");
        std::fs::write(&config_path, minimal_open_yaml(dir.path(), "")).unwrap();

        let (config, _guard, meta) =
            Config::open(config_path.to_str().unwrap(), true, false).unwrap();

        assert!(meta.identity_generated);
        assert_eq!(config.load().uuid.len(), 36);
        assert_eq!(config.load().token_id.len(), 16);
        assert_eq!(config.load().token.len(), 64);

        let saved = std::fs::read_to_string(&config_path).unwrap();
        assert!(saved.contains("uuid:"));
        assert!(saved.contains("token_id:"));
        assert!(saved.contains("mysql_image:"));
    }

    #[test]
    fn open_persists_missing_defaults_without_identity_change() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yml");
        std::fs::write(
            &config_path,
            format!(
                "{}\nuuid: 11111111-1111-1111-1111-111111111111\ntoken_id: testtokenid12345\ntoken: test-token",
                minimal_open_yaml(
                    dir.path(),
                    r"  shared_services: true
",
                )
            ),
        )
        .unwrap();

        let (config, _guard, meta) =
            Config::open(config_path.to_str().unwrap(), true, false).unwrap();

        assert!(!meta.identity_generated);
        assert!(meta.config_upgraded);
        assert_eq!(config.load().hosting.mongodb_image, "mongo:7");

        let saved = std::fs::read_to_string(&config_path).unwrap();
        assert!(saved.contains("mongodb_image:"));
        assert!(saved.contains("database_root_password:"));
    }

    #[test]
    fn legacy_migration_triggers_config_persist() {
        let raw = br#"
debug: true
node:
  id: legacy-node-id
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#;
        let inner = Config::parse_preview(raw).unwrap();
        assert!(Config::config_needs_persist(raw, &inner).unwrap());
    }

    #[test]
    fn hosting_defaults_include_shared_database_settings() {
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

        assert_eq!(inner.hosting.mysql_image, "mysql:8");
        assert_eq!(inner.hosting.postgres_image, "postgres:16-alpine");
        assert_eq!(inner.hosting.redis_image, "redis:7-alpine");
        assert_eq!(inner.hosting.mongodb_image, "mongo:7");
        assert!(inner.hosting.shared_services);
        assert_eq!(inner.hosting.ports.mysql, 3306);
        assert_eq!(inner.hosting.ports.postgres, 5432);
        assert_eq!(inner.hosting.ports.redis, 6379);
        assert_eq!(inner.hosting.ports.mongodb, 27017);
    }

    #[test]
    fn value_contains_all_keys_detects_nested_missing_fields() {
        let disk: serde_norway::Value = serde_norway::from_str(
            r"
hosting:
  ports:
    http: 80
",
        )
        .unwrap();
        let complete: serde_norway::Value = serde_norway::from_str(
            r"
hosting:
  ports:
    http: 80
    mysql: 3306
",
        )
        .unwrap();
        assert!(!value_contains_all_keys(&disk, &complete));
    }

    #[test]
    fn upgrades_config_with_missing_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yml");
        std::fs::write(
            &config_path,
            format!(
                "{}\nuuid: 11111111-1111-1111-1111-111111111111\ntoken_id: testtokenid12345\ntoken: test-token",
                minimal_open_yaml(
                    dir.path(),
                    r"  shared_services: true
  ports:
    http: 80
    https: 443
",
                )
            ),
        )
        .unwrap();

        let raw = std::fs::read(&config_path).unwrap();
        let inner = Config::parse_preview(&raw).unwrap();
        assert_eq!(inner.hosting.mysql_image, "mysql:8");
        assert_eq!(inner.hosting.ports.mysql, 3306);
        assert!(Config::config_needs_persist(&raw, &inner).unwrap());

        let yaml = serde_norway::to_string(&inner).unwrap();
        std::fs::write(&config_path, yaml).unwrap();

        let saved = std::fs::read_to_string(&config_path).unwrap();
        assert!(saved.contains("mysql_image:"));
        assert!(saved.contains("mongodb:"));

        let raw_after = std::fs::read(&config_path).unwrap();
        assert!(!Config::config_needs_persist(&raw_after, &inner).unwrap());
    }
}
