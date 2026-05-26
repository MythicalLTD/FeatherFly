use std::collections::HashMap;

use anyhow::Context;
use featherfly_plugin_sdk::PluginEvent;
use uuid::Uuid;

use crate::{
    cache::SiteRecord,
    config::InnerConfig,
    deployments::health::wait_for_site_ready,
    eggs::load_egg,
    routes::State,
    sites::create_site_container_internal,
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

    let egg = load_egg(&inner.hosting.eggs_directory, &site.egg_id)?;
    let mut runtime = egg.resolve(&site.variables)?;
    if let Some(root) = site.document_root.as_deref().filter(|r| !r.is_empty()) {
        runtime
            .env
            .insert("FEATHERFLY_DOCUMENT_ROOT".into(), root.into());
    }

    let network = site.network.as_ref().context("site has no network")?;
    let volume = site.volume.as_ref().context("site has no volume")?;
    let old_id = site.container_id.clone();

    let green_name = format!("site-{}-{}", site.id, Uuid::new_v4().simple());
    let mut labels = HashMap::new();
    labels.insert("featherfly.site.id".into(), site.id.clone());
    labels.insert("featherfly.site.domain".into(), site.domain.clone());

    let green_id = create_site_container_internal(
        docker,
        inner,
        &green_name,
        &runtime,
        network,
        volume,
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

    let health_path = inner.proxy.health_check_path.as_deref();
    if let Err(err) =
        wait_for_site_ready(docker, &green_id, network, runtime.port, health_path).await
    {
        let _ = docker.stop_container(&green_id).await;
        docker.remove_container(&green_id, true).await.ok();
        anyhow::bail!("blue/green deploy rolled back — new container failed health check: {err:#}");
    }

    if let Some(old) = old_id {
        let _ = docker.stop_container(&old).await;
        docker.remove_container(&old, true).await.ok();
    }

    let _ = docker.stop_container(&green_id).await;
    docker.remove_container(&green_id, true).await.ok();

    let routing_domains = crate::sites::site_routing_hosts(site);
    let mut final_labels = HashMap::new();
    final_labels.insert("featherfly.site.id".into(), site.id.clone());
    final_labels.insert("featherfly.site.domain".into(), site.domain.clone());
    if inner.proxy.enabled && crate::sites::site_is_active(&site.status) {
        final_labels.extend(crate::proxy::build_site_labels(
            &inner.proxy,
            &crate::proxy::SiteProxySpec {
                site_id: site.id.clone(),
                domains: routing_domains,
                port: runtime.port,
                path_prefix: None,
            },
        ));
    }

    let final_name = format!("site-{}", site.id);
    let final_id = create_site_container_internal(
        docker,
        inner,
        &final_name,
        &runtime,
        network,
        volume,
        final_labels,
        site.memory_mb.map(|m| m as u64),
        site.cpu_quota,
    )
    .await?;

    if inner.hosting.attach_proxy_network && inner.proxy.enabled {
        crate::docker::connect_container_network(docker.client(), &inner.proxy.network, &final_id)
            .await?;
    }

    docker.start_container(&final_id).await?;

    let mut updated = site.clone();
    updated.container_id = Some(final_id);
    cache.save_site(&updated).await?;

    plugin_events::emit_state_event(
        state,
        PluginEvent::DeployBlueGreenFinished,
        &DeployBlueGreenPayload { site_id: &site.id },
    );

    Ok(())
}
