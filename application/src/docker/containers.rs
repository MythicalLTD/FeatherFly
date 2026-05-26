use std::collections::HashMap;

use anyhow::Context;
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, StopContainerOptions,
    UpdateContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::PortBinding;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{DockerManager, memory_limit_bytes, secure_host_config};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub exposed_ports: Vec<u16>,
    #[serde(default)]
    pub memory_mb: Option<u64>,
    #[serde(default)]
    pub cpu_quota: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateContainerResponse {
    pub id: String,
    pub name: String,
    pub started: bool,
}

impl DockerManager {
    pub async fn list_containers(&self) -> Result<Vec<ContainerSummary>, anyhow::Error> {
        let options = Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        });

        let containers = self
            .client()
            .list_containers(options)
            .await
            .context("failed to list containers")?;

        Ok(containers
            .into_iter()
            .filter_map(|container| {
                let id = container.id?;
                let name = container
                    .names
                    .and_then(|names| names.into_iter().next())
                    .unwrap_or_else(|| id.clone());
                Some(ContainerSummary {
                    id,
                    name: name.trim_start_matches('/').to_string(),
                    image: container.image.unwrap_or_else(|| "unknown".into()),
                    state: container.state.unwrap_or_else(|| "unknown".into()),
                    status: container.status.unwrap_or_default(),
                })
            })
            .collect())
    }

    pub async fn create_container(
        &self,
        request: &CreateContainerRequest,
        start: bool,
    ) -> Result<CreateContainerResponse, anyhow::Error> {
        let env: Vec<String> = request
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();

        let mut exposed_ports = HashMap::new();
        let mut port_bindings = HashMap::new();

        for port in &request.exposed_ports {
            let key = format!("{port}/tcp");
            exposed_ports.insert(key.clone(), HashMap::new());
            port_bindings.insert(
                key,
                Some(vec![PortBinding {
                    host_ip: Some(String::new()),
                    host_port: Some(port.to_string()),
                }]),
            );
        }

        let host_config = secure_host_config(
            &self.config.network.network_mode,
            self.config.container_pid_limit,
            request.memory_mb,
            request.cpu_quota,
            port_bindings,
        );

        let config = Config {
            image: Some(request.image.clone()),
            env: Some(env),
            labels: Some(request.labels.clone()),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = Some(CreateContainerOptions {
            name: request.name.clone(),
            platform: None,
        });

        let response = self
            .client()
            .create_container(options, config)
            .await
            .with_context(|| format!("failed to create container {}", request.name))?;

        let id = response.id;

        if start {
            self.start_container(&id).await?;
        }

        Ok(CreateContainerResponse {
            id: id.clone(),
            name: request.name.clone(),
            started: start,
        })
    }

    pub async fn start_container(&self, id: &str) -> Result<(), anyhow::Error> {
        self.client()
            .start_container(id, None::<StartContainerOptions<String>>)
            .await
            .with_context(|| format!("failed to start container {id}"))?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str) -> Result<(), anyhow::Error> {
        self.client()
            .stop_container(id, None::<StopContainerOptions>)
            .await
            .with_context(|| format!("failed to stop container {id}"))?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> Result<(), anyhow::Error> {
        self.client()
            .restart_container(id, None::<RestartContainerOptions>)
            .await
            .with_context(|| format!("failed to restart container {id}"))?;
        Ok(())
    }

    pub async fn pull_image(&self, image: &str) -> Result<(), anyhow::Error> {
        let options = Some(CreateImageOptions {
            from_image: image,
            ..Default::default()
        });

        let mut stream = self.client().create_image(options, None, None);
        while let Some(result) = stream.next().await {
            result.with_context(|| format!("failed while pulling image {image}"))?;
        }
        Ok(())
    }

    pub async fn remove_container(&self, id: &str, force: bool) -> Result<(), anyhow::Error> {
        let options = Some(RemoveContainerOptions {
            force,
            ..Default::default()
        });

        self.client()
            .remove_container(id, options)
            .await
            .with_context(|| format!("failed to remove container {id}"))?;
        Ok(())
    }

    pub async fn update_container_resources(
        &self,
        id: &str,
        memory_mb: Option<u64>,
        cpu_quota: Option<i64>,
    ) -> Result<(), anyhow::Error> {
        let memory = memory_limit_bytes(memory_mb);
        let options = UpdateContainerOptions::<String> {
            memory,
            memory_swap: memory,
            cpu_quota,
            ..Default::default()
        };

        self.client()
            .update_container(id, options)
            .await
            .with_context(|| format!("failed to update container resource limits for {id}"))?;
        Ok(())
    }

    pub async fn inspect_container(
        &self,
        id: &str,
    ) -> Result<bollard::models::ContainerInspectResponse, anyhow::Error> {
        self.client()
            .inspect_container(id, None::<InspectContainerOptions>)
            .await
            .with_context(|| format!("failed to inspect container {id}"))
    }
}
