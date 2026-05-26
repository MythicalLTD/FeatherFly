use std::collections::HashMap;

use anyhow::Context;
use serde::Deserialize;

use super::definition::{
    EggDefinition, EggInstallScript, EggProcessConfig, EggStopConfig, EggVariable,
};

/// Pterodactyl Panel egg export (PTDL_v2) — imported into FeatherFly's local catalog format.
#[derive(Debug, Deserialize)]
struct PterodactylEggExport {
    name: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    docker_images: HashMap<String, String>,
    #[serde(default)]
    startup: String,
    #[serde(default)]
    config: Option<PterodactylEggConfig>,
    #[serde(default)]
    scripts: Option<PterodactylEggScripts>,
    #[serde(default)]
    variables: Vec<PterodactylEggVariable>,
    #[serde(default)]
    file_denylist: Vec<String>,
    #[serde(default)]
    features: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PterodactylEggConfig {
    #[serde(default)]
    startup: Option<PterodactylStartupConfig>,
    #[serde(default)]
    stop: Option<String>,
    #[serde(default)]
    files: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PterodactylStartupConfig {
    #[serde(default)]
    done: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PterodactylEggScripts {
    #[serde(default)]
    installation: Option<PterodactylInstallScript>,
}

#[derive(Debug, Deserialize)]
struct PterodactylInstallScript {
    #[serde(default)]
    container: Option<String>,
    #[serde(default)]
    entrypoint: Option<String>,
    #[serde(default)]
    script: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PterodactylEggVariable {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    env_variable: Option<String>,
    #[serde(default)]
    default_value: Option<String>,
    #[serde(default)]
    user_viewable: bool,
    #[serde(default = "default_true")]
    user_editable: bool,
    #[serde(default)]
    rules: Option<String>,
}

fn default_true() -> bool {
    true
}

pub fn import_pterodactyl_egg(id: &str, raw: &str) -> Result<EggDefinition, anyhow::Error> {
    let exported: PterodactylEggExport =
        serde_json::from_str(raw).context("failed to parse Pterodactyl egg JSON")?;

    let docker_image = exported
        .docker_images
        .values()
        .next()
        .cloned()
        .context("Pterodactyl egg has no docker_images")?;

    let install = exported.scripts.and_then(|scripts| {
        scripts.installation.and_then(|install| {
            let container_image = install.container?;
            let script = install.script?;
            Some(EggInstallScript {
                container_image,
                entrypoint: install.entrypoint.unwrap_or_else(|| "bash".into()),
                script,
                environment: HashMap::new(),
            })
        })
    });

    let config = exported.config.map(|cfg| {
        let startup_done = cfg
            .startup
            .and_then(|s| s.done)
            .map(|done| vec![done])
            .unwrap_or_default();
        let stop = cfg.stop.map(|value| EggStopConfig {
            stop_type: if value.starts_with('^') {
                "signal".into()
            } else {
                "command".into()
            },
            value: Some(value),
        });
        EggProcessConfig {
            startup_done,
            stop,
            files: Vec::new(),
        }
    });

    let variables = exported
        .variables
        .into_iter()
        .map(|var| EggVariable {
            name: var.env_variable.unwrap_or(var.name),
            description: var.description,
            default_value: var.default_value.unwrap_or_default(),
            user_viewable: var.user_viewable,
            user_editable: var.user_editable,
            rules: var.rules,
        })
        .collect();

    Ok(EggDefinition {
        id: id.into(),
        name: exported.name,
        description: exported.description,
        author: exported.author,
        docker_image,
        startup: exported.startup,
        entrypoint: Some(vec!["/bin/bash".into(), "-c".into()]),
        port: 80,
        workdir: "/home/container".into(),
        features: exported.features.unwrap_or_default(),
        file_denylist: exported.file_denylist,
        variables,
        install,
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_minimal_ptdl_export() {
        let raw = r#"{
            "meta": { "version": "PTDL_v2" },
            "name": "Node Server",
            "author": "Pterodactyl",
            "docker_images": { "main": "ghcr.io/pterodactyl/games:nodejs" },
            "startup": "node /home/container/{{STARTUP_FILE}}",
            "variables": [{
                "name": "Startup File",
                "env_variable": "STARTUP_FILE",
                "default_value": "index.js",
                "user_viewable": true,
                "user_editable": true
            }]
        }"#;

        let egg = import_pterodactyl_egg("node-server", raw).unwrap();
        assert_eq!(egg.name, "Node Server");
        assert_eq!(egg.docker_image, "ghcr.io/pterodactyl/games:nodejs");
        assert_eq!(egg.variables[0].name, "STARTUP_FILE");
    }
}
