use anyhow::{Context, bail};
use clap::{Args, FromArgMatches};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tokio::process::Command;

const BIN_PATH: &str = "/usr/local/bin/featherfly";
const ALIAS_PATH: &str = "/usr/local/bin/featherd";
const CONFIG_DIR: &str = "/etc/featherfly";
const CONFIG_PATH: &str = "/etc/featherfly/config.yml";
const SERVICE_PATH: &str = "/etc/systemd/system/featherfly.service";
const DEFAULT_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../config.example.yml"
));

#[derive(Args)]
pub struct InstallArgs {
    #[arg(long, help = "replace an existing service file")]
    r#override: bool,
}

pub struct InstallCommand;

impl crate::commands::CliCommand<InstallArgs> for InstallCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|_config, arg_matches| {
            Box::pin(async move {
                let args = InstallArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                if !is_root() {
                    bail!("install must be run as root");
                }

                if !Path::new("/run/systemd/system").exists() {
                    bail!("systemd was not detected on this host");
                }

                if Path::new(SERVICE_PATH).exists() && !args.r#override {
                    bail!("{SERVICE_PATH} already exists (use --override to replace it)");
                }

                let current = std::env::current_exe().context("failed to locate current binary")?;
                install_binary(&current)?;
                install_layout()?;
                write_config_if_missing()?;
                write_systemd_unit()?;
                reload_systemd().await?;

                println!("installed {BIN_PATH}");
                println!("alias at {ALIAS_PATH}");
                println!("config at {CONFIG_PATH}");
                println!("run: systemctl enable --now featherfly");

                Ok(0)
            })
        })
    }
}

fn install_binary(current: &Path) -> anyhow::Result<()> {
    if current != Path::new(BIN_PATH) {
        fs::copy(current, BIN_PATH)
            .with_context(|| format!("failed to copy binary to {BIN_PATH}"))?;
        fs::set_permissions(BIN_PATH, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("failed to chmod {BIN_PATH}"))?;
    }

    if Path::new(ALIAS_PATH).exists() {
        fs::remove_file(ALIAS_PATH).ok();
    }
    std::os::unix::fs::symlink(BIN_PATH, ALIAS_PATH)
        .with_context(|| format!("failed to create {ALIAS_PATH} symlink"))?;

    Ok(())
}

fn install_layout() -> anyhow::Result<()> {
    for dir in [
        CONFIG_DIR,
        "/var/lib/featherfly",
        "/var/lib/featherfly/plugins",
        "/var/log/featherfly",
        "/tmp/featherfly",
        "/var/run/featherfly",
    ] {
        fs::create_dir_all(dir).with_context(|| format!("failed to create {dir}"))?;
    }

    Ok(())
}

fn write_config_if_missing() -> anyhow::Result<()> {
    if Path::new(CONFIG_PATH).exists() {
        return Ok(());
    }

    fs::write(CONFIG_PATH, DEFAULT_CONFIG).context("failed to write default config")?;
    Ok(())
}

fn write_systemd_unit() -> anyhow::Result<()> {
    let unit = format!(
        r#"[Unit]
Description=FeatherFly
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={BIN_PATH}
WorkingDirectory={CONFIG_DIR}
Restart=on-failure
RestartSec=5
LimitNOFILE=65535
PIDFile=/var/run/featherfly/daemon.pid

[Install]
WantedBy=multi-user.target
"#
    );

    fs::write(SERVICE_PATH, unit).context("failed to write systemd unit")?;
    Ok(())
}

async fn reload_systemd() -> anyhow::Result<()> {
    let status = Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .await
        .context("failed to run systemctl daemon-reload")?;

    if !status.success() {
        bail!("systemctl daemon-reload failed");
    }

    Ok(())
}

fn is_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                let mut parts = line.split_whitespace();
                if parts.next()? == "Uid:" {
                    parts.next()?.parse::<u32>().ok()
                } else {
                    None
                }
            })
        })
        .is_some_and(|uid| uid == 0)
}
