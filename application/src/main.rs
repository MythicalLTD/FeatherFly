use colored::Colorize;
use std::sync::Arc;

use clap::parser::ValueSource;

fn print_banner() {
    const BANNER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../banner.txt"));

    for line in BANNER.trim_end().lines() {
        println!("{line}");
    }

    println!(
        "featherfly v{} ({})",
        featherfly::VERSION,
        featherfly::GIT_COMMIT
    );
    println!("{}", featherfly::GITHUB_REPOSITORY);
    println!("{}", featherfly::PROJECT_WEBSITE);
    println!("license: {}", featherfly::PROJECT_LICENSE);
    println!();
}

async fn main_rt() -> Result<(), i32> {
    let cli = featherfly::commands::CliCommandGroupBuilder::new(
        "featherfly",
        "The FeatherFly daemon implementing web hosting for the panel.",
    );

    let mut cli = featherfly::commands::commands(cli);
    let mut matches = cli.get_matches();

    let debug = *matches.get_one::<bool>("debug").unwrap();
    let config_explicit = matches.get_one::<String>("config").map(String::as_str);
    let config_from_cli = matches
        .value_source("config")
        .is_some_and(|source| source == ValueSource::CommandLine);
    let config_path = match featherfly::config::Config::resolve_config_path(
        debug,
        config_explicit,
        config_from_cli,
    ) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to resolve config path: {err:#?}");
            return Err(1);
        }
    };

    if matches.subcommand().is_none() {
        print_banner();
    }

    let config =
        featherfly::config::Config::open(&config_path, debug, matches.subcommand().is_some());

    if let Some((command, arg_matches)) = matches.remove_subcommand() {
        if let Some((func, arg_matches)) = cli.match_command(command, arg_matches) {
            match func(config.as_ref().ok().map(|e| e.0.clone()), arg_matches).await {
                Ok(exit_code) => {
                    drop(config);
                    if exit_code != 0 {
                        return Err(exit_code);
                    }
                    return Ok(());
                }
                Err(err) => {
                    drop(config);
                    eprintln!(
                        "{}: {:#?}",
                        "an error occurred while running cli command".red(),
                        err
                    );
                    return Err(1);
                }
            }
        } else {
            cli.print_help();
            return Ok(());
        }
    }

    featherfly::daemon::start(&config_path, debug).await
}

fn main() {
    let thread_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(1024)
        .thread_name_fn({
            let thread_count = thread_count.clone();
            move || {
                let count = thread_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                format!("featherfly-rt-{count}")
            }
        })
        .name("featherfly-rt")
        .build()
        .unwrap()
        .block_on(main_rt())
        .unwrap_or_else(|code| std::process::exit(code));
}
