use anyhow::{Context, bail};
use clap::{Args, FromArgMatches, ValueEnum};
use std::path::Path;
use std::process::Command;

#[derive(Clone, Copy, ValueEnum)]
enum ChannelArg {
    Stable,
    Nightly,
}

impl From<ChannelArg> for crate::utils::update::UpdateChannel {
    fn from(value: ChannelArg) -> Self {
        match value {
            ChannelArg::Stable => Self::Stable,
            ChannelArg::Nightly => Self::Nightly,
        }
    }
}

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(long, help = "download and install the latest available build")]
    apply: bool,

    #[arg(
        long,
        help = "restart featherfly through systemd after applying the update"
    )]
    restart: bool,

    #[arg(
        long,
        value_enum,
        help = "release channel to use (defaults to configured channel)"
    )]
    channel: Option<ChannelArg>,
}

pub struct UpdateCommand;

impl crate::commands::CliCommand<UpdateArgs> for UpdateCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|config, arg_matches| {
            Box::pin(async move {
                let args = UpdateArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                if args.apply {
                    run_apply(config.as_ref(), args).await
                } else {
                    run_check(config.as_ref(), args).await
                }
            })
        })
    }
}

async fn run_check(
    config: Option<&std::sync::Arc<crate::config::Config>>,
    args: UpdateArgs,
) -> Result<i32, anyhow::Error> {
    let channel = resolve_channel(config, args.channel);
    let status = crate::utils::update::check_update(channel).await?;

    print_status(&status);

    if status.update_available {
        println!();
        println!(
            "download: {}",
            status.download_url.as_deref().unwrap_or("unknown")
        );
        if let Some(release_url) = &status.release_url {
            println!("release: {release_url}");
        }
        println!("install: sudo featherfly update --apply");
    } else {
        println!("FeatherFly is up to date.");
    }

    Ok(0)
}

async fn run_apply(
    config: Option<&std::sync::Arc<crate::config::Config>>,
    args: UpdateArgs,
) -> Result<i32, anyhow::Error> {
    if !is_root() {
        bail!("update apply must be run as root");
    }

    let channel = resolve_channel(config, args.channel);
    let result = crate::utils::update::apply_update(channel).await?;

    println!("installed update to {}", result.binary_path);

    if let Some(version) = &result.installed_version {
        println!("version: {version}");
    }

    if args.restart {
        restart_service()?;
    } else if Path::new("/run/systemd/system").exists() {
        println!("run: systemctl restart featherfly");
    }

    Ok(0)
}

fn resolve_channel(
    config: Option<&std::sync::Arc<crate::config::Config>>,
    override_channel: Option<ChannelArg>,
) -> crate::utils::update::UpdateChannel {
    if let Some(channel) = override_channel {
        return channel.into();
    }

    config
        .map(|config| config.load().updates.channel)
        .unwrap_or(crate::utils::update::UpdateChannel::Stable)
}

fn print_status(status: &crate::utils::update::UpdateStatus) {
    println!("channel: {:?}", status.channel);
    println!(
        "current: {} ({})",
        status.current_version, status.current_commit
    );

    if let Some(latest_version) = &status.latest_version {
        println!("latest version: {latest_version}");
    }

    if let Some(latest_commit) = &status.latest_commit {
        println!("latest commit: {latest_commit}");
    }

    if let Some(published_at) = &status.published_at {
        println!("published: {published_at}");
    }

    println!(
        "update available: {}",
        if status.update_available { "yes" } else { "no" }
    );
}

fn restart_service() -> Result<(), anyhow::Error> {
    let status = Command::new("systemctl")
        .args(["restart", "featherfly"])
        .status()
        .context("failed to run systemctl restart featherfly")?;

    if status.success() {
        println!("restarted featherfly.service");
        Ok(())
    } else {
        bail!("systemctl restart featherfly failed");
    }
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
