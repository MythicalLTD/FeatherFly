use crate::config::{InnerConfig, LoggingConfig};
use anyhow::Context;
use std::path::{Path, PathBuf};
use tracing_subscriber::fmt::writer::MakeWriterExt;

pub struct LogGuards {
    _stdout: Option<tracing_appender::non_blocking::WorkerGuard>,
    _file: Option<tracing_appender::non_blocking::WorkerGuard>,
}

pub fn init(inner: &InnerConfig, is_subcommand: bool) -> anyhow::Result<LogGuards> {
    if tracing::dispatcher::has_been_set() {
        return Ok(LogGuards {
            _stdout: None,
            _file: None,
        });
    }

    let log_dir = Path::new(&inner.system.log_directory);
    let level = inner.logging.effective_level(inner.debug, is_subcommand);

    if inner.logging.enabled {
        std::fs::create_dir_all(log_dir)
            .with_context(|| format!("failed to create log directory {}", log_dir.display()))?;

        if inner.logging.archive.prune_on_startup {
            let removed = prune_archives(inner)?;
            if removed > 0 {
                eprintln!("pruned {removed} old log archive(s)");
            }
        }
    }

    let log_to_stdout = inner.logging.log_to_stdout || !inner.logging.enabled;
    let log_to_file = inner.logging.enabled;

    let stdout_layer = if log_to_stdout {
        Some(tracing_appender::non_blocking(std::io::stdout()))
    } else {
        None
    };

    let file_layer = if log_to_file {
        Some(build_file_writer(inner)?)
    } else {
        None
    };

    let guards = match (stdout_layer, file_layer) {
        (Some((stdout, stdout_guard)), Some((file, file_guard))) => {
            let writer = stdout.and(file);
            init_subscriber(inner, level, writer)?;
            LogGuards {
                _stdout: Some(stdout_guard),
                _file: Some(file_guard),
            }
        }
        (Some((stdout, stdout_guard)), None) => {
            init_subscriber(inner, level, stdout)?;
            LogGuards {
                _stdout: Some(stdout_guard),
                _file: None,
            }
        }
        (None, Some((file, file_guard))) => {
            init_subscriber(inner, level, file)?;
            LogGuards {
                _stdout: None,
                _file: Some(file_guard),
            }
        }
        (None, None) => {
            init_subscriber(inner, level, std::io::sink)?;
            LogGuards {
                _stdout: None,
                _file: None,
            }
        }
    };

    if log_to_file {
        tracing::debug!(
            directory = %log_dir.display(),
            latest = %inner.logging.latest_file,
            rotation = inner.logging.archive.rotation.as_str(),
            max_files = inner.logging.archive.max_files,
            level = %level,
            "file logging initialized"
        );
    }

    Ok(guards)
}

fn build_file_writer(
    inner: &InnerConfig,
) -> anyhow::Result<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let log_dir = Path::new(&inner.system.log_directory);
    let latest_path = inner.logging.latest_path(log_dir);

    let latest_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&latest_path)
        .with_context(|| format!("failed to open latest log file {}", latest_path.display()))?;

    let archive = &inner.logging.archive;

    let writer: Box<dyn std::io::Write + Send + 'static> = if archive.enabled {
        let rolling = tracing_appender::rolling::Builder::new()
            .filename_prefix(&archive.prefix)
            .filename_suffix(&archive.suffix)
            .max_log_files(archive.max_files)
            .rotation(archive.rotation.as_tracing_rotation())
            .build(log_dir)
            .with_context(|| {
                format!(
                    "failed to create log archive appender in {}",
                    log_dir.display()
                )
            })?;

        Box::new(latest_file.and(rolling))
    } else {
        Box::new(latest_file)
    };

    Ok(
        tracing_appender::non_blocking::NonBlockingBuilder::default()
            .buffered_lines_limit(128)
            .finish(writer),
    )
}

fn init_subscriber(
    inner: &InnerConfig,
    level: tracing::Level,
    writer: impl for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let format = &inner.logging.format;
    let use_file_context = format.include_file_line || inner.debug;

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            format.timestamp.clone(),
        ))
        .with_writer(writer)
        .with_target(format.include_target)
        .with_level(true)
        .with_file(use_file_context)
        .with_line_number(use_file_context)
        .with_max_level(level)
        .try_init()
        .map_err(|err| anyhow::anyhow!("failed to initialize tracing subscriber: {err}"))
}

pub fn prune_archives(inner: &InnerConfig) -> anyhow::Result<usize> {
    let archive = &inner.logging.archive;
    if !inner.logging.enabled || !archive.enabled {
        return Ok(0);
    }

    let log_dir = Path::new(&inner.system.log_directory);
    if !log_dir.is_dir() {
        return Ok(0);
    }

    let latest_name = inner.logging.latest_file.as_str();
    let mut candidates = Vec::new();

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if name == latest_name {
            continue;
        }

        if !archive.looks_like_archive(name) {
            continue;
        }

        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        candidates.push((path, modified));
    }

    candidates.sort_by_key(|(_, modified)| *modified);

    let mut removed = 0usize;
    let now = std::time::SystemTime::now();

    if archive.max_age_days > 0 {
        let max_age = std::time::Duration::from_secs(archive.max_age_days as u64 * 86_400);
        candidates.retain(|(path, modified)| {
            if now.duration_since(*modified).unwrap_or_default() > max_age {
                if std::fs::remove_file(path).is_ok() {
                    removed += 1;
                }
                false
            } else {
                true
            }
        });
    }

    if archive.max_files > 0 && candidates.len() > archive.max_files {
        let delete_count = candidates.len() - archive.max_files;
        for (path, _) in candidates.into_iter().take(delete_count) {
            if std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

impl LoggingConfig {
    pub fn latest_path(&self, log_dir: &Path) -> PathBuf {
        log_dir.join(&self.latest_file)
    }

    pub fn effective_level(&self, debug: bool, is_subcommand: bool) -> tracing::Level {
        let configured = parse_level(&self.level);
        if debug && !is_subcommand {
            std::cmp::max(configured, tracing::Level::DEBUG)
        } else {
            configured
        }
    }
}

pub fn parse_level(level: &str) -> tracing::Level {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" | "warning" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InnerConfig, LogArchiveConfig, LoggingConfig, SystemConfig};

    #[test]
    fn parse_level_accepts_common_names() {
        assert_eq!(parse_level("debug"), tracing::Level::DEBUG);
        assert_eq!(parse_level("WARNING"), tracing::Level::WARN);
    }

    #[test]
    fn archive_name_detection_matches_prefix_suffix() {
        let logging = LoggingConfig {
            archive: LogArchiveConfig {
                prefix: "featherfly".into(),
                suffix: "log".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(
            logging
                .archive
                .looks_like_archive("featherfly.2026-05-23.log")
        );
        assert!(!logging.archive.looks_like_archive("latest.log"));
    }

    #[test]
    fn prune_respects_max_files() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().to_string_lossy().into_owned();

        for day in 1..=5 {
            std::fs::write(
                dir.path().join(format!("featherfly.2026-05-{day:02}.log")),
                "x",
            )
            .unwrap();
        }

        let inner = InnerConfig {
            system: SystemConfig {
                log_directory: log_dir,
                ..Default::default()
            },
            logging: LoggingConfig {
                enabled: true,
                archive: LogArchiveConfig {
                    enabled: true,
                    max_files: 2,
                    prune_on_startup: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let removed = prune_archives(&inner).unwrap();
        assert_eq!(removed, 3);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}
