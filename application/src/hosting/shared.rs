use std::collections::HashMap;

use anyhow::Context;
use bollard::container::{Config, CreateContainerOptions};
use bollard::models::{Mount, MountTypeEnum};

use featherfly_plugin_sdk::PluginEvent;

use crate::{
    cache::{FtpAccountRecord, MailAccountRecord, SiteRecord},
    config::InnerConfig,
    docker::{DockerManager, absolute_bind_path, connect_container_network},
    hosting::error_pages::{
        ERRORS_CONTAINER, ensure_error_page_files, write_traefik_error_middleware,
    },
    proxy::{ServiceRouteSpec, build_service_labels},
    routes::State,
    utils::plugin_events::{
        self, ErrorPagesReadyPayload, PhpMyAdminProvisionedPayload, SharedStackSyncedPayload,
    },
};

pub const SHARED_MAIL: &str = "featherfly-mailserver";
pub const SHARED_ROUNDCUBE: &str = "featherfly-roundcube";
pub const SHARED_SFTP: &str = "featherfly-sftp";
pub const SHARED_FTP: &str = "featherfly-ftp";
pub const SHARED_PHPMYADMIN: &str = "featherfly-phpmyadmin";
const SHARED_MAIL_VOLUME: &str = "featherfly-mail-data";

pub fn shared_username(site_id: &str, username: &str) -> String {
    format!("{site_id}__{username}")
}

pub async fn sync_shared_stack(state: &State) -> Result<(), anyhow::Error> {
    let inner = state.config.load();
    if !inner.hosting.shared_services {
        return Ok(());
    }

    let cache = state.cache.as_ref().context("cache unavailable")?;
    let sites = cache.list_sites().await?;
    let mail_accounts = cache.list_all_mail_accounts().await?;
    let ftp_accounts = cache.list_all_ftp_accounts().await?;

    let errors_cid = ensure_errors_nginx(state, &inner).await?;
    plugin_events::emit_state_event(
        state,
        PluginEvent::ErrorPagesReady,
        &ErrorPagesReadyPayload {
            pages_directory: &inner.hosting.error_pages_directory,
            container_id: &errors_cid,
        },
    );

    crate::hosting::shared_db::ensure_shared_database_servers(state, &inner, &sites).await?;
    let pma_cid = ensure_shared_phpmyadmin(state, &inner, &sites).await?;
    plugin_events::emit_state_event(
        state,
        PluginEvent::PhpMyAdminProvisioned,
        &PhpMyAdminProvisionedPayload {
            container_id: &pma_cid,
            site_count: sites.len(),
        },
    );

    let mut mail_synced = false;
    let mut roundcube_synced = false;
    let mut sftp_synced = false;
    let mut ftp_synced = false;

    if !mail_accounts.is_empty() {
        let mail_cid = ensure_shared_mailserver(state, &inner).await?;
        mail_synced = true;
        for account in &mail_accounts {
            register_mailbox(state, &mail_cid, &account.address, &account.password)
                .await
                .ok();
        }
        ensure_shared_roundcube(state, &inner, &sites, &mail_accounts).await?;
        roundcube_synced = true;
    }

    if ftp_accounts.iter().any(|a| a.protocol != "ftp") {
        ensure_shared_sftp(state, &inner, &sites, &ftp_accounts).await?;
        sftp_synced = true;
    }

    if ftp_accounts
        .iter()
        .any(|a| a.protocol == "ftp" || a.protocol == "both")
    {
        ensure_shared_ftp(state, &inner, &sites, &ftp_accounts).await?;
        ftp_synced = true;
    }

    plugin_events::emit_state_event(
        state,
        PluginEvent::SharedStackSynced,
        &SharedStackSyncedPayload {
            mail: mail_synced,
            roundcube: roundcube_synced,
            sftp: sftp_synced,
            ftp: ftp_synced,
            phpmyadmin: true,
            database_servers: true,
        },
    );

    Ok(())
}

pub async fn ensure_shared_mailserver(
    state: &State,
    inner: &InnerConfig,
) -> Result<String, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    if container_running(docker, SHARED_MAIL).await? {
        return Ok(SHARED_MAIL.to_string());
    }

    docker.create_volume(SHARED_MAIL_VOLUME).await.ok();
    let port_bindings = crate::hosting::mail_port_bindings(&inner.hosting.ports);

    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        port_bindings,
    );
    host_config.cap_add = Some(vec!["NET_ADMIN".into(), "SYS_PTRACE".into()]);
    host_config.mounts = Some(vec![
        Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(format!("{SHARED_MAIL_VOLUME}-config")),
            target: Some("/tmp/docker-mailserver".into()),
            ..Default::default()
        },
        Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(SHARED_MAIL_VOLUME.into()),
            target: Some("/var/mail".into()),
            ..Default::default()
        },
    ]);

    docker
        .create_volume(&format!("{SHARED_MAIL_VOLUME}-config"))
        .await
        .ok();

    let config = Config {
        image: Some(inner.hosting.mailserver_image.clone()),
        hostname: Some("mail.featherfly.local".into()),
        env: Some(vec![
            "ENABLE_SPAMASSASSIN=0".into(),
            "ENABLE_CLAMAV=0".into(),
            "ENABLE_FAIL2BAN=0".into(),
            "ONE_DIR=1".into(),
            "SSL_TYPE=".into(),
            "PERMIT_DOCKER=network".into(),
        ]),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-mailserver".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };

    recreate_named(docker, SHARED_MAIL, config).await
}

pub async fn ensure_shared_roundcube(
    state: &State,
    inner: &InnerConfig,
    sites: &[SiteRecord],
    _mail_accounts: &[MailAccountRecord],
) -> Result<String, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let mut labels = HashMap::from([
        ("featherfly.role".into(), "shared-roundcube".into()),
        ("featherfly.mail.role".into(), "roundcube".into()),
    ]);

    if inner.proxy.enabled {
        for site in sites {
            let webmail_host = format!("{}.{}", inner.hosting.webmail_subdomain, site.domain);
            labels.extend(build_service_labels(
                &inner.proxy,
                &ServiceRouteSpec {
                    site_id: site.id.clone(),
                    role: "webmail".into(),
                    hosts: vec![webmail_host],
                    port: 80,
                    path_prefix: None,
                },
            ));
        }
    }

    let host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        HashMap::new(),
    );

    let config = Config {
        image: Some(inner.hosting.roundcube_image.clone()),
        env: Some(vec![
            format!("ROUNDCUBEMAIL_DEFAULT_HOST={SHARED_MAIL}"),
            format!("ROUNDCUBEMAIL_SMTP_SERVER={SHARED_MAIL}"),
            "ROUNDCUBEMAIL_DB_TYPE=sqlite".into(),
            "ROUNDCUBEMAIL_SKIN=elastic".into(),
        ]),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    recreate_named(docker, SHARED_ROUNDCUBE, config).await
}

pub async fn ensure_shared_sftp(
    state: &State,
    inner: &InnerConfig,
    sites: &[SiteRecord],
    accounts: &[FtpAccountRecord],
) -> Result<String, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let cache = state.cache.as_ref().context("cache unavailable")?;

    let sftp_accounts: Vec<_> = accounts
        .iter()
        .filter(|a| a.protocol == "sftp" || a.protocol == "both")
        .collect();

    if sftp_accounts.is_empty() {
        anyhow::bail!("no SFTP accounts");
    }

    let mut mounts = Vec::new();
    let mut sftp_users = Vec::new();

    for account in sftp_accounts {
        let site = sites
            .iter()
            .find(|s| s.id == account.site_id)
            .context("site missing for ftp account")?;
        let volume = site.volume.as_ref().context("site has no volume")?;
        let user = shared_username(&account.site_id, &account.username);
        sftp_users.push(format!(
            "{user}:{}:::{}",
            account.password, account.home_subpath
        ));
        mounts.push(Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume.clone()),
            target: Some(format!("/home/{user}/{}", account.home_subpath)),
            ..Default::default()
        });
        mounts.push(Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume.clone()),
            target: Some(format!("/home/{user}/site")),
            ..Default::default()
        });
    }

    let sftp_ports = crate::hosting::sftp_port_bindings(&inner.hosting.ports);
    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        sftp_ports,
    );
    host_config.mounts = Some(mounts);

    let config = Config {
        image: Some(inner.hosting.sftp_image.clone()),
        env: Some(vec![format!("SFTP_USERS={}", sftp_users.join(" "))]),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-sftp".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };

    let cid = recreate_named(docker, SHARED_SFTP, config).await?;

    for account in accounts {
        if account.protocol == "sftp" || account.protocol == "both" {
            let mut meta = cache.get_ftp_meta(&account.site_id).await?.unwrap_or(
                crate::cache::FtpMetaRecord {
                    site_id: account.site_id.clone(),
                    sftp_container_id: None,
                    ftp_container_id: None,
                    updated_at: 0,
                },
            );
            meta.sftp_container_id = Some(cid.clone());
            meta.updated_at = chrono::Utc::now().timestamp();
            cache.save_ftp_meta(&meta).await?;
        }
    }

    Ok(cid)
}

pub async fn ensure_shared_ftp(
    state: &State,
    inner: &InnerConfig,
    sites: &[SiteRecord],
    accounts: &[FtpAccountRecord],
) -> Result<String, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let cache = state.cache.as_ref().context("cache unavailable")?;

    let ftp_accounts: Vec<_> = accounts
        .iter()
        .filter(|a| a.protocol == "ftp" || a.protocol == "both")
        .collect();
    let primary = ftp_accounts.first().context("no ftp accounts")?;
    let site = sites
        .iter()
        .find(|s| s.id == primary.site_id)
        .context("site missing")?;
    let volume = site.volume.as_ref().context("site has no volume")?;

    let ftp_ports = crate::hosting::ftp_port_bindings(&inner.hosting.ports);
    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        ftp_ports,
    );
    host_config.mounts = Some(vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(volume.clone()),
        target: Some("/home/ftp".into()),
        ..Default::default()
    }]);

    let user = shared_username(&primary.site_id, &primary.username);
    let env = vec![
        format!("FTP_USER_NAME={user}"),
        format!("FTP_USER_PASS={}", primary.password),
        "FTP_USER_HOME=/home/ftp".into(),
        format!(
            "FTP_PASSIVE_PORTS={}:{}",
            inner.hosting.ports.ftp_passive_min, inner.hosting.ports.ftp_passive_max
        ),
    ];

    let config = Config {
        image: Some(inner.hosting.ftp_image.clone()),
        env: Some(env),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-ftp".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };

    let cid = recreate_named(docker, SHARED_FTP, config).await?;

    for account in ftp_accounts {
        let mut meta =
            cache
                .get_ftp_meta(&account.site_id)
                .await?
                .unwrap_or(crate::cache::FtpMetaRecord {
                    site_id: account.site_id.clone(),
                    sftp_container_id: None,
                    ftp_container_id: None,
                    updated_at: 0,
                });
        meta.ftp_container_id = Some(cid.clone());
        meta.updated_at = chrono::Utc::now().timestamp();
        cache.save_ftp_meta(&meta).await?;
    }

    Ok(cid)
}

pub async fn ensure_shared_phpmyadmin(
    state: &State,
    inner: &InnerConfig,
    sites: &[SiteRecord],
) -> Result<String, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;

    let mut labels = HashMap::from([
        ("featherfly.role".into(), "shared-phpmyadmin".into()),
        ("featherfly.database.role".into(), "phpmyadmin".into()),
    ]);

    if inner.proxy.enabled {
        for site in sites {
            let host = format!("{}.{}", inner.hosting.phpmyadmin_subdomain, site.domain);
            labels.extend(build_service_labels(
                &inner.proxy,
                &ServiceRouteSpec {
                    site_id: site.id.clone(),
                    role: "phpmyadmin".into(),
                    hosts: vec![host],
                    port: 80,
                    path_prefix: None,
                },
            ));
        }
    }

    let host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        HashMap::new(),
    );

    let env = vec![
        format!("PMA_HOST={}", crate::hosting::shared_db::SHARED_MYSQL),
        "UPLOAD_LIMIT=512M".into(),
        "MAX_EXECUTION_TIME=600".into(),
    ];

    let config = Config {
        image: Some(inner.hosting.phpmyadmin_image.clone()),
        env: Some(env),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };

    let cid = recreate_named(docker, SHARED_PHPMYADMIN, config).await?;

    for site in sites {
        if let Some(network) = &site.network {
            connect_container_network(docker.client(), network, &cid)
                .await
                .ok();
        }
    }

    Ok(cid)
}

async fn ensure_errors_nginx(state: &State, inner: &InnerConfig) -> Result<String, anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let pages_dir = ensure_error_page_files(inner)?;
    write_traefik_error_middleware(inner)?;

    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        HashMap::new(),
    );
    host_config.mounts = Some(vec![Mount {
        typ: Some(MountTypeEnum::BIND),
        source: Some(absolute_bind_path(&pages_dir)?),
        target: Some("/usr/share/nginx/html".into()),
        read_only: Some(true),
        ..Default::default()
    }]);

    let config = Config {
        image: Some(crate::docker::error_pages_image().into()),
        labels: Some(HashMap::from([
            ("featherfly.role".into(), "error-pages".into()),
            ("traefik.enable".into(), "false".into()),
        ])),
        host_config: Some(host_config),
        ..Default::default()
    };

    recreate_named(docker, ERRORS_CONTAINER, config).await
}

pub async fn recreate_named_public(
    docker: &DockerManager,
    name: &str,
    config: Config<String>,
) -> Result<String, anyhow::Error> {
    recreate_named(docker, name, config).await
}

pub async fn ensure_named_public(
    docker: &DockerManager,
    name: &str,
    config: Config<String>,
) -> Result<String, anyhow::Error> {
    if container_running(docker, name).await? {
        return Ok(name.to_string());
    }
    recreate_named(docker, name, config).await
}

async fn recreate_named(
    docker: &DockerManager,
    name: &str,
    config: Config<String>,
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
        .with_context(|| format!("failed to create shared container {name}"))?;

    docker.start_container(&response.id).await?;
    Ok(response.id)
}

async fn container_running(docker: &DockerManager, name: &str) -> Result<bool, anyhow::Error> {
    let inspect = docker.inspect_container(name).await;
    Ok(inspect
        .ok()
        .and_then(|c| c.state)
        .and_then(|s| s.running)
        .unwrap_or(false))
}

async fn register_mailbox(
    state: &State,
    container_id: &str,
    address: &str,
    password: &str,
) -> Result<(), anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
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

pub fn shared_mail_container_id() -> String {
    SHARED_MAIL.to_string()
}

pub fn shared_roundcube_container_id() -> String {
    SHARED_ROUNDCUBE.to_string()
}

pub fn shared_sftp_container_id() -> String {
    SHARED_SFTP.to_string()
}

pub fn shared_ftp_container_id() -> String {
    SHARED_FTP.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_username_format() {
        assert_eq!(shared_username("site42", "admin"), "site42__admin");
    }
}
