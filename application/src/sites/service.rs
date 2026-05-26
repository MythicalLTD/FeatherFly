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
    eggs::{ConfigFileContext, EggRuntime, apply_process_config, load_egg, run_install_script},
    ftp::FtpService,
    mail::MailService,
    proxy::{RedirectSpec, SiteProxySpec, build_redirect_labels, build_site_labels},
    routes::State,
    sites::{
        SiteRedirect,
        log_store::{self, LogArchiveEntry},
        routing::{build_routing_hosts, site_routing_hosts},
        state::{self, SITE_STATUS_ACTIVE, SITE_STATUS_SUSPENDED, SiteType},
    },
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
    #[serde(alias = "template")]
    pub egg_id: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
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
    #[serde(default)]
    pub document_root: Option<String>,
    #[serde(default)]
    pub run_install: bool,
    #[serde(default)]
    pub redirects: Vec<SiteRedirect>,
    #[serde(default)]
    pub preview_domain: Option<String>,
    #[serde(default)]
    pub wildcard_domain: bool,
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
    pub egg_id: String,
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
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_root: Option<String>,
    pub site_type: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub redirects: Vec<SiteRedirect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_domain: Option<String>,
    #[serde(default)]
    pub wildcard_domain: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateSiteRequest {
    pub name: Option<String>,
    pub domain: Option<String>,
    #[serde(alias = "template")]
    pub egg_id: Option<String>,
    #[serde(default)]
    pub domains: Option<Vec<String>>,
    pub git_repo: Option<String>,
    pub memory_mb: Option<u64>,
    pub cpu_quota: Option<i64>,
    pub disk_quota_mb: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
    pub document_root: Option<String>,
    pub site_type: Option<String>,
    #[serde(default)]
    pub variables: Option<HashMap<String, String>>,
    #[serde(default)]
    pub redirects: Option<Vec<SiteRedirect>>,
    pub preview_domain: Option<String>,
    pub wildcard_domain: Option<bool>,
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

        let egg = load_egg(&inner.hosting.eggs_directory, &req.egg_id)?;
        let mut runtime = egg.resolve(&req.variables)?;
        runtime.env.extend(req.env.clone());
        apply_document_root_env(&mut runtime.env, req.document_root.as_deref());

        let volume_name = format!("site-vol-{}", req.id);
        let container_name = format!("site-{}", req.id);
        let site_type = egg
            .features
            .first()
            .cloned()
            .unwrap_or_else(|| SiteType::Custom.as_str().to_string());
        let routing_domains = build_routing_hosts(
            &req.domain,
            &req.domains,
            req.preview_domain.as_deref(),
            req.wildcard_domain,
        );

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
            docker.pull_image(&runtime.image).await?;
            if let Some(install) = &egg.install {
                docker.pull_image(&install.container_image).await.ok();
            }
        }

        if req.run_install
            && let Some(install) = &egg.install
        {
            run_install_script(
                docker,
                &network,
                &volume_name,
                &req.id,
                install,
                &runtime.env,
            )
            .await?;
        }

        if let Some(config) = &egg.config {
            apply_process_config(
                &volume_name,
                config,
                &ConfigFileContext {
                    site_id: &req.id,
                    port: runtime.port,
                    env: &runtime.env,
                },
            )
            .await?;
        }

        let alias_domains = req.domains.clone();
        let mut labels = base_site_labels(&req.id, &req.domain);
        if inner.proxy.enabled && state::site_is_active(SITE_STATUS_ACTIVE) {
            let proxy_labels = build_site_labels(
                &inner.proxy,
                &SiteProxySpec {
                    site_id: req.id.clone(),
                    domains: routing_domains.clone(),
                    port: runtime.port,
                    path_prefix: None,
                },
            );
            labels.extend(proxy_labels);
            plugin_events::emit_state_event(
                state,
                PluginEvent::ProxyRegistered,
                &ProxyRegisteredPayload {
                    site_id: &req.id,
                    domains: &routing_domains,
                    provider: &inner.proxy.provider,
                },
            );
            if inner.proxy.tls_enabled {
                plugin_events::emit_state_event(
                    state,
                    PluginEvent::SslProvisioned,
                    &SslProvisionedPayload {
                        site_id: &req.id,
                        domains: &routing_domains,
                        cert_resolver: &inner.proxy.cert_resolver,
                    },
                );
            }
        }

        let container_id = create_site_container_internal(
            docker,
            &inner,
            &container_name,
            &runtime,
            &network,
            &volume_name,
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
            egg_id: req.egg_id.clone(),
            container_id: Some(container_id.clone()),
            network: Some(network.clone()),
            volume: Some(volume_name.clone()),
            git_repo: req.git_repo.clone(),
            memory_mb: req.memory_mb.map(|m| m as i64),
            cpu_quota: req.cpu_quota,
            disk_quota_mb: req.disk_quota_mb,
            bandwidth_mbps: req.bandwidth_mbps,
            status: SITE_STATUS_ACTIVE.into(),
            domains: alias_domains,
            document_root: req.document_root.clone(),
            site_type,
            variables: req.variables.clone(),
            redirects: req.redirects.clone(),
            preview_domain: req.preview_domain.clone(),
            wildcard_domain: req.wildcard_domain,
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
                egg_id: &req.egg_id,
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

    pub async fn update(
        state: &State,
        site_id: &str,
        req: UpdateSiteRequest,
    ) -> Result<SiteSummary, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let mut site = cache.get_site(site_id).await?.context("site not found")?;

        let mut refresh_container = false;

        if let Some(name) = req.name {
            site.name = name;
        }
        if let Some(domain) = req.domain {
            if domain != site.domain {
                refresh_container = true;
            }
            site.domain = domain;
        }
        if let Some(egg_id) = req.egg_id {
            if egg_id != site.egg_id {
                refresh_container = true;
            }
            site.egg_id = egg_id;
        }
        if let Some(domains) = req.domains {
            if domains != site.domains {
                refresh_container = true;
            }
            site.domains = domains;
        }
        if let Some(git_repo) = req.git_repo {
            site.git_repo = Some(git_repo);
        }
        if let Some(memory_mb) = req.memory_mb {
            site.memory_mb = Some(memory_mb as i64);
            refresh_container = true;
        }
        if let Some(cpu_quota) = req.cpu_quota {
            site.cpu_quota = Some(cpu_quota);
            refresh_container = true;
        }
        if let Some(disk_quota_mb) = req.disk_quota_mb {
            site.disk_quota_mb = Some(disk_quota_mb);
        }
        if let Some(bandwidth_mbps) = req.bandwidth_mbps {
            site.bandwidth_mbps = Some(bandwidth_mbps);
        }
        if let Some(document_root) = req.document_root {
            if site.document_root.as_deref() != Some(document_root.as_str()) {
                refresh_container = true;
            }
            site.document_root = Some(document_root);
        }
        if let Some(site_type) = req.site_type {
            site.site_type = SiteType::parse(&site_type).as_str().to_string();
        }
        if let Some(variables) = req.variables {
            site.variables = variables;
            refresh_container = true;
        }
        if let Some(redirects) = req.redirects {
            if redirects != site.redirects {
                refresh_container = true;
            }
            site.redirects = redirects;
        }
        if let Some(preview_domain) = req.preview_domain {
            if site.preview_domain.as_deref() != Some(preview_domain.as_str()) {
                refresh_container = true;
            }
            site.preview_domain = Some(preview_domain);
        }
        if let Some(wildcard_domain) = req.wildcard_domain {
            if site.wildcard_domain != wildcard_domain {
                refresh_container = true;
            }
            site.wildcard_domain = wildcard_domain;
        }

        if refresh_container && state::site_is_active(&site.status) {
            let docker = state.docker.as_ref().context("docker unavailable")?;
            let container_id = recreate_site_container(state, docker, &inner, &site).await?;
            site.container_id = Some(container_id);
        }

        cache.save_site(&site).await?;
        Ok(record_to_summary(&site))
    }

    pub async fn suspend(state: &State, site_id: &str) -> Result<SiteSummary, anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let mut site = cache.get_site(site_id).await?.context("site not found")?;

        if site.status == SITE_STATUS_SUSPENDED {
            return Ok(record_to_summary(&site));
        }

        if let Some(cid) = &site.container_id {
            let _ = docker.stop_container(cid).await;
        }

        site.status = SITE_STATUS_SUSPENDED.into();
        cache.save_site(&site).await?;
        Ok(record_to_summary(&site))
    }

    pub async fn unsuspend(state: &State, site_id: &str) -> Result<SiteSummary, anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let mut site = cache.get_site(site_id).await?.context("site not found")?;

        if site.status == SITE_STATUS_ACTIVE {
            return Ok(record_to_summary(&site));
        }

        site.status = SITE_STATUS_ACTIVE.into();
        cache.save_site(&site).await?;

        if site.container_id.is_none() {
            let inner = state.config.load();
            let container_id = recreate_site_container(state, docker, &inner, &site).await?;
            site.container_id = Some(container_id);
            cache.save_site(&site).await?;
        } else if let Some(cid) = &site.container_id {
            docker.start_container(cid).await?;
            let inner = state.config.load();
            refresh_site_routing(state, docker, &inner, &site).await?;
        }

        Ok(record_to_summary(&site))
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
        Self::fetch_container_logs(state, site_id, tail, grep).await
    }

    pub async fn logs_download(
        state: &State,
        site_id: &str,
        tail: Option<String>,
    ) -> Result<(String, String), anyhow::Error> {
        let text = Self::fetch_container_logs(state, site_id, tail, None).await?;
        let filename = format!("{site_id}-logs.txt");
        Ok((filename, text))
    }

    pub async fn snapshot_logs(
        state: &State,
        site_id: &str,
    ) -> Result<LogArchiveEntry, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let _site = cache.get_site(site_id).await?.context("site not found")?;
        let text = Self::fetch_container_logs(state, site_id, Some("all".into()), None).await?;
        let entry = log_store::archive_log(&inner.system.data, site_id, &text)?;
        log_store::prune_archives(
            &inner.system.data,
            site_id,
            inner.hosting.site_log_retention,
        )?;
        Ok(entry)
    }

    pub async fn list_log_archives(
        state: &State,
        site_id: &str,
    ) -> Result<Vec<LogArchiveEntry>, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let _site = cache.get_site(site_id).await?.context("site not found")?;
        log_store::list_archives(&inner.system.data, site_id)
    }

    pub async fn read_log_archive(
        state: &State,
        site_id: &str,
        name: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let _site = cache.get_site(site_id).await?.context("site not found")?;
        log_store::read_archive(&inner.system.data, site_id, name)
    }

    pub async fn reinstall(state: &State, site_id: &str) -> Result<(), anyhow::Error> {
        let inner = state.config.load();
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let egg = load_egg(&inner.hosting.eggs_directory, &site.egg_id)?;
        let runtime = site_runtime_for_record(&inner, &site).await?;
        let install = egg.install.context("egg has no install script")?;
        let network = site.network.as_ref().context("site has no network")?;
        let volume = site.volume.as_ref().context("site has no volume")?;
        run_install_script(docker, network, volume, site_id, &install, &runtime.env).await?;
        if let Some(config) = &egg.config {
            apply_process_config(
                volume,
                config,
                &ConfigFileContext {
                    site_id,
                    port: runtime.port,
                    env: &runtime.env,
                },
            )
            .await?;
        }
        Ok(())
    }

    async fn fetch_container_logs(
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
    runtime: &EggRuntime,
    network: &str,
    volume: &str,
    labels: HashMap<String, String>,
    memory_mb: Option<u64>,
    cpu_quota: Option<i64>,
) -> Result<String, anyhow::Error> {
    let env: Vec<String> = runtime
        .env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();

    let mounts = vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(volume.to_string()),
        target: Some("/home/container".to_string()),
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

    let uses_bash_c =
        runtime.entrypoint.len() >= 2 && runtime.entrypoint.last().is_some_and(|part| part == "-c");
    let (entrypoint, cmd) = if uses_bash_c {
        (
            Some(runtime.entrypoint.clone()),
            Some(vec![runtime.startup.clone()]),
        )
    } else if runtime.entrypoint.is_empty() {
        (None, Some(vec![runtime.startup.clone()]))
    } else {
        (Some(runtime.entrypoint.clone()), None)
    };

    let config = Config {
        image: Some(runtime.image.clone()),
        env: Some(env),
        labels: Some(labels),
        working_dir: Some(runtime.workdir.clone()),
        entrypoint,
        cmd,
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
        egg_id: record.egg_id.clone(),
        container_id: record.container_id.clone(),
        network: record.network.clone(),
        volume: record.volume.clone(),
        memory_mb: record.memory_mb,
        cpu_quota: record.cpu_quota,
        disk_quota_mb: record.disk_quota_mb,
        bandwidth_mbps: record.bandwidth_mbps,
        status: record.status.clone(),
        domains: record.domains.clone(),
        document_root: record.document_root.clone(),
        site_type: record.site_type.clone(),
        variables: record.variables.clone(),
        redirects: record.redirects.clone(),
        preview_domain: record.preview_domain.clone(),
        wildcard_domain: record.wildcard_domain,
    }
}

fn base_site_labels(site_id: &str, domain: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site_id.into());
    labels.insert("featherfly.site.domain".into(), domain.into());
    labels
}

fn apply_document_root_env(env: &mut HashMap<String, String>, document_root: Option<&str>) {
    if let Some(root) = document_root.filter(|r| !r.is_empty()) {
        env.insert("FEATHERFLY_DOCUMENT_ROOT".into(), root.into());
    }
}

fn site_container_labels(
    inner: &InnerConfig,
    site: &SiteRecord,
    port: u16,
    include_proxy: bool,
) -> HashMap<String, String> {
    let mut labels = base_site_labels(&site.id, &site.domain);
    if include_proxy && inner.proxy.enabled && state::site_is_active(&site.status) {
        let domains = site_routing_hosts(site);
        let router_key = format!("site-{}", site.id);
        labels.extend(build_site_labels(
            &inner.proxy,
            &SiteProxySpec {
                site_id: site.id.clone(),
                domains,
                port,
                path_prefix: None,
            },
        ));
        let redirect_specs: Vec<RedirectSpec> = site
            .redirects
            .iter()
            .map(|r| RedirectSpec {
                from_host: r.from_host.clone(),
                to_url: r.to_url.clone(),
                permanent: r.permanent,
            })
            .collect();
        labels.extend(build_redirect_labels(
            &inner.proxy,
            &site.id,
            &router_key,
            &redirect_specs,
        ));
    }
    labels
}

async fn refresh_site_routing(
    state: &State,
    docker: &DockerManager,
    inner: &InnerConfig,
    site: &SiteRecord,
) -> Result<(), anyhow::Error> {
    if !inner.proxy.enabled || !state::site_is_active(&site.status) {
        return Ok(());
    }

    let domains = state::site_effective_domains(&site.domain, &site.domains);
    plugin_events::emit_state_event(
        state,
        PluginEvent::ProxyRegistered,
        &ProxyRegisteredPayload {
            site_id: &site.id,
            domains: &domains,
            provider: &inner.proxy.provider,
        },
    );
    let _ = docker;
    Ok(())
}

async fn site_runtime_for_record(
    inner: &InnerConfig,
    site: &SiteRecord,
) -> Result<EggRuntime, anyhow::Error> {
    let egg = load_egg(&inner.hosting.eggs_directory, &site.egg_id)?;
    let mut runtime = egg.resolve(&site.variables)?;
    apply_document_root_env(&mut runtime.env, site.document_root.as_deref());
    Ok(runtime)
}

async fn apply_egg_config_for_site(
    inner: &InnerConfig,
    site: &SiteRecord,
    volume: &str,
    runtime: &EggRuntime,
) -> Result<(), anyhow::Error> {
    let egg = load_egg(&inner.hosting.eggs_directory, &site.egg_id)?;
    if let Some(config) = &egg.config {
        apply_process_config(
            volume,
            config,
            &ConfigFileContext {
                site_id: &site.id,
                port: runtime.port,
                env: &runtime.env,
            },
        )
        .await?;
    }
    Ok(())
}

async fn recreate_site_container(
    state: &State,
    docker: &DockerManager,
    inner: &InnerConfig,
    site: &SiteRecord,
) -> Result<String, anyhow::Error> {
    let runtime = site_runtime_for_record(inner, site).await?;
    let network = site.network.as_ref().context("site has no network")?;
    let volume = site.volume.as_ref().context("site has no volume")?;
    let container_name = format!("site-{}", site.id);

    if let Some(old_id) = &site.container_id {
        let _ = docker.stop_container(old_id).await;
        docker.remove_container(old_id, true).await.ok();
    }

    if inner.hosting.auto_pull_images {
        docker.pull_image(&runtime.image).await?;
    }

    apply_egg_config_for_site(inner, site, volume, &runtime).await?;

    let labels = site_container_labels(inner, site, runtime.port, true);
    let container_id = create_site_container_internal(
        docker,
        inner,
        &container_name,
        &runtime,
        network,
        volume,
        labels,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
    )
    .await?;

    if inner.hosting.attach_proxy_network && inner.proxy.enabled {
        connect_container_network(docker.client(), &inner.proxy.network, &container_id).await?;
    }

    docker.start_container(&container_id).await?;
    refresh_site_routing(state, docker, inner, site).await?;
    Ok(container_id)
}

fn emit_volume_created(state: &State, volume: &str) {
    plugin_events::emit_state_event(
        state,
        PluginEvent::VolumeCreated,
        &VolumeCreatedPayload { name: volume },
    );
}
