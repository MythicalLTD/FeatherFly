use anyhow::{Context, bail};
use clap::{Args, FromArgMatches, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args)]
pub struct PluginCommandArgs {
    #[command(subcommand)]
    action: PluginArgs,
}

#[derive(Subcommand)]
pub enum PluginArgs {
    Build(PluginBuildArgs),
    Install(PluginInstallArgs),
    Ship(PluginShipArgs),
}

#[derive(Args)]
pub struct PluginBuildArgs {
    #[arg(default_value = "plugins/hello")]
    path: PathBuf,

    #[arg(long, default_value_t = true)]
    release: bool,
}

#[derive(Args)]
pub struct PluginInstallArgs {
    #[arg(default_value = "plugins/hello")]
    path: PathBuf,
}

#[derive(Args)]
pub struct PluginShipArgs {
    #[arg(default_value = "plugins/hello")]
    path: PathBuf,
}

pub struct PluginCommand;

impl crate::commands::CliCommand<PluginCommandArgs> for PluginCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|config, arg_matches| {
            Box::pin(async move {
                let args = PluginCommandArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                match args.action {
                    PluginArgs::Build(build_args) => run_build(build_args).await,
                    PluginArgs::Install(install_args) => {
                        run_install(config.as_ref(), install_args).await
                    }
                    PluginArgs::Ship(ship_args) => {
                        run_build(PluginBuildArgs {
                            path: ship_args.path.clone(),
                            release: true,
                        })
                        .await?;
                        run_install(
                            config.as_ref(),
                            PluginInstallArgs {
                                path: ship_args.path,
                            },
                        )
                        .await
                    }
                }
            })
        })
    }
}

async fn run_build(args: PluginBuildArgs) -> Result<i32, anyhow::Error> {
    let manifest = args
        .path
        .join("Cargo.toml")
        .canonicalize()
        .with_context(|| format!("plugin manifest not found in {}", args.path.display()))?;

    let mut command = Command::new("cargo");
    command.arg("build").arg("--manifest-path").arg(manifest);

    if args.release {
        command.arg("--release");
    }

    let status = command
        .status()
        .context("failed to run cargo build for plugin")?;

    if !status.success() {
        bail!("plugin build failed");
    }

    if let Some(artifact) = crate::plugins::find_plugin_artifact(&args.path) {
        println!("built {}", artifact.display());
    }

    Ok(0)
}

async fn run_install(
    config: Option<&std::sync::Arc<crate::config::Config>>,
    args: PluginInstallArgs,
) -> Result<i32, anyhow::Error> {
    let artifact = crate::plugins::find_plugin_artifact(&args.path)
        .with_context(|| format!("no plugin .so found for {}", args.path.display()))?;

    let plugins_directory = config
        .map(|config| config.load().system.plugins_directory.clone())
        .unwrap_or_else(|| "/var/lib/featherfly/plugins".to_string());

    let destination = Path::new(&plugins_directory).join(
        artifact
            .file_name()
            .context("plugin artifact is missing a file name")?,
    );

    std::fs::create_dir_all(&plugins_directory)
        .with_context(|| format!("failed to create plugins directory {}", plugins_directory))?;

    std::fs::copy(&artifact, &destination).with_context(|| {
        format!(
            "failed to install plugin from {} to {}",
            artifact.display(),
            destination.display()
        )
    })?;

    println!("installed plugin to {}", destination.display());
    Ok(0)
}
