use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

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

fn default_true() -> bool {
    true
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LogFormatConfig {
    #[serde(default = "timestamp_format_default")]
    pub timestamp: String,

    #[serde(default)]
    pub include_target: bool,

    #[serde(default)]
    pub include_file_line: bool,
}

impl Default for LogFormatConfig {
    fn default() -> Self {
        Self {
            timestamp: timestamp_format_default(),
            include_target: false,
            include_file_line: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

impl Default for LogArchiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: archive_prefix_default(),
            suffix: archive_suffix_default(),
            rotation: LogRotation::default(),
            max_files: archive_max_files_default(),
            max_age_days: 0,
            prune_on_startup: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: logging_enabled_default(),
            log_to_stdout: log_to_stdout_default(),
            level: log_level_default(),
            latest_file: latest_file_default(),
            archive: LogArchiveConfig::default(),
            format: LogFormatConfig::default(),
        }
    }
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InnerConfig {
    #[serde(default = "app_name")]
    pub app_name: String,

    #[serde(default)]
    pub debug: bool,

    /// Node UUID. Auto-generated on first boot if empty.
    #[serde(default = "node_id_default")]
    pub uuid: String,

    /// Panel authentication token ID. Auto-generated on first boot if empty.
    #[serde(default = "token_id_default")]
    pub token_id: String,

    /// Daemon API bearer token. Auto-generated on first boot if empty.
    #[serde(default)]
    pub token: String,

    /// CloudPanel base URL.
    #[serde(default)]
    pub remote: String,

    #[serde(default)]
    pub api: ApiConfig,

    #[serde(default)]
    pub system: SystemConfig,

    #[serde(default)]
    pub plugins: PluginsConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for InnerConfig {
    fn default() -> Self {
        Self {
            app_name: app_name(),
            debug: false,
            uuid: String::new(),
            token_id: String::new(),
            token: String::new(),
            remote: String::new(),
            api: ApiConfig::default(),
            system: SystemConfig::default(),
            plugins: PluginsConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

impl Default for ProbeProtectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_404s: probe_max_404s_default(),
            window_secs: probe_window_secs_default(),
            block_secs: probe_block_secs_default(),
            log_requests: true,
            log_unknown_routes: true,
            log_blocked: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    #[serde(default = "api_host")]
    pub host: String,

    #[serde(default = "api_port")]
    pub port: u16,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: api_host(),
            port: api_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            root_directory: system_root_directory(),
            log_directory: system_log_directory(),
            tmp_directory: system_tmp_directory(),
            username: system_username(),
            pid_file: system_pid_file(),
            plugins_directory: system_plugins_directory(),
        }
    }
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
                tracing::info!(path, "updated config file with bare daemon settings");
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

    fn config_needs_persist(raw: &[u8], inner: &InnerConfig) -> Result<bool, anyhow::Error> {
        let disk: serde_norway::Value =
            serde_norway::from_slice(raw).context("failed to parse config yaml")?;
        let complete: serde_norway::Value =
            serde_norway::to_value(inner).context("failed to serialize config")?;
        Ok(disk != complete)
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
            api: ApiConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            ..Default::default()
        };

        let error = Config::validate_inner(&inner).unwrap_err();
        assert!(error.to_string().contains("api.port"));
    }

    #[test]
    fn parse_preview_reads_bare_daemon_config() {
        let yaml = br#"
app_name: FeatherFly
token: test
api:
  host: 127.0.0.1
  port: 9090
"#;

        let inner = Config::parse_preview(yaml).unwrap();
        assert_eq!(inner.api.host, "127.0.0.1");
        assert_eq!(inner.token, "test");
    }
}
