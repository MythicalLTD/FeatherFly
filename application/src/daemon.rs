use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

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

pub async fn start(config_path: &str, debug: bool) -> Result<(), i32> {
    let raw = match std::fs::read(config_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("failed to read config file {config_path}: {err:#?}");
            return Err(1);
        }
    };

    let preview = match crate::config::Config::parse_preview(&raw) {
        Ok(inner) => inner,
        Err(err) => {
            eprintln!("failed to parse config file {config_path}: {err:#?}");
            return Err(1);
        }
    };

    let preview = crate::config::Config::apply_debug_overrides(preview, debug);

    if let Err(err) = crate::config::Config::ensure_directories(&preview) {
        eprintln!("failed to prepare data directories: {err:#?}");
        return Err(1);
    }

    let _early_log_guard = match crate::config::Config::init_tracing(&preview, debug, false) {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("failed to initialize logging: {err:#?}");
            return Err(1);
        }
    };

    tracing::info!("starting FeatherFly daemon");

    let plugins_dir = std::path::Path::new(&preview.system.plugins_directory);
    tracing::debug!(
        path = %plugins_dir.display(),
        enabled = preview.plugins.enabled,
        api_version = featherfly_plugin_sdk::FEATHERFLY_PLUGIN_API_VERSION,
        "loading plugins"
    );

    let plugin_registry = crate::plugins::load_directory(plugins_dir, preview.plugins.enabled)
        .unwrap_or_else(|err| {
            tracing::error!("failed to load plugins: {err:#}");
            crate::plugins::PluginRegistry::empty()
        });

    if plugin_registry.config_hook_count() > 0 {
        tracing::debug!(
            hooks = plugin_registry.config_hook_count(),
            "running config.mutate pipeline"
        );
    }

    let raw_before = raw.clone();
    let raw = plugin_registry.mutate_config(&raw);

    if raw != raw_before {
        tracing::debug!("configuration updated by plugin config.mutate hooks");
    }

    tracing::debug!(path = config_path, "applying configuration");

    let inner = match crate::config::Config::parse_preview(&raw) {
        Ok(inner) => inner,
        Err(err) => {
            eprintln!("failed to parse config after plugin mutation: {err:#?}");
            return Err(1);
        }
    };

    let (config, _guard) =
        match crate::config::Config::open_from_inner(inner, config_path, debug, false) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("failed to apply configuration: {err:#?}");
                return Err(1);
            }
        };
    tracing::debug!(path = config_path, "configuration loaded");

    plugin_registry.log_startup_summary();
    plugin_registry.emit(featherfly_plugin_sdk::PluginEvent::ConfigLoaded, &raw);
    tracing::debug!("emitting daemon.starting lifecycle event");
    plugin_registry.emit(featherfly_plugin_sdk::PluginEvent::DaemonStarting, b"");

    let state = Arc::new(crate::routes::AppState {
        start_time: std::time::Instant::now(),
        container_type: match std::env::var("OCI_CONTAINER").as_deref() {
            Ok("official") => crate::routes::AppContainerType::Official,
            Ok(_) => crate::routes::AppContainerType::Unknown,
            Err(_) => crate::routes::AppContainerType::None,
        },
        version: crate::full_version(),
        config: Arc::clone(&config),
        plugins: plugin_registry,
    });

    let app = OpenApiRouter::new()
        .merge(crate::routes::router(&state))
        .merge(crate::plugins::routes::router(&state))
        .fallback(|_req: Request| async move {
            ApiResponse::error("route not found")
                .with_status(StatusCode::NOT_FOUND)
                .into_response()
        })
        .layer(axum::middleware::from_fn(handle_request))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::plugins::request_middleware::inject_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::plugins::request_middleware::intercept_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::plugins::middleware::response_middleware,
        ))
        .with_state(state.clone());

    tracing::debug!(
        plugin_routes = state.plugins.routes().len(),
        config_hooks = state.plugins.config_hook_count(),
        "HTTP middleware stack ready"
    );

    let (mut router, openapi) = app.split_for_parts();
    let openapi = crate::api_spec::finalize_openapi(openapi, &config.load().app_name);

    if !config.load().api.disable_openapi_docs {
        router = router.route(
            "/openapi.json",
            axum::routing::get({
                let openapi = openapi.clone();
                move || async move { axum::Json(openapi) }
            }),
        );
    }

    if let Ok(host) = config.load().api.host.parse::<std::net::IpAddr>() {
        let address = std::net::SocketAddr::from((host, config.load().api.port));

        tracing::info!(
            address = %address,
            plugins = state.plugins.summaries().len(),
            hooks = state.plugins.hook_count(),
            plugin_routes = state.plugins.routes().len(),
            openapi = !config.load().api.disable_openapi_docs,
            "FeatherFly ready"
        );

        if !config.load().api.disable_openapi_docs {
            tracing::debug!(
                schema = %format!("http://{address}/openapi.json"),
                docs = crate::DOCS_WEBSITE,
                "documentation endpoints"
            );
        }

        state.plugins.emit(
            featherfly_plugin_sdk::PluginEvent::DaemonStarted,
            address.to_string().as_bytes(),
        );

        if config.load().updates.check_on_startup
            && config.load().updates.channel != crate::update::UpdateChannel::Disabled
        {
            let channel = config.load().updates.channel;
            tokio::spawn(async move {
                match crate::update::check_update(channel).await {
                    Ok(status) if status.update_available => {
                        tracing::info!(
                            channel = ?status.channel,
                            latest_version = ?status.latest_version,
                            latest_commit = ?status.latest_commit,
                            download_url = ?status.download_url,
                            "a newer FeatherFly build is available on GitHub"
                        );
                    }
                    Ok(_) => {}
                    Err(err) => {
                        tracing::debug!("startup update check failed: {err:#}");
                    }
                }
            });
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
                    return Err(1);
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
                return Err(1);
            }
        }
    } else {
        tracing::error!("invalid api.host configured: {}", config.load().api.host);
        return Err(1);
    }

    crate::config::Config::remove_pid_file(&config.load());
    state.plugins.shutdown_all();

    Ok(())
}
