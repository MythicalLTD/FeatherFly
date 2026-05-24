use std::collections::HashMap;

use anyhow::Context;
use featherfly_plugin_sdk::PluginEvent;
use uuid::Uuid;

use crate::{
    cache::SiteRecord,
    config::InnerConfig,
    routes::State,
    templates::load_template,
    utils::plugin_events::{self, DeployBlueGreenPayload},
};

pub async fn run_blue_green_deploy(
    state: &State,
    inner: &InnerConfig,
    site: &SiteRecord,
    git_ref: Option<&str>,
) -> Result<(), anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let cache = state.cache.as_ref().context("cache unavailable")?;

    plugin_events::emit_state_event(
        state,
        PluginEvent::DeployBlueGreenStarted,
        &DeployBlueGreenPayload { site_id: &site.id },
    );

    crate::deployments::run_deploy(inner, site, git_ref, false, false).await?;

    let template = load_template(&inner.hosting.templates_directory, &site.template)?;
    let network = site.network.as_ref().context("site has no network")?;
    let volume = site.volume.as_ref().context("site has no volume")?;
    let old_id = site.container_id.clone();

    let green_name = format!("site-{}-{}", site.id, Uuid::new_v4().simple());
    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site.id.clone());
    labels.insert("featherfly.site.domain".into(), site.domain.clone());

    if inner.proxy.enabled {
        labels.extend(crate::proxy::build_site_labels(
            &inner.proxy,
            &crate::proxy::SiteProxySpec {
                site_id: site.id.clone(),
                domains: vec![site.domain.clone()],
                port: template.port,
                path_prefix: None,
            },
        ));
    }

    let mount_target = if template.mount_path.is_empty() {
        template.workdir.clone()
    } else {
        template.mount_path.clone()
    };

    let green_id = crate::sites::create_site_container_internal(
        docker,
        inner,
        &green_name,
        &template,
        network,
        volume,
        &mount_target,
        &HashMap::new(),
        labels,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
    )
    .await?;

    if inner.hosting.attach_proxy_network && inner.proxy.enabled {
        crate::docker::connect_container_network(docker.client(), &inner.proxy.network, &green_id)
            .await?;
    }

    docker.start_container(&green_id).await?;

    if let Some(old) = old_id {
        let _ = docker.stop_container(&old).await;
        docker.remove_container(&old, true).await.ok();
    }

    let mut updated = site.clone();
    updated.container_id = Some(green_id);
    cache.save_site(&updated).await?;

    plugin_events::emit_state_event(
        state,
        PluginEvent::DeployBlueGreenFinished,
        &DeployBlueGreenPayload { site_id: &site.id },
    );

    Ok(())
}
