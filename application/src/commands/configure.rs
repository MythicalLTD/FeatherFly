use anyhow::Context;
use clap::{Args, FromArgMatches};
use std::sync::Arc;

#[derive(Args)]
pub struct ConfigureArgs {
    #[arg(long, help = "FeatherPanel base URL (stored as remote)")]
    panel_url: String,

    #[arg(long, help = "Node authentication token from FeatherPanel")]
    token: String,
}

pub struct ConfigureCommand;

impl crate::commands::CliCommand<ConfigureArgs> for ConfigureCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|config, arg_matches| {
            Box::pin(async move {
                let args = ConfigureArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                let config_path = config
                    .as_ref()
                    .map(|config| config.path.clone())
                    .unwrap_or_else(|| crate::DEFAULT_CONFIG_PATH.to_string());

                let raw = std::fs::read(&config_path)
                    .with_context(|| format!("failed to read config file {config_path}"))?;

                let mut inner = crate::config::Config::parse_preview(&raw)
                    .context("failed to parse config file")?;

                inner.remote = args.panel_url.trim().trim_end_matches('/').to_string();
                inner.token = args.token;

                let _ = crate::utils::identity::ensure_node_identity(
                    &mut inner.uuid,
                    &mut inner.token_id,
                    &mut inner.token,
                );

                let config = Arc::new(crate::config::Config {
                    inner: arc_swap::ArcSwap::from(Arc::new(inner)),
                    path: config_path.clone(),
                });

                config.persist_inner()?;

                println!("configured node at {config_path}");
                println!("  remote: {}", config.load().remote);
                println!("  uuid:   {}", config.load().uuid);
                println!("  token_id: {}", config.load().token_id);

                Ok(0)
            })
        })
    }
}
