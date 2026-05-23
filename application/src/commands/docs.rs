use clap::{Args, FromArgMatches, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct DocsCommandArgs {
    #[command(subcommand)]
    action: DocsArgs,
}

#[derive(Subcommand)]
enum DocsArgs {
    Generate(DocsGenerateArgs),
}

#[derive(Args)]
struct DocsGenerateArgs {
    #[arg(long, default_value = "../docs")]
    output: PathBuf,
}

pub struct DocsCommand;

impl crate::commands::CliCommand<DocsCommandArgs> for DocsCommand {
    fn get_command(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn get_executor(self) -> Box<crate::commands::ExecutorFunc> {
        Box::new(|_config, arg_matches| {
            Box::pin(async move {
                let args = DocsCommandArgs::from_arg_matches(&arg_matches)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;

                let DocsArgs::Generate(generate_args) = args.action;
                let openapi = crate::api_spec::build_openapi("FeatherFly");
                featherfly_docgen::generate_all(&generate_args.output, &openapi)
                    .map_err(anyhow::Error::from)?;
                println!("generated docs at {}", generate_args.output.display());
                Ok(0)
            })
        })
    }
}
