use chrono::Datelike;
use clap::{Args, FromArgMatches};

#[derive(Args)]
pub struct VersionArgs {
    #[arg(long, help = "check GitHub for a newer FeatherFly build")]
    check: bool,

    #[arg(long, help = "release channel to check (defaults to stable)")]
    channel: Option<String>,
}

pub struct VersionCommand;

impl crate::commands::CliCommand<VersionArgs> for VersionCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|config, _arg_matches| {
            Box::pin(async move {
                let args = VersionArgs::from_arg_matches(&_arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                println!("featherfly {} ({})", crate::full_version(), crate::TARGET);
                println!("{}", crate::GITHUB_REPOSITORY);
                println!("releases: {}", crate::utils::update::releases_page_url());
                println!(
                    "nightly: {}",
                    crate::utils::update::nightly_download_page_url()
                );
                println!("{}", crate::PROJECT_WEBSITE);
                println!("license: {}", crate::PROJECT_LICENSE);
                println!(
                    "copyright © 2025 - {} MythicalSystems Studios",
                    chrono::Local::now().year()
                );

                if args.check {
                    println!();
                    let channel = args
                        .channel
                        .as_deref()
                        .map(parse_channel)
                        .transpose()?
                        .or_else(|| config.map(|config| config.load().updates.channel))
                        .unwrap_or(crate::utils::update::UpdateChannel::Stable);

                    let status = crate::utils::update::check_update(channel).await?;
                    println!("channel: {:?}", status.channel);
                    println!(
                        "update available: {}",
                        if status.update_available { "yes" } else { "no" }
                    );

                    if let Some(latest_version) = &status.latest_version {
                        println!("latest version: {latest_version}");
                    }

                    if let Some(latest_commit) = &status.latest_commit {
                        println!("latest commit: {latest_commit}");
                    }

                    if status.update_available {
                        if let Some(download_url) = &status.download_url {
                            println!("download: {download_url}");
                        }
                        if let Some(release_url) = &status.release_url {
                            println!("release: {release_url}");
                        }
                    }
                }

                Ok(0)
            })
        })
    }
}

fn parse_channel(value: &str) -> Result<crate::utils::update::UpdateChannel, anyhow::Error> {
    match value.to_ascii_lowercase().as_str() {
        "stable" => Ok(crate::utils::update::UpdateChannel::Stable),
        "nightly" => Ok(crate::utils::update::UpdateChannel::Nightly),
        "disabled" => Ok(crate::utils::update::UpdateChannel::Disabled),
        _ => anyhow::bail!("invalid update channel: {value}"),
    }
}
