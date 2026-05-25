use crate::config::InnerConfig;

use super::DockerManager;

const ERROR_PAGES_IMAGE: &str = "nginx:alpine";

/// All stack images that should stay current when auto-update is enabled.
pub fn hosting_images(inner: &InnerConfig) -> Vec<String> {
    let mut images = vec![
        inner.hosting.mailserver_image.clone(),
        inner.hosting.roundcube_image.clone(),
        inner.hosting.sftp_image.clone(),
        inner.hosting.ftp_image.clone(),
        inner.hosting.phpmyadmin_image.clone(),
        inner.hosting.mysql_image.clone(),
        inner.hosting.postgres_image.clone(),
        inner.hosting.redis_image.clone(),
        inner.hosting.mongodb_image.clone(),
        ERROR_PAGES_IMAGE.to_string(),
    ];
    if inner.proxy.enabled && inner.proxy.auto_provision {
        images.push(inner.proxy.traefik_image.clone());
    }
    images.sort();
    images.dedup();
    images
}

pub async fn pull_hosting_images(docker: &DockerManager, inner: &InnerConfig) {
    for image in hosting_images(inner) {
        tracing::info!(%image, "pulling hosting stack image");
        if let Err(err) = docker.pull_image(&image).await {
            tracing::warn!(%image, %err, "failed to pull image");
        }
    }
}

pub fn error_pages_image() -> &'static str {
    ERROR_PAGES_IMAGE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn hosting_images_are_sorted_unique() {
        let inner = Config::parse_preview(
            br#"
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  pid_file: ./featherfly.pid
"#,
        )
        .unwrap();
        let images = hosting_images(&inner);
        let mut sorted = images.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(images, sorted);
        assert!(images.contains(&inner.hosting.mysql_image));
    }

    #[test]
    fn error_pages_image_constant() {
        assert_eq!(error_pages_image(), "nginx:alpine");
    }
}
