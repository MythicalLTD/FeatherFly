use anyhow::Context;
use clap::{Args, FromArgMatches, ValueEnum};
use colored::Colorize;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tokio::process::Command;

#[derive(ValueEnum, Clone, Default, PartialEq, Debug)]
pub enum InitSystem {
    #[default]
    Auto,
    Systemd,
    Openrc,
}

#[derive(Args)]
pub struct ServiceInstallArgs {
    #[arg(
        short = 'o',
        long = "override",
        help = "set to true to override an existing service file"
    )]
    r#override: bool,

    #[arg(
        short = 'i',
        long = "init",
        help = "specify the init system to install for (systemd, openrc, or auto)",
        default_value = "auto"
    )]
    init: InitSystem,
}

fn generate_systemd_service(binary_path: &Path) -> String {
    format!(
        r#"[Unit]
Description=FeatherFly Web Hosting Daemon
After=network.target

[Service]
User=root
KillMode=process
WorkingDirectory=/etc/featherfly
LimitNOFILE=4096
PIDFile=/var/run/featherfly/daemon.pid
ExecStart={}
Restart=on-failure
StartLimitInterval=180
StartLimitBurst=30
RestartSec=5s

[Install]
WantedBy=multi-user.target
"#,
        binary_path.display()
    )
}

fn generate_openrc_service(binary_path: &Path) -> String {
    format!(
        r#"#!/sbin/openrc-run

description="FeatherFly Web Hosting Daemon"

command="{}"
supervisor="supervise-daemon"
pidfile="/var/run/featherfly/daemon.pid"
directory="/etc/featherfly"
rc_ulimit="-n 4096"

respawn_delay=5
respawn_max=30

depend() {{
    need net
}}
"#,
        binary_path.display()
    )
}

pub struct ServiceInstallCommand;

impl crate::commands::CliCommand<ServiceInstallArgs> for ServiceInstallCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|config, arg_matches| {
            Box::pin(async move {
                let args = ServiceInstallArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                let binary_path = std::env::current_exe()
                    .context("failed to determine current executable path")?;

                let init = match args.init {
                    InitSystem::Auto => {
                        if Path::new("/run/systemd/system").exists() {
                            InitSystem::Systemd
                        } else if Path::new("/sbin/openrc").exists() {
                            InitSystem::Openrc
                        } else {
                            eprintln!(
                                "{}",
                                "could not detect init system, please specify with --init"
                                    .red()
                            );
                            return Ok(1);
                        }
                    }
                    other => other,
                };

                match init {
                    InitSystem::Systemd => {
                        let service_path = Path::new("/etc/systemd/system/featherfly.service");

                        if service_path.exists() && !args.r#override {
                            eprintln!(
                                "{}",
                                "service file already exists, use --override to replace it".red()
                            );
                            return Ok(1);
                        }

                        tokio::fs::write(service_path, generate_systemd_service(&binary_path))
                            .await
                            .context("failed to write systemd service file")?;

                        Command::new("systemctl")
                            .arg("daemon-reload")
                            .status()
                            .await
                            .context("failed to reload systemd")?;

                        Command::new("systemctl")
                            .args(["enable", "featherfly.service"])
                            .status()
                            .await
                            .context("failed to enable featherfly service")?;

                        println!("{}", "systemd service installed successfully".green());
                    }
                    InitSystem::Openrc => {
                        let service_path = Path::new("/etc/init.d/featherfly");

                        if service_path.exists() && !args.r#override {
                            eprintln!(
                                "{}",
                                "service file already exists, use --override to replace it".red()
                            );
                            return Ok(1);
                        }

                        tokio::fs::write(service_path, generate_openrc_service(&binary_path))
                            .await
                            .context("failed to write openrc service file")?;

                        std::fs::set_permissions(
                            service_path,
                            std::fs::Permissions::from_mode(0o755),
                        )
                        .context("failed to set openrc service permissions")?;

                        Command::new("rc-update")
                            .args(["add", "featherfly", "default"])
                            .status()
                            .await
                            .context("failed to enable featherfly service")?;

                        println!("{}", "openrc service installed successfully".green());
                    }
                    InitSystem::Auto => unreachable!(),
                }

                if let Some(config) = config {
                    println!(
                        "config: {}",
                        config.path
                    );
                }

                Ok(0)
            })
        })
    }
}
