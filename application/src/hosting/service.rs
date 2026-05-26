use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    hosting::ports::{PublishedPort, list_published_ports},
    routes::State,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HostingOverview {
    pub site_id: String,
    pub domain: String,
    pub ssl_enabled: bool,
    pub published_ports: Vec<PublishedPort>,
    pub services: HostingServices,
    pub limits: HostingLimits,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HostingServices {
    pub web: bool,
    pub mail: bool,
    pub webmail: bool,
    pub phpmyadmin: bool,
    pub sftp: bool,
    pub ftp: bool,
    pub databases: usize,
    pub cron_tasks: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HostingLimits {
    pub memory_mb: Option<i64>,
    pub cpu_quota: Option<i64>,
    pub disk_quota_mb: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateLimitsRequest {
    pub memory_mb: Option<u64>,
    pub cpu_quota: Option<i64>,
    pub disk_quota_mb: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
}

pub struct HostingService;

impl HostingService {
    pub async fn overview(state: &State, site_id: &str) -> Result<HostingOverview, anyhow::Error> {
        let inner = state.config.load();
        let cache = state
            .cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
        let site = cache
            .get_site(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("site not found"))?;

        let mail_meta = cache.get_mail_meta(site_id).await?;
        let ftp_meta = cache.get_ftp_meta(site_id).await?;
        let databases_list = cache.list_databases(site_id).await?;
        let databases = databases_list.len();
        let has_mysql = databases_list.iter().any(|d| d.engine == "mysql");
        let has_phpmyadmin = has_mysql && inner.hosting.shared_services && state.docker.is_some();

        let has_webmail = mail_meta
            .as_ref()
            .and_then(|m| m.roundcube_container_id.as_ref())
            .is_some();

        let cron_tasks = cache.list_tasks(site_id).await?.len();
        let has_mail = mail_meta.is_some();

        Ok(HostingOverview {
            site_id: site.id.clone(),
            domain: site.domain.clone(),
            ssl_enabled: inner.proxy.acme_ready(),
            published_ports: list_published_ports(&inner.hosting.ports),
            services: HostingServices {
                web: site.container_id.is_some(),
                mail: has_mail,
                webmail: has_webmail,
                phpmyadmin: has_phpmyadmin,
                sftp: ftp_meta.is_some(),
                ftp: ftp_meta.is_some(),
                databases,
                cron_tasks,
            },
            limits: HostingLimits {
                memory_mb: site.memory_mb,
                cpu_quota: site.cpu_quota,
                disk_quota_mb: site.disk_quota_mb,
                bandwidth_mbps: site.bandwidth_mbps,
            },
        })
    }

    pub async fn update_limits(
        state: &State,
        site_id: &str,
        req: UpdateLimitsRequest,
    ) -> Result<HostingLimits, anyhow::Error> {
        let update_container_resources = req.memory_mb.is_some() || req.cpu_quota.is_some();
        let cache = state
            .cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
        let mut site = cache
            .get_site(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("site not found"))?;

        if let Some(m) = req.memory_mb {
            site.memory_mb = Some(m as i64);
        }
        if let Some(c) = req.cpu_quota {
            site.cpu_quota = Some(c);
        }
        if let Some(d) = req.disk_quota_mb {
            site.disk_quota_mb = Some(d);
        }
        if let Some(b) = req.bandwidth_mbps {
            site.bandwidth_mbps = Some(b);
        }

        if update_container_resources && let Some(container_id) = site.container_id.as_deref() {
            let docker = state
                .docker
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("docker unavailable"))?;
            docker
                .update_container_resources(
                    container_id,
                    site.memory_mb.and_then(|memory| u64::try_from(memory).ok()),
                    site.cpu_quota,
                )
                .await?;
        }

        cache.save_site(&site).await?;

        crate::utils::plugin_events::emit_state_event(
            state,
            featherfly_plugin_sdk::PluginEvent::HostingLimitsUpdated,
            &crate::utils::plugin_events::HostingLimitsUpdatedPayload { site_id },
        );

        Ok(HostingLimits {
            memory_mb: site.memory_mb,
            cpu_quota: site.cpu_quota,
            disk_quota_mb: site.disk_quota_mb,
            bandwidth_mbps: site.bandwidth_mbps,
        })
    }
}
