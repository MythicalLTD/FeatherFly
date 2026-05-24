use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SiteTemplate {
    pub name: String,
    pub image: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub startup_command: Vec<String>,
    #[serde(default = "default_workdir")]
    pub workdir: String,
    #[serde(default)]
    pub mount_path: String,
}

fn default_port() -> u16 {
    80
}

fn default_workdir() -> String {
    "/app".into()
}

pub fn load_template(templates_dir: &str, name: &str) -> Result<SiteTemplate, anyhow::Error> {
    let path = Path::new(templates_dir).join(format!("{name}.yml"));
    if !path.exists() {
        let alt = Path::new(templates_dir).join(format!("{name}.yaml"));
        if alt.exists() {
            return load_template_file(&alt);
        }
        anyhow::bail!("template not found: {name}");
    }
    load_template_file(&path)
}

fn load_template_file(path: &Path) -> Result<SiteTemplate, anyhow::Error> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read template {}", path.display()))?;
    serde_norway::from_str(&raw)
        .with_context(|| format!("failed to parse template {}", path.display()))
}

pub fn list_templates(templates_dir: &str) -> Result<Vec<String>, anyhow::Error> {
    let dir = Path::new(templates_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).context("failed to read templates directory")? {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub fn ensure_builtin_templates(templates_dir: &str) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(templates_dir)?;
    let nodejs = Path::new(templates_dir).join("nodejs.yml");
    if !nodejs.exists() {
        std::fs::write(
            &nodejs,
            r#"name: NodeJS
image: node:22-alpine
port: 3000
workdir: /app
mount_path: /app
startup_command:
  - node
  - server.js
env:
  NODE_ENV: production
"#,
        )?;
    }
    let static_site = Path::new(templates_dir).join("static.yml");
    if !static_site.exists() {
        std::fs::write(
            &static_site,
            r#"name: Static
image: nginx:alpine
port: 80
workdir: /usr/share/nginx/html
mount_path: /usr/share/nginx/html
"#,
        )?;
    }
    Ok(())
}
