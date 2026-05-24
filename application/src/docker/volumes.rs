use anyhow::Context;
use bollard::volume::{CreateVolumeOptions, RemoveVolumeOptions};

use super::DockerManager;

impl DockerManager {
    pub async fn create_volume(&self, name: &str) -> Result<(), anyhow::Error> {
        let options = CreateVolumeOptions {
            name: name.to_string(),
            driver: "local".into(),
            ..Default::default()
        };
        self.client()
            .create_volume(options)
            .await
            .with_context(|| format!("failed to create volume {name}"))?;
        Ok(())
    }

    pub async fn remove_volume(&self, name: &str, force: bool) -> Result<(), anyhow::Error> {
        let options = Some(RemoveVolumeOptions { force });
        self.client()
            .remove_volume(name, options)
            .await
            .with_context(|| format!("failed to remove volume {name}"))?;
        Ok(())
    }
}
