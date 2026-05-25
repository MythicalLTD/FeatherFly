use crate::{
    ftp::{FtpService, FtpSyncRequest},
    mail::{MailService, MailSyncRequest},
    routes::State,
    utils::plugin_events::{self, HostingImagesPulledPayload, HostingReconciledPayload},
};
use featherfly_plugin_sdk::PluginEvent;

#[derive(Debug, Default)]
pub struct ReconcileSummary {
    pub sites_total: usize,
    pub sites_started: usize,
    pub mail_synced: usize,
    pub ftp_synced: usize,
    pub shared_stack: bool,
}

pub async fn reconcile_on_startup(state: &State) -> ReconcileSummary {
    let inner = state.config.load();
    if !inner.hosting.reconcile_on_startup {
        tracing::debug!("hosting.reconcile_on_startup disabled");
        return ReconcileSummary::default();
    }

    let Some(docker) = state.docker.as_ref() else {
        tracing::warn!("skipping hosting reconcile — docker unavailable");
        return ReconcileSummary::default();
    };

    let Some(cache) = state.cache.as_ref() else {
        tracing::warn!("skipping hosting reconcile — cache unavailable");
        return ReconcileSummary::default();
    };

    if inner.hosting.auto_update_images || inner.hosting.auto_pull_images {
        let images = crate::docker::hosting_images(&inner);
        crate::docker::pull_hosting_images(docker, &inner).await;
        plugin_events::emit_state_event(
            state,
            PluginEvent::HostingImagesPulled,
            &HostingImagesPulledPayload {
                image_count: images.len(),
                images,
            },
        );
    }

    let mut summary = ReconcileSummary::default();

    let sites = match cache.list_sites().await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(%err, "failed to list sites for reconcile");
            return summary;
        }
    };
    summary.sites_total = sites.len();

    for site in &sites {
        if let Some(cid) = &site.container_id
            && docker.start_container(cid).await.is_ok()
        {
            summary.sites_started += 1;
        }

        let mail_accounts = cache.list_mail_accounts(&site.id).await.unwrap_or_default();
        let mail_meta = cache.get_mail_meta(&site.id).await.ok().flatten();
        if !mail_accounts.is_empty() || mail_meta.is_some() {
            let req = MailSyncRequest {
                provision_mailserver: true,
                provision_roundcube: true,
                mail_domain: None,
            };
            if MailService::sync(state, &site.id, req).await.is_ok() {
                summary.mail_synced += 1;
            }
        }

        let ftp_accounts = cache.list_ftp_accounts(&site.id).await.unwrap_or_default();
        if !ftp_accounts.is_empty() {
            let req = FtpSyncRequest { reprovision: true };
            if FtpService::sync(state, &site.id, req).await.is_ok() {
                summary.ftp_synced += 1;
            }
        }
    }

    if inner.hosting.shared_services {
        match crate::hosting::shared::sync_shared_stack(state).await {
            Ok(()) => summary.shared_stack = true,
            Err(err) => tracing::warn!(%err, "shared hosting stack reconcile failed"),
        }
    }

    tracing::info!(
        sites = summary.sites_total,
        started = summary.sites_started,
        mail = summary.mail_synced,
        ftp = summary.ftp_synced,
        shared = summary.shared_stack,
        "hosting reconcile complete"
    );

    plugin_events::emit_state_event(
        state,
        PluginEvent::HostingReconciled,
        &HostingReconciledPayload {
            sites_total: summary.sites_total,
            sites_started: summary.sites_started,
            mail_synced: summary.mail_synced,
            ftp_synced: summary.ftp_synced,
            shared_stack: summary.shared_stack,
        },
    );

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_summary_is_empty() {
        let s = ReconcileSummary::default();
        assert_eq!(s.sites_total, 0);
        assert_eq!(s.sites_started, 0);
        assert!(!s.shared_stack);
    }
}
