use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use bollard::container::{Config, CreateContainerOptions};
use bollard::models::{Mount, MountTypeEnum};
use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    cache::{FtpAccountRecord, FtpMetaRecord},
    config::InnerConfig,
    docker::DockerManager,
    routes::State,
    utils::plugin_events::{
        self, FtpAccountCreatedPayload, FtpAccountRemovedPayload, FtpGatewaySyncedPayload,
    },
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FtpProtocol {
    Sftp,
    Ftp,
    Both,
}

impl FtpProtocol {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sftp => "sftp",
            Self::Ftp => "ftp",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateFtpAccountRequest {
    pub id: String,
    pub username: String,
    pub password: String,
    pub protocol: FtpProtocol,
    #[serde(default = "default_home")]
    pub home_subpath: String,
}

fn default_home() -> String {
    "files".into()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FtpAccountSummary {
    pub id: String,
    pub site_id: String,
    pub username: String,
    pub protocol: String,
    pub home_subpath: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FtpSyncRequest {
    #[serde(default = "default_true")]
    pub reprovision: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FtpSyncResult {
    pub account_count: usize,
    pub sftp_container_id: Option<String>,
    pub ftp_container_id: Option<String>,
    pub sftp_port: i64,
    pub ftp_port: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FtpStatus {
    pub sftp_container_id: Option<String>,
    pub ftp_container_id: Option<String>,
    pub account_count: usize,
}

pub struct FtpService;

impl FtpService {
    pub async fn create_account(
        state: &State,
        site_id: &str,
        req: CreateFtpAccountRequest,
    ) -> Result<FtpAccountSummary, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        cache
            .get_site(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("site not found"))?;

        let record = FtpAccountRecord {
            id: req.id.clone(),
            site_id: site_id.to_string(),
            username: req.username.clone(),
            password: req.password.clone(),
            protocol: req.protocol.as_str().into(),
            home_subpath: req.home_subpath.clone(),
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.save_ftp_account(&record).await?;
        write_passwd_manifest(state, site_id).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::FtpAccountCreated,
            &FtpAccountCreatedPayload {
                site_id,
                account_id: &req.id,
                username: &req.username,
                protocol: req.protocol.as_str(),
            },
        );

        Ok(to_summary(&record))
    }

    pub async fn list_accounts(
        state: &State,
        site_id: &str,
    ) -> Result<Vec<FtpAccountSummary>, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        Ok(cache
            .list_ftp_accounts(site_id)
            .await?
            .iter()
            .map(to_summary)
            .collect())
    }

    pub async fn remove_account(
        state: &State,
        site_id: &str,
        account_id: &str,
    ) -> Result<(), anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let record = cache
            .get_ftp_account(site_id, account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("ftp account not found"))?;
        cache.delete_ftp_account(site_id, account_id).await?;
        write_passwd_manifest(state, site_id).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::FtpAccountRemoved,
            &FtpAccountRemovedPayload {
                site_id,
                account_id,
                username: &record.username,
            },
        );
        Ok(())
    }

    pub async fn status(state: &State, site_id: &str) -> Result<FtpStatus, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let meta = cache.get_ftp_meta(site_id).await?;
        let accounts = cache.list_ftp_accounts(site_id).await?;
        Ok(FtpStatus {
            sftp_container_id: meta.as_ref().and_then(|m| m.sftp_container_id.clone()),
            ftp_container_id: meta.as_ref().and_then(|m| m.ftp_container_id.clone()),
            account_count: accounts.len(),
        })
    }

    pub async fn sync(
        state: &State,
        site_id: &str,
        req: FtpSyncRequest,
    ) -> Result<FtpSyncResult, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let accounts = cache.list_ftp_accounts(site_id).await?;
        let volume = site.volume.as_ref().context("site has no volume")?;

        let mut meta = cache.get_ftp_meta(site_id).await?.unwrap_or(FtpMetaRecord {
            site_id: site_id.to_string(),
            sftp_container_id: None,
            ftp_container_id: None,
            updated_at: 0,
        });

        let needs_sftp = accounts
            .iter()
            .any(|a| a.protocol == "sftp" || a.protocol == "both");
        let needs_ftp = accounts
            .iter()
            .any(|a| a.protocol == "ftp" || a.protocol == "both");

        if req.reprovision {
            remove_gateway_containers(state, &meta).await.ok();
            meta.sftp_container_id = None;
            meta.ftp_container_id = None;
        }

        if needs_sftp {
            meta.sftp_container_id =
                Some(ensure_sftp_gateway(state, &inner, site_id, &site, volume, &accounts).await?);
        }

        if needs_ftp {
            meta.ftp_container_id =
                Some(ensure_ftp_gateway(state, &inner, site_id, &site, volume, &accounts).await?);
        }

        meta.updated_at = chrono::Utc::now().timestamp();
        cache.save_ftp_meta(&meta).await?;
        write_passwd_manifest(state, site_id).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::FtpGatewaySynced,
            &FtpGatewaySyncedPayload {
                site_id,
                account_count: accounts.len(),
            },
        );

        Ok(FtpSyncResult {
            account_count: accounts.len(),
            sftp_container_id: meta.sftp_container_id.clone(),
            ftp_container_id: meta.ftp_container_id.clone(),
            sftp_port: 22,
            ftp_port: 21,
        })
    }

    pub async fn teardown(state: &State, site_id: &str, _force: bool) -> Result<(), anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let inner = state.config.load();

        if let Some(meta) = cache.get_ftp_meta(site_id).await? {
            remove_gateway_containers(state, &meta).await.ok();
        }

        cache.delete_ftp_accounts_for_site(site_id).await.ok();
        cache.delete_ftp_meta(site_id).await.ok();

        let dir = Path::new(&inner.system.data).join("ftp").join(site_id);
        if dir.exists() {
            std::fs::remove_dir_all(dir).ok();
        }
        Ok(())
    }
}

async fn ensure_sftp_gateway(
    state: &State,
    inner: &InnerConfig,
    site_id: &str,
    site: &crate::cache::SiteRecord,
    volume: &str,
    accounts: &[FtpAccountRecord],
) -> Result<String, anyhow::Error> {
    if inner.hosting.shared_services {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let sites = cache.list_sites().await?;
        let all = cache.list_all_ftp_accounts().await?;
        return crate::hosting::shared::ensure_shared_sftp(state, inner, &sites, &all).await;
    }

    let docker = state.docker.as_ref().context("docker unavailable")?;
    let network = site.network.as_ref().context("site has no network")?;
    let container_name = format!("sftp-{site_id}");

    let sftp_users: Vec<String> = accounts
        .iter()
        .filter(|a| a.protocol != "ftp")
        .map(|a| format!("{}:{}:::{}", a.username, a.password, a.home_subpath))
        .collect();

    if sftp_users.is_empty() {
        anyhow::bail!("no SFTP accounts configured");
    }

    let mut mounts: Vec<Mount> = accounts
        .iter()
        .filter(|a| a.protocol != "ftp")
        .map(|a| Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume.to_string()),
            target: Some(format!("/home/{}/{}", a.username, a.home_subpath)),
            ..Default::default()
        })
        .collect();

    mounts.push(Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(volume.to_string()),
        target: Some("/var/www/site".into()),
        ..Default::default()
    });

    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site_id.to_string());
    labels.insert("featherfly.ftp.role".into(), "sftp".into());

    let sftp_ports = crate::hosting::sftp_port_bindings(&inner.hosting.ports);

    let mut host_config = crate::docker::secure_host_config(
        network,
        inner.docker.container_pid_limit,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
        sftp_ports,
    );
    host_config.mounts = Some(mounts);

    let config = Config {
        image: Some(inner.hosting.sftp_image.clone()),
        env: Some(vec![format!("SFTP_USERS={}", sftp_users.join(" "))]),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    recreate_gateway(docker, &container_name, config).await
}

async fn ensure_ftp_gateway(
    state: &State,
    inner: &InnerConfig,
    site_id: &str,
    site: &crate::cache::SiteRecord,
    volume: &str,
    accounts: &[FtpAccountRecord],
) -> Result<String, anyhow::Error> {
    if inner.hosting.shared_services {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let sites = cache.list_sites().await?;
        let all = cache.list_all_ftp_accounts().await?;
        return crate::hosting::shared::ensure_shared_ftp(state, inner, &sites, &all).await;
    }

    let docker = state.docker.as_ref().context("docker unavailable")?;
    let network = site.network.as_ref().context("site has no network")?;
    let container_name = format!("ftp-{site_id}");

    let ftp_accounts: Vec<_> = accounts
        .iter()
        .filter(|a| a.protocol == "ftp" || a.protocol == "both")
        .collect();

    if ftp_accounts.is_empty() {
        anyhow::bail!("no FTP accounts configured");
    }

    let primary = &ftp_accounts[0];
    let mounts = vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(volume.to_string()),
        target: Some("/home/ftp".into()),
        ..Default::default()
    }];

    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site_id.to_string());
    labels.insert("featherfly.ftp.role".into(), "ftp".into());

    let ftp_ports = crate::hosting::ftp_port_bindings(&inner.hosting.ports);

    let mut host_config = crate::docker::secure_host_config(
        network,
        inner.docker.container_pid_limit,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
        ftp_ports,
    );
    host_config.mounts = Some(mounts);

    let env = vec![
        format!("FTP_USER_NAME={}", primary.username),
        format!("FTP_USER_PASS={}", primary.password),
        "FTP_USER_HOME=/home/ftp".into(),
        "ADDED_FLAGS=-O w3c,/var/www/site".into(),
        format!(
            "FTP_PASSIVE_PORTS={}:{}",
            inner.hosting.ports.ftp_passive_min, inner.hosting.ports.ftp_passive_max
        ),
    ];

    let config = Config {
        image: Some(inner.hosting.ftp_image.clone()),
        env: Some(env),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    recreate_gateway(docker, &container_name, config).await
}

async fn recreate_gateway(
    docker: &DockerManager,
    name: &str,
    config: bollard::container::Config<String>,
) -> Result<String, anyhow::Error> {
    let _ = docker.stop_container(name).await;
    docker.remove_container(name, true).await.ok();

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
        .with_context(|| format!("failed to create gateway container {name}"))?;

    docker.start_container(&response.id).await?;
    Ok(response.id)
}

async fn remove_gateway_containers(
    state: &State,
    meta: &FtpMetaRecord,
) -> Result<(), anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    if let Some(cid) = &meta.sftp_container_id {
        let _ = docker.stop_container(cid).await;
        docker.remove_container(cid, true).await.ok();
    }
    if let Some(cid) = &meta.ftp_container_id {
        let _ = docker.stop_container(cid).await;
        docker.remove_container(cid, true).await.ok();
    }
    Ok(())
}

async fn write_passwd_manifest(state: &State, site_id: &str) -> Result<(), anyhow::Error> {
    let inner = state.config.load();
    let cache = state.cache.as_ref().context("cache unavailable")?;
    let accounts = cache.list_ftp_accounts(site_id).await?;
    let dir = Path::new(&inner.system.data).join("ftp").join(site_id);
    std::fs::create_dir_all(&dir)?;

    let manifest: Vec<_> = accounts
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "username": a.username,
                "protocol": a.protocol,
                "home_subpath": a.home_subpath,
            })
        })
        .collect();
    std::fs::write(
        dir.join("accounts.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

fn to_summary(record: &FtpAccountRecord) -> FtpAccountSummary {
    FtpAccountSummary {
        id: record.id.clone(),
        site_id: record.site_id.clone(),
        username: record.username.clone(),
        protocol: record.protocol.clone(),
        home_subpath: record.home_subpath.clone(),
    }
}
