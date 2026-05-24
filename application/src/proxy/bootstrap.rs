use std::collections::HashMap;

use anyhow::Context;
use bollard::container::{Config, CreateContainerOptions, ListContainersOptions};
use bollard::models::{Mount, MountTypeEnum, PortBinding};

use crate::{
    config::InnerConfig,
    docker::DockerManager,
    hosting::{ensure_error_page_files, traefik_dynamic_directory, write_traefik_error_middleware},
};

pub const TRAEFIK_CONTAINER_NAME: &str = "featherfly-traefik";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraefikEnsureResult {
    Skipped,
    AlreadyRunning,
    Created,
    Restarted,
}

pub struct TraefikStatus {
    pub enabled: bool,
    pub auto_provision: bool,
    pub running: bool,
    pub container_id: Option<String>,
    pub image: Option<String>,
    pub network: String,
    pub tls_enabled: bool,
    pub cert_resolver: String,
}

pub async fn ensure_traefik(
    docker: &DockerManager,
    inner: &InnerConfig,
) -> Result<TraefikEnsureResult, anyhow::Error> {
    if !inner.proxy.enabled || inner.proxy.provider != "traefik" || !inner.proxy.auto_provision {
        return Ok(TraefikEnsureResult::Skipped);
    }

    if inner.proxy.tls_enabled && inner.proxy.acme_email.trim().is_empty() {
        tracing::info!(
            "Traefik starting in HTTP-only mode — set proxy.acme_email in config for automatic HTTPS"
        );
    }

    ensure_error_page_files(inner).ok();
    write_traefik_error_middleware(inner).ok();

    if inner.hosting.auto_update_images || inner.hosting.auto_pull_images {
        docker.pull_image(&inner.proxy.traefik_image).await.ok();
    }

    let existing = find_traefik_container(docker).await?;
    let had_existing = existing.is_some();
    if let Some(summary) = existing {
        if inner.hosting.auto_update_images {
            docker.stop_container(&summary.id).await.ok();
            docker.remove_container(&summary.id, true).await.ok();
        } else if summary.state == "running" {
            return Ok(TraefikEnsureResult::AlreadyRunning);
        } else {
            docker.start_container(&summary.id).await?;
            return Ok(TraefikEnsureResult::Restarted);
        }
    }

    docker.create_volume("featherfly-traefik-acme").await.ok();

    let http_port = inner.hosting.ports.http;
    let https_port = inner.hosting.ports.https;

    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        "80/tcp".into(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".into()),
            host_port: Some(http_port.to_string()),
        }]),
    );
    port_bindings.insert(
        "443/tcp".into(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".into()),
            host_port: Some(https_port.to_string()),
        }]),
    );

    let dynamic_dir = traefik_dynamic_directory(inner);
    std::fs::create_dir_all(&dynamic_dir).ok();
    let dynamic_bind = crate::docker::absolute_bind_path(&dynamic_dir)?;

    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        port_bindings,
    );
    host_config.cap_add = Some(vec!["NET_BIND_SERVICE".into()]);
    host_config.mounts = Some(vec![
        Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some("featherfly-traefik-acme".into()),
            target: Some("/acme".into()),
            ..Default::default()
        },
        Mount {
            typ: Some(MountTypeEnum::BIND),
            source: Some(dynamic_bind),
            target: Some("/etc/traefik/dynamic".into()),
            read_only: Some(true),
            ..Default::default()
        },
    ]);

    let mut labels = HashMap::new();
    labels.insert("featherfly.role".into(), "traefik".into());
    labels.insert("traefik.enable".into(), "true".into());

    let cmd = build_traefik_command(inner);

    let config = Config {
        image: Some(inner.proxy.traefik_image.clone()),
        cmd: Some(cmd),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    let response = docker
        .client()
        .create_container(
            Some(CreateContainerOptions {
                name: TRAEFIK_CONTAINER_NAME.to_string(),
                platform: None,
            }),
            config,
        )
        .await
        .context("failed to create Traefik container")?;

    crate::docker::connect_container_network(docker.client(), &inner.proxy.network, &response.id)
        .await
        .ok();

    docker.start_container(&response.id).await?;

    tracing::info!(
        container = TRAEFIK_CONTAINER_NAME,
        network = %inner.proxy.network,
        http_port = http_port,
        https_port = https_port,
        auto_update = inner.hosting.auto_update_images,
        "Traefik reverse proxy is running"
    );

    Ok(if had_existing {
        TraefikEnsureResult::Restarted
    } else {
        TraefikEnsureResult::Created
    })
}

pub async fn traefik_runtime_status(
    docker: Option<&DockerManager>,
    inner: &InnerConfig,
) -> TraefikStatus {
    let base = TraefikStatus {
        enabled: inner.proxy.enabled,
        auto_provision: inner.proxy.auto_provision,
        running: false,
        container_id: None,
        image: None,
        network: inner.proxy.network.clone(),
        tls_enabled: inner.proxy.tls_enabled,
        cert_resolver: inner.proxy.cert_resolver.clone(),
    };

    let Some(docker) = docker else {
        return base;
    };

    let Ok(Some(summary)) = find_traefik_container(docker).await else {
        return base;
    };

    TraefikStatus {
        running: summary.state == "running",
        container_id: Some(summary.id),
        image: Some(summary.image),
        ..base
    }
}

struct TraefikContainerSummary {
    id: String,
    state: String,
    image: String,
}

async fn find_traefik_container(
    docker: &DockerManager,
) -> Result<Option<TraefikContainerSummary>, anyhow::Error> {
    let mut filters = HashMap::new();
    filters.insert("name", vec![TRAEFIK_CONTAINER_NAME]);

    let list = docker
        .client()
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .context("failed to list containers for Traefik lookup")?;

    Ok(list.into_iter().find_map(|c| {
        let name = c
            .names
            .as_ref()
            .and_then(|names| names.first())
            .map(|n| n.trim_start_matches('/').to_string())?;
        if name != TRAEFIK_CONTAINER_NAME {
            return None;
        }
        Some(TraefikContainerSummary {
            id: c.id?,
            state: c.state.unwrap_or_default(),
            image: c.image.unwrap_or_else(|| "traefik".into()),
        })
    }))
}

fn build_traefik_command(inner: &InnerConfig) -> Vec<String> {
    let proxy = &inner.proxy;
    let http_port = inner.hosting.ports.http;
    let https_port = inner.hosting.ports.https;
    let mut args = vec![
        "--providers.docker=true".into(),
        "--providers.docker.exposedbydefault=false".into(),
        format!("--providers.docker.network={}", proxy.network),
        "--providers.file.directory=/etc/traefik/dynamic".into(),
        "--providers.file.watch=true".into(),
        format!(
            "--entrypoints.{}.address=:{}",
            proxy.entrypoint_http, http_port
        ),
        format!(
            "--entrypoints.{}.address=:{}",
            proxy.entrypoint_https, https_port
        ),
        "--log.level=INFO".into(),
        "--accesslog=true".into(),
        "--ping=true".into(),
    ];

    if proxy.routes_redirect_http() {
        args.push(format!(
            "--entrypoints.{}.http.redirections.entrypoint.to={}",
            proxy.entrypoint_http, proxy.entrypoint_https
        ));
        args.push(format!(
            "--entrypoints.{}.http.redirections.entrypoint.scheme=https",
            proxy.entrypoint_http
        ));
    }

    if proxy.acme_ready() {
        args.push(format!(
            "--certificatesresolvers.{}.acme.email={}",
            proxy.cert_resolver, proxy.acme_email
        ));
        args.push(format!(
            "--certificatesresolvers.{}.acme.storage=/acme/acme.json",
            proxy.cert_resolver
        ));
        args.push(format!(
            "--certificatesresolvers.{}.acme.httpchallenge.entrypoint={}",
            proxy.cert_resolver, proxy.entrypoint_http
        ));
    }

    if proxy.dashboard_enabled {
        args.push("--api.dashboard=true".into());
    }

    args
}
