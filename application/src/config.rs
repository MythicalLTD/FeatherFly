use anyhow::Context;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use serde_default::DefaultFromSerde;
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing_subscriber::fmt::writer::MakeWriterExt;

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

#[derive(Debug, Deserialize, Serialize, DefaultFromSerde)]
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
}

#[derive(Debug, Deserialize, Serialize, DefaultFromSerde)]
pub struct ApiConfig {
    #[serde(default = "api_host")]
    pub host: String,

    #[serde(default = "api_port")]
    pub port: u16,

    #[serde(default)]
    pub disable_openapi_docs: bool,
}

#[derive(Debug, Deserialize, Serialize, DefaultFromSerde)]
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
}

pub struct Config {
    pub inner: ArcSwap<InnerConfig>,
    pub path: String,
}

pub struct ConfigGuard(
    #[allow(dead_code)]
    Option<(
        tracing_appender::non_blocking::WorkerGuard,
        tracing_appender::non_blocking::WorkerGuard,
    )>,
);

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
        let file = File::open(path).context(format!("failed to open config file {path}"))?;
        let reader = std::io::BufReader::new(file);
        let mut inner: InnerConfig = serde_norway::from_reader(reader)
            .context(format!("failed to parse config file {path}"))?;

        if debug {
            inner.debug = true;
            inner.system.root_directory = "./data".into();
            inner.system.log_directory = "./logs".into();
            inner.system.tmp_directory = "./tmp".into();
            inner.system.pid_file = "./featherfly.pid".into();
        }

        Self::ensure_directories(&inner)?;

        let dev_mode = inner.debug || is_subcommand;

        let log_guards = if dev_mode {
            let _ = tracing_subscriber::fmt()
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_max_level(if inner.debug && !is_subcommand {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                })
                .try_init();

            None
        } else {
            let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

            let latest_log_path = Path::new(&inner.system.log_directory).join("featherfly.log");
            let latest_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&latest_log_path)
                .context("failed to open latest log file")?;

            let rolling_appender = tracing_appender::rolling::Builder::new()
                .filename_prefix("featherfly")
                .filename_suffix("log")
                .max_log_files(30)
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .build(&inner.system.log_directory)
                .context("failed to create rolling log file appender")?;

            let (file_appender, guard) =
                tracing_appender::non_blocking::NonBlockingBuilder::default()
                    .buffered_lines_limit(50)
                    .finish(latest_file.and(rolling_appender));

            let _ = tracing_subscriber::fmt()
                .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                    "%Y-%m-%d %H:%M:%S %z".to_string(),
                ))
                .with_writer(stdout_writer.and(file_appender))
                .with_target(false)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_max_level(tracing::Level::INFO)
                .try_init();

            Some((guard, stdout_guard))
        };

        Self::validate_inner(&inner)?;

        if !dev_mode {
            Self::write_pid_file(&inner)?;
        }

        let config = Arc::new(Self {
            inner: ArcSwap::new(Arc::new(inner)),
            path: path.to_string(),
        });

        Ok((config, ConfigGuard(log_guards)))
    }

    fn ensure_directories(inner: &InnerConfig) -> Result<(), anyhow::Error> {
        for dir in [
            &inner.system.root_directory,
            &inner.system.log_directory,
            &inner.system.tmp_directory,
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

    fn validate_inner(inner: &InnerConfig) -> Result<(), anyhow::Error> {
        if inner.api.port == 0 {
            anyhow::bail!("api.port must be greater than 0");
        }

        if inner.system.root_directory.is_empty() {
            anyhow::bail!("system.root_directory must not be empty");
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
            },
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
