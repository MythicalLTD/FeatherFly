use axum::{extract::Request, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

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

    let (config, _guard, open_meta) =
        match crate::config::Config::open_from_inner(inner, config_path, debug, false) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("failed to apply configuration: {err:#?}");
                return Err(1);
            }
        };
    tracing::debug!(path = config_path, "configuration loaded");

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

    if open_meta.config_upgraded {
        crate::utils::plugin_events::emit_json(
            &plugin_registry,
            featherfly_plugin_sdk::PluginEvent::ConfigUpgraded,
            &crate::utils::plugin_events::ConfigUpgradedPayload { path: config_path },
        );
    }

    if let Err(err) =
        crate::templates::ensure_builtin_templates(&config.load().hosting.templates_directory)
    {
        tracing::warn!(%err, "failed to install builtin site templates");
    }

    plugin_registry.log_startup_summary();
    plugin_registry.emit(featherfly_plugin_sdk::PluginEvent::ConfigLoaded, &raw);
    tracing::debug!("emitting daemon.starting lifecycle event");
    plugin_registry.emit(featherfly_plugin_sdk::PluginEvent::DaemonStarting, b"");

    let docker = if config.load().docker.enabled {
        let socket = config.load().docker.socket.clone();
        match crate::docker::bootstrap_system(&config.load()).await {
            Ok(Some(result)) => {
                let api_version = result
                    .manager
                    .version()
                    .await
                    .ok()
                    .and_then(|v| v.api_version);
                crate::utils::plugin_events::emit_json(
                    &plugin_registry,
                    featherfly_plugin_sdk::PluginEvent::DockerReady,
                    &crate::utils::plugin_events::DockerReadyPayload {
                        socket: &socket,
                        network: &config.load().docker.network.name,
                        network_created: result.summary.docker_network
                            == crate::docker::NetworkEnsureResult::Created,
                        api_version,
                    },
                );
                tracing::info!(
                    socket = %socket,
                    network = %config.load().docker.network.name,
                    traefik = ?result.summary.traefik,
                    docker_installed = result.summary.docker_installed,
                    "docker bootstrap complete"
                );
                if result.summary.traefik != crate::proxy::TraefikEnsureResult::Skipped {
                    let inner = config.load();
                    crate::utils::plugin_events::emit_json(
                        &plugin_registry,
                        featherfly_plugin_sdk::PluginEvent::TraefikProvisioned,
                        &crate::utils::plugin_events::TraefikProvisionedPayload {
                            result: crate::proxy::traefik_result_label(result.summary.traefik),
                            image: &inner.proxy.traefik_image,
                            network: &inner.proxy.network,
                            tls_enabled: inner.proxy.tls_enabled,
                        },
                    );
                }
                Some(result.manager)
            }
            Ok(None) => {
                if config.load().docker.enabled {
                    crate::utils::plugin_events::emit_json(
                        &plugin_registry,
                        featherfly_plugin_sdk::PluginEvent::DockerUnavailable,
                        &crate::utils::plugin_events::DockerUnavailablePayload {
                            socket: &socket,
                            error: "docker engine not reachable".into(),
                        },
                    );
                }
                None
            }
            Err(err) => {
                crate::utils::plugin_events::emit_json(
                    &plugin_registry,
                    featherfly_plugin_sdk::PluginEvent::DockerUnavailable,
                    &crate::utils::plugin_events::DockerUnavailablePayload {
                        socket: &socket,
                        error: err.to_string(),
                    },
                );
                tracing::warn!(%err, "docker bootstrap failed — container APIs disabled");
                None
            }
        }
    } else {
        tracing::debug!("docker integration disabled in configuration");
        None
    };

    let cache = match crate::cache::Cache::open(&config.load().system.data).await {
        Ok(cache) => Some(Arc::new(cache)),
        Err(err) => {
            tracing::warn!(%err, "sqlite cache unavailable — panel idempotency disabled");
            None
        }
    };

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
        docker,
        panel: arc_swap::ArcSwapOption::from(None),
        cache,
    });

    let panel_handle = crate::websocket::spawn_panel_client(Arc::clone(&state));
    state.panel.store(Some(Arc::new(panel_handle)));

    let reconcile = crate::hosting::reconcile_on_startup(&state).await;
    tracing::info!(
        sites = reconcile.sites_total,
        started = reconcile.sites_started,
        mail = reconcile.mail_synced,
        ftp = reconcile.ftp_synced,
        shared = reconcile.shared_stack,
        "startup hosting reconcile complete"
    );

    crate::scheduler::spawn_scheduler(Arc::clone(&state));

    let app = OpenApiRouter::new()
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
            && config.load().updates.channel != crate::utils::update::UpdateChannel::Disabled
        {
            let channel = config.load().updates.channel;
            tokio::spawn(async move {
                match crate::utils::update::check_update(channel).await {
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
