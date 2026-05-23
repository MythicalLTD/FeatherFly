use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use colored::Colorize;
use std::sync::Arc;

use clap::parser::ValueSource;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::security::SecurityRequirement;
use utoipa::openapi::ServerBuilder;

use crate::response::ApiResponse;

async fn handle_request(req: Request, next: Next) -> Result<Response<Body>, StatusCode> {
    tracing::debug!(
        path = req.uri().path(),
        query = req.uri().query().unwrap_or_default(),
        "http {}",
        req.method().as_str().to_lowercase(),
    );

    Ok(next.run(req).await)
}

mod commands;
mod config;
mod docs;
mod response;
mod routes;

#[cfg(not(unix))]
compile_error!("FeatherFly only supports Unix-like systems");

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_COMMIT: &str = env!("CARGO_GIT_COMMIT");
const GIT_BRANCH: &str = env!("CARGO_GIT_BRANCH");
const TARGET: &str = env!("CARGO_TARGET");

const DEFAULT_CONFIG_PATH: &str = "/etc/featherfly/config.yml";
pub(crate) const GITHUB_REPOSITORY: &str = "https://github.com/mythicalltd/featherfly";
pub(crate) const PROJECT_WEBSITE: &str = "https://featherpanel.com";
pub(crate) const PROJECT_LICENSE: &str = "MIT";

fn full_version() -> String {
    if GIT_BRANCH == "unknown" {
        VERSION.to_string()
    } else {
        format!("{VERSION}:{GIT_COMMIT}@{GIT_BRANCH}")
    }
}

fn print_banner() {
    const BANNER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../banner.txt"));

    for line in BANNER.trim_end().lines() {
        println!("{line}");
    }

    println!("featherfly v{} ({})", crate::VERSION, crate::GIT_COMMIT);
    println!("{GITHUB_REPOSITORY}");
    println!("{PROJECT_WEBSITE}");
    println!("license: {PROJECT_LICENSE}");
    println!();
}

async fn main_rt() {
    let cli = crate::commands::CliCommandGroupBuilder::new(
        "featherfly",
        "The FeatherFly daemon implementing web hosting for the panel.",
    );

    let mut cli = crate::commands::commands(cli);
    let mut matches = cli.get_matches();

    let debug = *matches.get_one::<bool>("debug").unwrap();
    let config_explicit = matches.get_one::<String>("config").map(String::as_str);
    let config_from_cli = matches
        .value_source("config")
        .is_some_and(|source| source == ValueSource::CommandLine);
    let config_path = match crate::config::Config::resolve_config_path(
        debug,
        config_explicit,
        config_from_cli,
    ) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to resolve config path: {err:#?}");
            std::process::exit(1);
        }
    };

    if matches.subcommand().is_none() {
        print_banner();
    }

    let config = crate::config::Config::open(
        &config_path,
        debug,
        matches.subcommand().is_some(),
    );

    match matches.remove_subcommand() {
        Some((command, arg_matches)) => {
            if let Some((func, arg_matches)) = cli.match_command(command, arg_matches) {
                match func(config.as_ref().ok().map(|e| e.0.clone()), arg_matches).await {
                    Ok(exit_code) => {
                        drop(config);
                        std::process::exit(exit_code);
                    }
                    Err(err) => {
                        drop(config);
                        eprintln!(
                            "{}: {:#?}",
                            "an error occurred while running cli command".red(),
                            err
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                cli.print_help();
                std::process::exit(0);
            }
        }
        None => {}
    }

    let (config, _guard) = match config {
        Ok(config) => config,
        Err(err) => {
            eprintln!("failed to load configuration: {err:#?}");
            std::process::exit(1);
        }
    };
    tracing::info!("config loaded from {}", config_path);

    let state = Arc::new(crate::routes::AppState {
        start_time: std::time::Instant::now(),
        container_type: match std::env::var("OCI_CONTAINER").as_deref() {
            Ok("official") => crate::routes::AppContainerType::Official,
            Ok(_) => crate::routes::AppContainerType::Unknown,
            Err(_) => crate::routes::AppContainerType::None,
        },
        version: crate::full_version(),
        config: Arc::clone(&config),
    });

    let app = utoipa_axum::router::OpenApiRouter::new()
        .merge(crate::routes::router(&state))
        .fallback(|_req: Request| async move {
            ApiResponse::error("route not found")
                .with_status(StatusCode::NOT_FOUND)
                .into_response()
        })
        .layer(axum::middleware::from_fn(handle_request))
        .with_state(state.clone());

    let (mut router, mut openapi) = app.split_for_parts();
    openapi.info.version = crate::VERSION.into();
    openapi.info.description = Some(
        "FeatherFly web hosting daemon API — part of the FeatherPanel ecosystem.".into(),
    );
    openapi.info.title = format!("{} API", config.load().app_name);
    openapi.info.contact = None;
    openapi.info.license = Some(utoipa::openapi::License::new(PROJECT_LICENSE));

    openapi.components.as_mut().unwrap().add_security_scheme(
        "bearer_auth",
        SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
    );

    let security_req = SecurityRequirement::new("bearer_auth", Vec::<String>::new());
    for (path, item) in openapi.paths.paths.iter_mut() {
        let operations = [
            ("get", &mut item.get),
            ("post", &mut item.post),
            ("put", &mut item.put),
            ("patch", &mut item.patch),
            ("delete", &mut item.delete),
        ];

        let operation_suffix = path
            .replace('/', "_")
            .replace(|c| ['{', '}'].contains(&c), "");

        for (method, operation) in operations {
            if let Some(operation) = operation {
                if operation.operation_id.as_deref() == Some("route") {
                    operation.operation_id = Some(format!("{method}{operation_suffix}"));
                }
                operation.security = Some(vec![security_req.clone()]);
            }
        }
    }

    openapi.servers = Some(vec![ServerBuilder::new().url("/").build()]);

    if !config.load().api.disable_openapi_docs {
        router = router
            .route(
                "/openapi.json",
                axum::routing::get(|| async move { axum::Json(openapi) }),
            )
            .route("/docs", axum::routing::get(crate::docs::swagger_ui));
    }

    if let Ok(host) = config.load().api.host.parse::<std::net::IpAddr>() {
        let address = std::net::SocketAddr::from((host, config.load().api.port));

        tracing::info!("http listening on {}", address);

        if !config.load().api.disable_openapi_docs {
            tracing::info!("api docs available at http://{}/docs", address);
        }

        match axum::serve(
            match tokio::net::TcpListener::bind(address).await {
                Ok(listener) => listener,
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::AddrInUse {
                        tracing::error!("failed to start http server (address already in use)");
                    } else {
                        tracing::error!("failed to start http server: {:?}", err);
                    }
                    std::process::exit(1);
                }
            },
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        {
            Ok(_) => {}
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AddrInUse {
                    tracing::error!("failed to start http server (address already in use)");
                } else {
                    tracing::error!("failed to start http server: {:?}", err);
                }
                std::process::exit(1);
            }
        }
    } else {
        tracing::error!("invalid api.host configured: {}", config.load().api.host);
        std::process::exit(1);
    }

    crate::config::Config::remove_pid_file(&config.load());
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
        .block_on(main_rt());
}
