use anyhow::Context;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use serde_default::DefaultFromSerde;
use std::{path::PathBuf, sync::Arc};

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

#[derive(Debug, Clone, Deserialize, Serialize, DefaultFromSerde)]
pub struct InnerConfig {
    #[serde(default = "app_name")]
    pub app_name: String,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub token: String,

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
    pub remote: RemoteConfig,

    #[serde(default)]
    pub security: SecurityConfig,
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
pub struct RemoteConfig {
    #[serde(default = "default_true")]
    pub config_edit: bool,

    #[serde(default = "default_true")]
    pub restart: bool,
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
    pub channel: crate::update::UpdateChannel,

    #[serde(default = "updates_check_on_startup_default")]
    pub check_on_startup: bool,

    #[serde(default)]
    pub ignore_panel_upgrades: bool,
}

fn updates_channel_default() -> crate::update::UpdateChannel {
    crate::update::UpdateChannel::Stable
}

fn updates_check_on_startup_default() -> bool {
    true
}

pub struct Config {
    pub inner: ArcSwap<InnerConfig>,
    pub path: String,
}

pub struct ConfigGuard(#[allow(dead_code)] crate::logging::LogGuards);

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
    ) -> Result<(Arc<Self>, ConfigGuard), anyhow::Error> {
        let raw = std::fs::read(path).context(format!("failed to read config file {path}"))?;
        let inner = Self::parse_preview(&raw)?;
        Self::open_from_inner(inner, path, debug, is_subcommand)
    }

    pub fn parse_preview(bytes: &[u8]) -> Result<InnerConfig, anyhow::Error> {
        serde_norway::from_slice(bytes).context("failed to parse config yaml")
    }

    pub fn apply_debug_overrides(mut inner: InnerConfig, debug: bool) -> InnerConfig {
        if debug {
            inner.debug = true;
            inner.system.root_directory = "./data".into();
            inner.system.log_directory = "./logs".into();
            inner.system.tmp_directory = "./tmp".into();
            inner.system.pid_file = "./featherfly.pid".into();
            inner.system.plugins_directory = "./data/plugins".into();
        }
        inner
    }

    pub fn ensure_directories(inner: &InnerConfig) -> Result<(), anyhow::Error> {
        for dir in [
            &inner.system.root_directory,
            &inner.system.log_directory,
            &inner.system.tmp_directory,
            &inner.system.plugins_directory,
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
        crate::logging::init(inner, is_subcommand).map(ConfigGuard)
    }

    pub fn open_from_inner(
        mut inner: InnerConfig,
        path: &str,
        debug: bool,
        is_subcommand: bool,
    ) -> Result<(Arc<Self>, ConfigGuard), anyhow::Error> {
        inner = Self::apply_debug_overrides(inner, debug);

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

        Ok((config, log_guards))
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
        serde_norway::to_string(&inner).context("failed to serialize config")
    }

    pub fn apply_yaml(&self, yaml: &str) -> Result<ConfigApplyResult, anyhow::Error> {
        let mut new_inner: InnerConfig =
            serde_norway::from_str(yaml).context("failed to parse config yaml")?;

        let old = self.load();
        if new_inner.token == "***" {
            new_inner.token = old.token.clone();
        }

        Self::validate_inner(&new_inner)?;
        let restart_reasons = restart_reasons(&old, &new_inner);

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
            token: "docs".into(),
            api: ApiConfig {
                host: "127.0.0.1".into(),
                port: 8080,
                disable_openapi_docs: false,
            },
            system: SystemConfig {
                root_directory: "./data".into(),
                log_directory: "./logs".into(),
                tmp_directory: "./tmp".into(),
                username: "featherfly".into(),
                pid_file: "./featherfly.pid".into(),
                plugins_directory: "./data/plugins".into(),
            },
            updates: UpdatesConfig::default(),
            plugins: PluginsConfig::default(),
            logging: LoggingConfig::default(),
            remote: RemoteConfig::default(),
            security: SecurityConfig::default(),
        };

        Arc::new(Self {
            inner: ArcSwap::new(Arc::new(inner)),
            path: "docs".into(),
        })
    }
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

    reasons
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
            app_name: "FeatherFly".into(),
            debug: false,
            token: String::new(),
            api: ApiConfig {
                host: "127.0.0.1".into(),
                port: 0,
                disable_openapi_docs: false,
            },
            system: SystemConfig {
                root_directory: "/tmp/featherfly-test".into(),
                log_directory: "/tmp/featherfly-test/logs".into(),
                tmp_directory: "/tmp/featherfly-test/tmp".into(),
                username: "featherfly".into(),
                pid_file: "/tmp/featherfly-test/featherfly.pid".into(),
                plugins_directory: "/tmp/featherfly-test/plugins".into(),
            },
            updates: UpdatesConfig::default(),
            plugins: PluginsConfig::default(),
            logging: LoggingConfig::default(),
            remote: RemoteConfig::default(),
            security: SecurityConfig::default(),
        };

        let error = super::Config::validate_inner(&inner).unwrap_err();
        assert!(error.to_string().contains("api.port"));
    }

    #[test]
    fn loads_config_from_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yml");
        std::fs::write(
            &config_path,
            r#"
app_name: TestFly
debug: true
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
"#,
        )
        .unwrap();

        let (config, _guard) = Config::open(config_path.to_str().unwrap(), true, false).unwrap();

        assert_eq!(config.load().app_name, "TestFly");
        assert_eq!(config.load().token, "test-token");
        assert_eq!(config.load().api.port, 9090);
        assert!(config.load().debug);
    }
}
