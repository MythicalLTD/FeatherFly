use axum::{Router, extract::Request, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

use crate::utils::response::ApiResponse;

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
        eprintln!("failed to prepare daemon directories: {err:#?}");
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
            crate::plugins::PluginRegistry::with_load_error(
                plugins_dir.display().to_string(),
                err.to_string(),
            )
        });

    let raw_before = raw.clone();
    let raw = plugin_registry.mutate_config(&raw);
    if raw != raw_before {
        tracing::debug!("configuration updated by plugin config.mutate hooks");
        crate::utils::plugin_events::emit_json(
            &plugin_registry,
            featherfly_plugin_sdk::PluginEvent::PluginConfigMutated,
            &crate::utils::plugin_events::PluginConfigMutatedPayload {
                hook_count: plugin_registry.config_hook_count(),
                input_len: raw_before.len(),
                output_len: raw.len(),
            },
        );
    }

    let inner = match crate::config::Config::parse_preview(&raw) {
        Ok(inner) => inner,
        Err(err) => {
            eprintln!("failed to parse config after plugin mutation: {err:#?}");
            return Err(1);
        }
    };

    let (config, _guard, open_meta) =
        match crate::config::Config::open_from_inner(inner, config_path, debug, false) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("failed to apply configuration: {err:#?}");
                return Err(1);
            }
        };

    if open_meta.identity_generated {
        let inner = config.load();
        crate::utils::plugin_events::emit_json(
            &plugin_registry,
            featherfly_plugin_sdk::PluginEvent::NodeIdentityGenerated,
            &crate::utils::plugin_events::NodeIdentityPayload {
                uuid: &inner.uuid,
                token_id: &inner.token_id,
            },
        );
    }

    plugin_registry.log_startup_summary();
    plugin_registry.emit(featherfly_plugin_sdk::PluginEvent::ConfigLoaded, &raw);
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
        probe_guard: crate::middlewares::probe::ProbeGuard::new(),
    });

    let app = Router::new()
        .merge(crate::routes::router(&state))
        .merge(crate::plugins::routes::router(&state))
        .fallback(|_req: Request| async move {
            ApiResponse::error("route not found")
                .with_status(StatusCode::NOT_FOUND)
                .into_response()
        })
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middlewares::probe::middleware,
        ))
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
        .layer(axum::middleware::from_fn(
            crate::middlewares::request_id::middleware,
        ))
        .with_state(state.clone());

    let Ok(host) = config.load().api.host.parse::<std::net::IpAddr>() else {
        tracing::error!("invalid api.host configured: {}", config.load().api.host);
        return Err(1);
    };

    let address = std::net::SocketAddr::from((host, config.load().api.port));
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::AddrInUse {
                tracing::error!("failed to start http server (address already in use)");
            } else {
                tracing::error!("failed to start http server: {:?}", err);
            }
            return Err(1);
        }
    };

    tracing::info!(address = %address, "FeatherFly daemon ready");
    state.plugins.emit(
        featherfly_plugin_sdk::PluginEvent::DaemonStarted,
        address.to_string().as_bytes(),
    );

    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await;

    crate::config::Config::remove_pid_file(&config.load());
    state.plugins.shutdown_all();

    if let Err(err) = result {
        if err.kind() == std::io::ErrorKind::AddrInUse {
            tracing::error!("failed to start http server (address already in use)");
        } else {
            tracing::error!("failed to start http server: {:?}", err);
        }
        return Err(1);
    }

    Ok(())
}
