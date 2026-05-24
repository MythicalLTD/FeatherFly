use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use bollard::container::{Config, CreateContainerOptions};
use bollard::models::{Mount, MountTypeEnum};
use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    cache::{MailAccountRecord, MailMetaRecord},
    config::InnerConfig,
    docker::{DockerManager, connect_container_network},
    proxy::{ServiceRouteSpec, build_service_labels},
    routes::State,
    utils::plugin_events::{
        self, MailAccountCreatedPayload, MailAccountRemovedPayload, MailSyncedPayload,
        RoundcubeProvisionedPayload,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateMailAccountRequest {
    pub id: String,
    pub address: String,
    pub password: String,
    #[serde(default)]
    pub quota_mb: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailAccountSummary {
    pub id: String,
    pub site_id: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_mb: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MailSyncRequest {
    #[serde(default = "default_true")]
    pub provision_mailserver: bool,
    #[serde(default)]
    pub provision_roundcube: bool,
    #[serde(default)]
    pub mail_domain: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecommendedDnsRecord {
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub value: String,
    pub ttl: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailSyncResult {
    pub account_count: usize,
    pub mail_host: String,
    pub webmail_host: Option<String>,
    pub recommended_dns: Vec<RecommendedDnsRecord>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailStatus {
    pub mail_domain: Option<String>,
    pub mailserver_container_id: Option<String>,
    pub roundcube_container_id: Option<String>,
    pub account_count: usize,
}

pub struct MailService;

impl MailService {
    pub async fn create_account(
        state: &State,
        site_id: &str,
        req: CreateMailAccountRequest,
    ) -> Result<MailAccountSummary, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        cache
            .get_site(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("site not found"))?;

        let record = MailAccountRecord {
            id: req.id.clone(),
            site_id: site_id.to_string(),
            address: req.address.clone(),
            password: req.password.clone(),
            quota_mb: req.quota_mb,
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.save_mail_account(&record).await?;

        if let Some(meta) = cache.get_mail_meta(site_id).await?
            && let Some(cid) = meta.mailserver_container_id
        {
            let docker = state.docker.as_ref().context("docker unavailable")?;
            register_mailbox(docker, &cid, &req.address, &req.password)
                .await
                .ok();
        }

        write_manifest(state, site_id).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::MailAccountCreated,
            &MailAccountCreatedPayload {
                site_id,
                account_id: &req.id,
                address: &req.address,
            },
        );

        Ok(to_summary(&record))
    }

    pub async fn list_accounts(
        state: &State,
        site_id: &str,
    ) -> Result<Vec<MailAccountSummary>, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        Ok(cache
            .list_mail_accounts(site_id)
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
            .get_mail_account(site_id, account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mail account not found"))?;

        if let Some(meta) = cache.get_mail_meta(site_id).await?
            && let Some(cid) = meta.mailserver_container_id
        {
            let docker = state.docker.as_ref().context("docker unavailable")?;
            docker
                .exec_run(
                    &cid,
                    vec![
                        "setup".into(),
                        "email".into(),
                        "del".into(),
                        record.address.clone(),
                    ],
                )
                .await
                .ok();
        }

        cache.delete_mail_account(site_id, account_id).await?;
        write_manifest(state, site_id).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::MailAccountRemoved,
            &MailAccountRemovedPayload {
                site_id,
                account_id,
                address: &record.address,
            },
        );
        Ok(())
    }

    pub async fn status(state: &State, site_id: &str) -> Result<MailStatus, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let meta = cache.get_mail_meta(site_id).await?;
        let accounts = cache.list_mail_accounts(site_id).await?;
        Ok(MailStatus {
            mail_domain: meta.as_ref().and_then(|m| m.mail_domain.clone()),
            mailserver_container_id: meta
                .as_ref()
                .and_then(|m| m.mailserver_container_id.clone()),
            roundcube_container_id: meta.as_ref().and_then(|m| m.roundcube_container_id.clone()),
            account_count: accounts.len(),
        })
    }

    pub async fn sync(
        state: &State,
        site_id: &str,
        req: MailSyncRequest,
    ) -> Result<MailSyncResult, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;
        let mail_domain = req
            .mail_domain
            .clone()
            .unwrap_or_else(|| site.domain.clone());
        let mail_host = format!("{}.{}", inner.hosting.mail_subdomain, mail_domain);
        let webmail_host = if req.provision_roundcube {
            Some(format!(
                "{}.{}",
                inner.hosting.webmail_subdomain, mail_domain
            ))
        } else {
            None
        };

        let mut meta = cache
            .get_mail_meta(site_id)
            .await?
            .unwrap_or(MailMetaRecord {
                site_id: site_id.to_string(),
                mailserver_container_id: None,
                mailserver_volume: None,
                roundcube_container_id: None,
                mail_domain: None,
                updated_at: 0,
            });

        meta.mail_domain = Some(mail_domain.clone());
        meta.updated_at = chrono::Utc::now().timestamp();

        if req.provision_mailserver {
            meta.mailserver_container_id = Some(
                ensure_mailserver(state, &inner, site_id, &site, &mail_domain, &mut meta).await?,
            );
        }

        if req.provision_roundcube {
            let mailserver_name = format!("mail-{site_id}");
            meta.roundcube_container_id = Some(
                ensure_roundcube(
                    state,
                    &inner,
                    site_id,
                    &site,
                    &mailserver_name,
                    webmail_host.as_deref().context("webmail host missing")?,
                )
                .await?,
            );
            plugin_events::emit_state_event(
                state,
                PluginEvent::RoundcubeProvisioned,
                &RoundcubeProvisionedPayload {
                    site_id,
                    webmail_host: webmail_host.as_deref().unwrap_or(""),
                },
            );
        }

        cache.save_mail_meta(&meta).await?;

        let accounts = cache.list_mail_accounts(site_id).await?;
        if let Some(cid) = &meta.mailserver_container_id {
            let docker = state.docker.as_ref().context("docker unavailable")?;
            for account in &accounts {
                register_mailbox(docker, cid, &account.address, &account.password)
                    .await
                    .ok();
            }
        }

        write_manifest(state, site_id).await?;

        let recommended_dns = recommended_mail_dns(&mail_host, &mail_domain);

        plugin_events::emit_state_event(
            state,
            PluginEvent::MailSynced,
            &MailSyncedPayload {
                site_id,
                account_count: accounts.len(),
                mail_host: &mail_host,
            },
        );

        Ok(MailSyncResult {
            account_count: accounts.len(),
            mail_host,
            webmail_host,
            recommended_dns,
        })
    }

    pub async fn teardown(state: &State, site_id: &str, force: bool) -> Result<(), anyhow::Error> {
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let inner = state.config.load();

        if let Some(meta) = cache.get_mail_meta(site_id).await? {
            if let Some(cid) = meta.mailserver_container_id {
                let _ = docker.stop_container(&cid).await;
                docker.remove_container(&cid, force).await.ok();
            }
            if let Some(cid) = meta.roundcube_container_id {
                let _ = docker.stop_container(&cid).await;
                docker.remove_container(&cid, force).await.ok();
            }
            if let Some(vol) = meta.mailserver_volume {
                let _ = docker.remove_volume(&vol, force).await;
            }
        }

        cache.delete_mail_accounts_for_site(site_id).await.ok();
        cache.delete_mail_meta(site_id).await.ok();

        let mail_dir = Path::new(&inner.system.data).join("mail").join(site_id);
        if mail_dir.exists() {
            std::fs::remove_dir_all(mail_dir).ok();
        }
        Ok(())
    }
}

async fn ensure_mailserver(
    state: &State,
    inner: &InnerConfig,
    site_id: &str,
    site: &crate::cache::SiteRecord,
    mail_domain: &str,
    meta: &mut MailMetaRecord,
) -> Result<String, anyhow::Error> {
    if inner.hosting.shared_services {
        return crate::hosting::shared::ensure_shared_mailserver(state, inner).await;
    }

    if let Some(cid) = &meta.mailserver_container_id
        && docker_container_running(state, cid).await.unwrap_or(false)
    {
        return Ok(cid.clone());
    }

    let docker = state.docker.as_ref().context("docker unavailable")?;
    let network = site.network.as_ref().context("site has no network")?;
    let volume_name = format!("mail-vol-{site_id}");
    docker.create_volume(&volume_name).await?;
    meta.mailserver_volume = Some(volume_name.clone());

    let container_name = format!("mail-{site_id}");
    let mail_host = format!("{}.{}", inner.hosting.mail_subdomain, mail_domain);

    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site_id.to_string());
    labels.insert("featherfly.mail.role".into(), "mailserver".into());
    // SMTP/IMAP are exposed on the site Docker network — not via HTTP Traefik routes.

    let port_bindings = crate::hosting::mail_port_bindings(&inner.hosting.ports);

    let mut host_config = crate::docker::secure_host_config(
        network,
        inner.docker.container_pid_limit,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
        port_bindings,
    );
    host_config.cap_add = Some(vec!["NET_ADMIN".into(), "SYS_PTRACE".into()]);
    host_config.mounts = Some(vec![
        Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(format!("{volume_name}-config")),
            target: Some("/tmp/docker-mailserver".into()),
            ..Default::default()
        },
        Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume_name.clone()),
            target: Some("/var/mail".into()),
            ..Default::default()
        },
    ]);

    docker
        .create_volume(&format!("{volume_name}-config"))
        .await?;

    let env = vec![
        format!("HOSTNAME={mail_host}"),
        format!("DOMAINNAME={mail_domain}"),
        "ENABLE_SPAMASSASSIN=0".into(),
        "ENABLE_CLAMAV=0".into(),
        "ENABLE_FAIL2BAN=0".into(),
        "ONE_DIR=1".into(),
        "SSL_TYPE=".into(),
        "PERMIT_DOCKER=network".into(),
    ];

    let config = Config {
        image: Some(inner.hosting.mailserver_image.clone()),
        hostname: Some(mail_host.clone()),
        env: Some(env),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    let response = docker
        .client()
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.clone(),
                platform: None,
            }),
            config,
        )
        .await
        .with_context(|| format!("failed to create mailserver {container_name}"))?;

    if inner.hosting.attach_proxy_network && inner.proxy.enabled {
        connect_container_network(docker.client(), &inner.proxy.network, &response.id).await?;
    }

    docker.start_container(&response.id).await?;
    Ok(response.id)
}

async fn ensure_roundcube(
    state: &State,
    inner: &InnerConfig,
    site_id: &str,
    site: &crate::cache::SiteRecord,
    imap_host: &str,
    webmail_host: &str,
) -> Result<String, anyhow::Error> {
    if inner.hosting.shared_services {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let sites = cache.list_sites().await?;
        let accounts = cache.list_all_mail_accounts().await?;
        return crate::hosting::shared::ensure_shared_roundcube(state, inner, &sites, &accounts)
            .await;
    }

    let docker = state.docker.as_ref().context("docker unavailable")?;
    let network = site.network.as_ref().context("site has no network")?;
    let container_name = format!("roundcube-{site_id}");

    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site_id.to_string());
    labels.insert("featherfly.mail.role".into(), "roundcube".into());
    if inner.proxy.enabled {
        labels.extend(build_service_labels(
            &inner.proxy,
            &ServiceRouteSpec {
                site_id: site_id.to_string(),
                role: "webmail".into(),
                hosts: vec![webmail_host.to_string()],
                port: 80,
                path_prefix: None,
            },
        ));
    }

    let host_config = crate::docker::secure_host_config(
        network,
        inner.docker.container_pid_limit,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
        HashMap::new(),
    );

    let env = vec![
        format!("ROUNDCUBEMAIL_DEFAULT_HOST={imap_host}"),
        format!("ROUNDCUBEMAIL_SMTP_SERVER={imap_host}"),
        "ROUNDCUBEMAIL_DB_TYPE=sqlite".into(),
        "ROUNDCUBEMAIL_SKIN=elastic".into(),
    ];

    let config = Config {
        image: Some(inner.hosting.roundcube_image.clone()),
        env: Some(env),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    let response = docker
        .client()
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.clone(),
                platform: None,
            }),
            config,
        )
        .await
        .with_context(|| format!("failed to create roundcube {container_name}"))?;

    if inner.hosting.attach_proxy_network && inner.proxy.enabled {
        connect_container_network(docker.client(), &inner.proxy.network, &response.id).await?;
    }

    docker.start_container(&response.id).await?;
    Ok(response.id)
}

async fn register_mailbox(
    docker: &DockerManager,
    container_id: &str,
    address: &str,
    password: &str,
) -> Result<(), anyhow::Error> {
    docker
        .exec_run(
            container_id,
            vec![
                "setup".into(),
                "email".into(),
                "add".into(),
                address.into(),
                password.into(),
            ],
        )
        .await?;
    Ok(())
}

async fn write_manifest(state: &State, site_id: &str) -> Result<(), anyhow::Error> {
    let inner = state.config.load();
    let cache = state.cache.as_ref().context("cache unavailable")?;
    let accounts = cache.list_mail_accounts(site_id).await?;
    let dir = Path::new(&inner.system.data).join("mail").join(site_id);
    std::fs::create_dir_all(&dir)?;
    let manifest: Vec<_> = accounts
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "address": a.address,
                "quota_mb": a.quota_mb,
            })
        })
        .collect();
    std::fs::write(
        dir.join("accounts.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

fn recommended_mail_dns(mail_host: &str, mail_domain: &str) -> Vec<RecommendedDnsRecord> {
    vec![
        RecommendedDnsRecord {
            name: mail_domain.into(),
            record_type: "MX".into(),
            value: format!("10 {mail_host}."),
            ttl: 3600,
        },
        RecommendedDnsRecord {
            name: mail_domain.into(),
            record_type: "TXT".into(),
            value: format!("v=spf1 mx a:{mail_host} ~all"),
            ttl: 3600,
        },
        RecommendedDnsRecord {
            name: mail_host.into(),
            record_type: "A".into(),
            value: "<node-public-ip>".into(),
            ttl: 300,
        },
    ]
}

async fn docker_container_running(
    state: &State,
    container_id: &str,
) -> Result<bool, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let inspect = docker.inspect_container(container_id).await?;
    Ok(inspect.state.and_then(|s| s.running).unwrap_or(false))
}

fn to_summary(record: &MailAccountRecord) -> MailAccountSummary {
    MailAccountSummary {
        id: record.id.clone(),
        site_id: record.site_id.clone(),
        address: record.address.clone(),
        quota_mb: record.quota_mb,
    }
}
