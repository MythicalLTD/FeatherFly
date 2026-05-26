use clap::{Args, FromArgMatches};
use std::fmt::Write;

#[derive(Args)]
pub struct DiagnosticsArgs {
    #[arg(
        short = 'l',
        long = "log-lines",
        help = "number of recent log lines to include in the report",
        default_value_t = 50
    )]
    pub log_lines: usize,
}

pub struct DiagnosticsCommand;

impl crate::commands::CliCommand<DiagnosticsArgs> for DiagnosticsCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|config, arg_matches| {
            Box::pin(async move {
                let args = DiagnosticsArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                let Some(config) = config else {
                    eprintln!("no config found");
                    return Ok(1);
                };

                let inner = config.load();
                let mut report = String::new();

                writeln!(report, "FeatherFly diagnostics report").unwrap();
                writeln!(report).unwrap();

                write_section(&mut report, "version")?;
                write_line(&mut report, "featherfly", &crate::full_version())?;
                write_line(&mut report, "target", crate::TARGET)?;
                write_line(
                    &mut report,
                    "kernel",
                    &sysinfo::System::kernel_long_version(),
                )?;
                write_line(&mut report, "architecture", std::env::consts::ARCH)?;
                write_line(&mut report, "os", std::env::consts::OS)?;

                write_section(&mut report, "configuration")?;
                write_line(&mut report, "config path", &config.path)?;
                write_line(&mut report, "app name", &inner.app_name)?;
                write_line(&mut report, "debug", &inner.debug.to_string())?;
                write_line(
                    &mut report,
                    "api listen",
                    &format!("{}:{}", inner.api.host, inner.api.port),
                )?;
                write_line(&mut report, "root directory", &inner.system.root_directory)?;
                write_line(&mut report, "log directory", &inner.system.log_directory)?;
                write_line(&mut report, "tmp directory", &inner.system.tmp_directory)?;
                write_line(&mut report, "username", &inner.system.username)?;
                write_line(&mut report, "pid file", &inner.system.pid_file)?;

                write_section(&mut report, "runtime")?;
                write_line(
                    &mut report,
                    "cpu cores",
                    &std::thread::available_parallelism()
                        .map(|n| n.get().to_string())
                        .unwrap_or_else(|_| "unknown".into()),
                )?;

                let log_path = inner
                    .logging
                    .latest_path(std::path::Path::new(&inner.system.log_directory));
                write_section(&mut report, "logging")?;
                write_line(&mut report, "enabled", &inner.logging.enabled.to_string())?;
                write_line(&mut report, "level", &inner.logging.level)?;
                write_line(&mut report, "latest file", &inner.logging.latest_file)?;
                write_line(
                    &mut report,
                    "archive rotation",
                    inner.logging.archive.rotation.as_str(),
                )?;
                write_line(
                    &mut report,
                    "archive max files",
                    &inner.logging.archive.max_files.to_string(),
                )?;

                write_section(&mut report, "recent logs")?;
                match tokio::fs::read_to_string(&log_path).await {
                    Ok(contents) => {
                        let lines: Vec<&str> = contents.lines().collect();
                        let start = lines.len().saturating_sub(args.log_lines);
                        for line in &lines[start..] {
                            writeln!(report, "{line}").unwrap();
                        }
                    }
                    Err(err) => {
                        write_line(
                            &mut report,
                            "log read error",
                            &format!("{} ({err})", log_path.display()),
                        )?;
                    }
                }

                print!("{report}");
                Ok(0)
            })
        })
    }
}

fn write_section(output: &mut String, title: &str) -> anyhow::Result<()> {
    writeln!(output, "{title}")?;
    writeln!(output, "{}", "-".repeat(title.len()))?;
    Ok(())
}

fn write_line(output: &mut String, key: &str, value: &str) -> anyhow::Result<()> {
    writeln!(output, "{key}: {value}")?;
    Ok(())
}
