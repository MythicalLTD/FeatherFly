use std::collections::HashMap;

use anyhow::Context;
use bollard::container::{Config, CreateContainerOptions};
use bollard::models::{Mount, MountTypeEnum};
use featherfly_plugin_sdk::PluginEvent;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    backups,
    cache::SiteRecord,
    config::InnerConfig,
    databases::DatabaseService,
    docker::{
        DockerManager, LogsRequest, NetworkEnsureResult, connect_container_network,
        ensure_site_network,
    },
    ftp::FtpService,
    mail::MailService,
    proxy::{SiteProxySpec, build_site_labels},
    routes::State,
    templates::{SiteTemplate, load_template},
    utils::plugin_events::{
        self, BackupCompletedPayload, BackupStartedPayload, DeployFinishedPayload,
        DeployStartedPayload, ProxyRegisteredPayload, SiteDestroyedPayload,
        SiteExecCompletedPayload, SiteIdPayload, SiteProvisionedPayload, SslProvisionedPayload,
        VolumeCreatedPayload,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ProvisionSiteRequest {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub template: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub git_repo: Option<String>,
    #[serde(default)]
    pub memory_mb: Option<u64>,
    #[serde(default)]
    pub cpu_quota: Option<i64>,
    #[serde(default)]
    pub disk_quota_mb: Option<i64>,
    #[serde(default)]
    pub bandwidth_mbps: Option<i64>,
    #[serde(default = "default_true")]
    pub start: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SiteSummary {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub template: String,
    pub container_id: Option<String>,
    pub network: Option<String>,
    pub volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_quota: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_quota_mb: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DeployRequest {
    #[serde(default)]
    pub git_ref: Option<String>,
    #[serde(default = "default_true")]
    pub rebuild: bool,
    #[serde(default)]
    pub zero_downtime: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SiteExecRequest {
    pub cmd: Vec<String>,
    #[serde(default)]
    pub shell: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SiteExecResponse {
    pub container_id: String,
    pub output: String,
}

pub struct SiteService;

impl SiteService {
    pub async fn provision(
        state: &State,
        req: ProvisionSiteRequest,
    ) -> Result<SiteSummary, anyhow::Error> {
        let inner = state.config.load();
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;

        let template = load_template(&inner.hosting.templates_directory, &req.template)?;
        let volume_name = format!("site-vol-{}", req.id);
        let container_name = format!("site-{}", req.id);

        docker.create_volume(&volume_name).await?;
        emit_volume_created(state, &volume_name);

        let (network, net_created) = ensure_site_network(docker.client(), &req.id).await?;
        if net_created == NetworkEnsureResult::Created {
            plugin_events::emit_state_event(
                state,
                PluginEvent::SiteNetworkCreated,
                &SiteIdPayload { id: &req.id },
            );
        }

        if inner.hosting.auto_pull_images {
            docker.pull_image(&template.image).await?;
        }

        let mut domains = req.domains;
        if domains.is_empty() {
            domains.push(req.domain.clone());
        }

        let mut labels = HashMap::new();
        labels.insert("featherfly.site.id".into(), req.id.clone());
        labels.insert("featherfly.site.domain".into(), req.domain.clone());

        if inner.proxy.enabled {
            let proxy_labels = build_site_labels(
                &inner.proxy,
                &SiteProxySpec {
                    site_id: req.id.clone(),
                    domains: domains.clone(),
                    port: template.port,
                    path_prefix: None,
                },
            );
            labels.extend(proxy_labels);
            plugin_events::emit_state_event(
                state,
                PluginEvent::ProxyRegistered,
                &ProxyRegisteredPayload {
                    site_id: &req.id,
                    domains: &domains,
                    provider: &inner.proxy.provider,
                },
            );
            if inner.proxy.tls_enabled {
                plugin_events::emit_state_event(
                    state,
                    PluginEvent::SslProvisioned,
                    &SslProvisionedPayload {
                        site_id: &req.id,
                        domains: &domains,
                        cert_resolver: &inner.proxy.cert_resolver,
                    },
                );
            }
        }

        let mut env = template.env.clone();
        env.extend(req.env.clone());

        let mount_target = if template.mount_path.is_empty() {
            template.workdir.clone()
        } else {
            template.mount_path.clone()
        };

        let container_id = create_site_container_internal(
            docker,
            &inner,
            &container_name,
            &template,
            &network,
            &volume_name,
            &mount_target,
            &env,
            labels,
            req.memory_mb,
            req.cpu_quota,
        )
        .await?;

        if inner.hosting.attach_proxy_network && inner.proxy.enabled {
            connect_container_network(docker.client(), &inner.proxy.network, &container_id).await?;
        }

        if req.start {
            docker.start_container(&container_id).await?;
        }

        let record = SiteRecord {
            id: req.id.clone(),
            name: req.name.clone(),
            domain: req.domain.clone(),
            template: req.template.clone(),
            container_id: Some(container_id.clone()),
            network: Some(network.clone()),
            volume: Some(volume_name.clone()),
            git_repo: req.git_repo.clone(),
            memory_mb: req.memory_mb.map(|m| m as i64),
            cpu_quota: req.cpu_quota,
            disk_quota_mb: req.disk_quota_mb,
            bandwidth_mbps: req.bandwidth_mbps,
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.save_site(&record).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::SiteProvisioned,
            &SiteProvisionedPayload {
                id: &req.id,
                name: &req.name,
                domain: &req.domain,
                template: &req.template,
                container_id: &container_id,
            },
        );

        if let Some(repo) = &req.git_repo {
            let _ = Self::deploy(
                state,
                &req.id,
                DeployRequest {
                    git_ref: Some(repo.clone()),
                    rebuild: false,
                    zero_downtime: false,
                },
            )
            .await;
        }

        Ok(record_to_summary(&record))
    }

    pub async fn destroy(state: &State, site_id: &str, force: bool) -> Result<(), anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;

        for db in cache.list_databases(site_id).await? {
            DatabaseService::remove(state, site_id, &db.id, force)
                .await
                .ok();
        }
        cache.delete_tasks_for_site(site_id).await.ok();
        cache.delete_dns_for_site(site_id).await.ok();
        MailService::teardown(state, site_id, force).await.ok();
        FtpService::teardown(state, site_id, force).await.ok();

        let inner = state.config.load();
        let zone_dir = std::path::Path::new(&inner.system.data)
            .join("dns")
            .join(site_id);
        if zone_dir.exists() {
            std::fs::remove_dir_all(zone_dir).ok();
        }

        if let Some(ref cid) = site.container_id {
            let _ = docker.stop_container(cid).await;
            docker.remove_container(cid, force).await?;
        }
        if let Some(ref vol) = site.volume {
            let _ = docker.remove_volume(vol, force).await;
            plugin_events::emit_state_event(
                state,
                PluginEvent::VolumeRemoved,
                &plugin_events::VolumeRemovedPayload { name: vol },
            );
        }

        cache.delete_site(site_id).await?;
        plugin_events::emit_state_event(
            state,
            PluginEvent::SiteDestroyed,
            &SiteDestroyedPayload { id: site_id },
        );
        Ok(())
    }

    pub async fn list(state: &State) -> Result<Vec<SiteSummary>, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        Ok(cache
            .list_sites()
            .await?
            .iter()
            .map(record_to_summary)
            .collect())
    }

    pub async fn get(state: &State, site_id: &str) -> Result<Option<SiteSummary>, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        Ok(cache
            .get_site(site_id)
            .await?
            .map(|r| record_to_summary(&r)))
    }

    pub async fn deploy(
        state: &State,
        site_id: &str,
        req: DeployRequest,
    ) -> Result<(), anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::DeployStarted,
            &DeployStartedPayload {
                site_id,
                git_ref: req.git_ref.as_deref(),
            },
        );

        let result = if req.zero_downtime {
            crate::deployments::blue_green::run_blue_green_deploy(
                state,
                &inner,
                &site,
                req.git_ref.as_deref(),
            )
            .await
        } else {
            crate::deployments::run_deploy(&inner, &site, req.git_ref.as_deref(), req.rebuild, true)
                .await
        };

        match &result {
            Ok(()) => plugin_events::emit_state_event(
                state,
                PluginEvent::DeployFinished,
                &DeployFinishedPayload {
                    site_id,
                    ok: true,
                    error: None,
                },
            ),
            Err(err) => {
                plugin_events::emit_state_event(
                    state,
                    PluginEvent::DeployFailed,
                    &DeployFinishedPayload {
                        site_id,
                        ok: false,
                        error: Some(err.to_string()),
                    },
                );
            }
        }

        result
    }

    pub async fn create_backup(state: &State, site_id: &str) -> Result<String, anyhow::Error> {
        let inner = state.config.load();
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::BackupStarted,
            &BackupStartedPayload { site_id },
        );

        match backups::create_backup(&inner, docker, cache, &site).await {
            Ok(result) => {
                plugin_events::emit_state_event(
                    state,
                    PluginEvent::BackupCompleted,
                    &BackupCompletedPayload {
                        site_id,
                        backup_id: &result.id,
                    },
                );
                Ok(result.id)
            }
            Err(err) => {
                plugin_events::emit_json(
                    &state.plugins,
                    PluginEvent::BackupFailed,
                    &serde_json::json!({ "site_id": site_id, "error": err.to_string() }),
                );
                Err(err)
            }
        }
    }

    pub async fn logs(
        state: &State,
        site_id: &str,
        tail: Option<String>,
        grep: Option<String>,
    ) -> Result<String, anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let cid = site.container_id.context("site has no container")?;

        let request = LogsRequest {
            follow: false,
            tail: tail.or_else(|| Some("200".into())),
            ..Default::default()
        };

        let mut stream = docker.container_logs_stream(&cid, request)?;
        let mut output = String::new();
        while let Some(chunk) = stream.next().await {
            output.push_str(&String::from_utf8_lossy(&chunk?));
        }

        if let Some(pattern) = grep.filter(|p| !p.is_empty()) {
            output = output
                .lines()
                .filter(|line| line.contains(&pattern))
                .collect::<Vec<_>>()
                .join("\n");
        }

        Ok(output)
    }

    pub async fn exec(
        state: &State,
        site_id: &str,
        req: SiteExecRequest,
    ) -> Result<SiteExecResponse, anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let cid = site.container_id.context("site has no container")?;

        let cmd = if req.shell {
            vec!["/bin/sh".into(), "-c".into(), req.cmd.join(" ")]
        } else if req.cmd.is_empty() {
            anyhow::bail!("cmd is required");
        } else {
            req.cmd
        };

        plugin_events::emit_state_event(
            state,
            PluginEvent::ContainerExecStarted,
            &plugin_events::ContainerExecStartedPayload {
                container_id: &cid,
                exec_id: "site-exec",
                tty: false,
            },
        );

        let output = docker.exec_run(&cid, cmd).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::SiteExecCompleted,
            &SiteExecCompletedPayload {
                site_id,
                container_id: &cid,
                exit_output_len: output.len(),
            },
        );

        Ok(SiteExecResponse {
            container_id: cid,
            output,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn create_site_container_internal(
    docker: &DockerManager,
    inner: &InnerConfig,
    name: &str,
    template: &SiteTemplate,
    network: &str,
    volume: &str,
    mount_target: &str,
    env: &HashMap<String, String>,
    labels: HashMap<String, String>,
    memory_mb: Option<u64>,
    cpu_quota: Option<i64>,
) -> Result<String, anyhow::Error> {
    let env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();

    let mounts = vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(volume.to_string()),
        target: Some(mount_target.to_string()),
        ..Default::default()
    }];

    let host_config = crate::docker::secure_host_config(
        network,
        inner.docker.container_pid_limit,
        memory_mb,
        cpu_quota,
        HashMap::new(),
    );
    let mut host_config = host_config;
    host_config.mounts = Some(mounts);

    let config = Config {
        image: Some(template.image.clone()),
        env: Some(env),
        labels: Some(labels),
        working_dir: Some(template.workdir.clone()),
        cmd: if template.startup_command.is_empty() {
            None
        } else {
            Some(template.startup_command.clone())
        },
        host_config: Some(host_config),
        ..Default::default()
    };

    let response = docker
        .client()
        .create_container(
            Some(CreateContainerOptions {
                name: name.to_string(),
                platform: None,
            }),
            config,
        )
        .await
        .with_context(|| format!("failed to create site container {name}"))?;

    Ok(response.id)
}

fn record_to_summary(record: &SiteRecord) -> SiteSummary {
    SiteSummary {
        id: record.id.clone(),
        name: record.name.clone(),
        domain: record.domain.clone(),
        template: record.template.clone(),
        container_id: record.container_id.clone(),
        network: record.network.clone(),
        volume: record.volume.clone(),
        memory_mb: record.memory_mb,
        cpu_quota: record.cpu_quota,
        disk_quota_mb: record.disk_quota_mb,
        bandwidth_mbps: record.bandwidth_mbps,
    }
}

fn emit_volume_created(state: &State, volume: &str) {
    plugin_events::emit_state_event(
        state,
        PluginEvent::VolumeCreated,
        &VolumeCreatedPayload { name: volume },
    );
}
