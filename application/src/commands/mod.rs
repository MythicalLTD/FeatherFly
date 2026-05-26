#![allow(dead_code)]

use clap::{Arg, ArgMatches, Args, Command};
use std::{collections::HashMap, pin::Pin, sync::Arc};

mod configure;
mod diagnostics;
mod docs;
mod install;
mod plugin;
mod version;

pub type ExecutorFunc = dyn Fn(
        Option<Arc<crate::config::Config>>,
        ArgMatches,
    ) -> Pin<Box<dyn Future<Output = Result<i32, anyhow::Error>>>>
    + Send;

pub enum CommandMapEntry {
    Command(Box<ExecutorFunc>),
    Group(HashMap<&'static str, CommandMapEntry>),
}

pub struct CliCommandGroupBuilder {
    command: Command,
    map: HashMap<&'static str, CommandMapEntry>,
}

impl CliCommandGroupBuilder {
    pub fn new(name: &'static str, about: &'static str) -> Self {
        Self {
            command: Command::new(name)
                .version(crate::full_version())
                .arg(
                    Arg::new("config")
                        .help("set the location for the configuration file")
                        .num_args(1)
                        .short('c')
                        .long("config")
                        .alias("config-file")
                        .alias("config-path")
                        .default_value(crate::DEFAULT_CONFIG_PATH)
                        .global(true)
                        .required(false),
                )
                .arg(
                    Arg::new("debug")
                        .help("run featherfly in debug mode")
                        .num_args(0)
                        .short('d')
                        .long("debug")
                        .default_value("false")
                        .value_parser(clap::value_parser!(bool))
                        .global(true)
                        .required(false),
                )
                .about(about),
            map: HashMap::new(),
        }
    }

    pub fn get_matches(&mut self) -> ArgMatches {
        self.command.get_matches_mut()
    }

    pub fn print_help(&mut self) {
        let _ = self.command.print_long_help();
    }

    pub fn match_command(
        &self,
        command: String,
        arg_matches: ArgMatches,
    ) -> Option<(&ExecutorFunc, ArgMatches)> {
        let entry = self.map.get(command.as_str())?;

        match entry {
            CommandMapEntry::Command(executor) => Some((executor, arg_matches)),
            CommandMapEntry::Group(_) => None,
        }
    }

    pub fn add_command<A: Args>(
        mut self,
        name: &'static str,
        about: &'static str,
        cli_command: impl CliCommand<A>,
    ) -> Self {
        let command = cli_command.get_command(Command::new(name).about(about));
        let command = A::augment_args(command);

        self.command = self.command.subcommand(command);
        self.map
            .insert(name, CommandMapEntry::Command(cli_command.get_executor()));

        self
    }
}

pub trait CliCommand<A: Args> {
    fn get_command(&self, command: Command) -> Command;
    fn get_executor(self) -> Box<ExecutorFunc>;
}

pub fn commands(cli: CliCommandGroupBuilder) -> CliCommandGroupBuilder {
    cli.add_command(
        "version",
        "Prints the current executable version and exits.",
        version::VersionCommand,
    )
    .add_command(
        "install",
        "Install FeatherFly binary, config layout, and systemd unit.",
        install::InstallCommand,
    )
    .add_command(
        "configure",
        "Configure CloudPanel URL and authentication token.",
        configure::ConfigureCommand,
    )
    .add_command(
        "diagnostics",
        "Collects diagnostic information for support and troubleshooting.",
        diagnostics::DiagnosticsCommand,
    )
    .add_command(
        "docs",
        "Generate static plugin SDK documentation.",
        docs::DocsCommand,
    )
    .add_command(
        "plugin",
        "Build, install, and ship FeatherFly plugins.",
        plugin::PluginCommand,
    )
}
