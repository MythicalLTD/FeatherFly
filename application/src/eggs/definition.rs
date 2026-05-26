use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Local egg catalog entry (Wings/Pterodactyl-inspired runtime contract).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub docker_image: String,
    /// Startup invocation — passed as `STARTUP` env (Wings-compatible).
    pub startup: String,
    #[serde(default)]
    pub entrypoint: Option<Vec<String>>,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workdir")]
    pub workdir: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub file_denylist: Vec<String>,
    #[serde(default)]
    pub variables: Vec<EggVariable>,
    #[serde(default)]
    pub install: Option<EggInstallScript>,
    #[serde(default)]
    pub config: Option<EggProcessConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggVariable {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_value: String,
    #[serde(default = "default_true")]
    pub user_viewable: bool,
    #[serde(default = "default_true")]
    pub user_editable: bool,
    #[serde(default)]
    pub rules: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggInstallScript {
    pub container_image: String,
    #[serde(default = "default_install_entrypoint")]
    pub entrypoint: String,
    pub script: String,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggProcessConfig {
    #[serde(default)]
    pub startup_done: Vec<String>,
    #[serde(default)]
    pub stop: Option<EggStopConfig>,
    #[serde(default)]
    pub files: Vec<EggConfigFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggStopConfig {
    #[serde(rename = "type", default = "default_stop_type")]
    pub stop_type: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggConfigFile {
    pub file: String,
    #[serde(default = "default_true")]
    pub create_new: bool,
    #[serde(default = "default_parser")]
    pub parser: String,
    #[serde(default)]
    pub replace: Vec<EggConfigReplacement>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EggConfigReplacement {
    pub r#match: String,
    #[serde(default = "default_true")]
    pub update_existing: bool,
    #[serde(default)]
    pub replace_with: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EggSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    pub port: u16,
}

/// Resolved runtime values for container create (after variable substitution).
#[derive(Debug, Clone)]
pub struct EggRuntime {
    pub image: String,
    pub startup: String,
    pub entrypoint: Vec<String>,
    pub port: u16,
    pub workdir: String,
    pub env: HashMap<String, String>,
    pub file_denylist: Vec<String>,
}

fn default_port() -> u16 {
    80
}

fn default_workdir() -> String {
    "/home/container".into()
}

fn default_true() -> bool {
    true
}

fn default_install_entrypoint() -> String {
    "bash".into()
}

fn default_stop_type() -> String {
    "signal".into()
}

fn default_parser() -> String {
    "file".into()
}

impl EggDefinition {
    #[must_use]
    pub fn summary(&self) -> EggSummary {
        EggSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            features: self.features.clone(),
            port: self.port,
        }
    }

    pub fn resolve(
        &self,
        overrides: &HashMap<String, String>,
    ) -> Result<EggRuntime, anyhow::Error> {
        let mut env = HashMap::new();
        for var in &self.variables {
            env.insert(var.name.clone(), var.default_value.clone());
        }
        for (key, value) in overrides {
            env.insert(key.clone(), value.clone());
        }

        let startup = substitute_placeholders(&self.startup, &env);
        env.insert("STARTUP".into(), startup.clone());

        let entrypoint = self.entrypoint.clone().unwrap_or_else(default_entrypoint);

        Ok(EggRuntime {
            image: self.docker_image.clone(),
            startup,
            entrypoint,
            port: self.port,
            workdir: self.workdir.clone(),
            env,
            file_denylist: self.file_denylist.clone(),
        })
    }
}

fn default_entrypoint() -> Vec<String> {
    vec!["/bin/bash".into(), "-c".into()]
}

pub fn substitute_placeholders(input: &str, env: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, value) in env {
        result = result.replace(&format!("{{{{env.{key}}}}}"), value);
        result = result.replace(&format!("{{{{{key}}}}}"), value);
        result = result.replace(&format!("${{{key}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_merges_variables_and_startup() {
        let egg = EggDefinition {
            id: "test".into(),
            name: "Test".into(),
            description: None,
            author: None,
            docker_image: "alpine:3".into(),
            startup: "node {{APP_FILE}}".into(),
            entrypoint: None,
            port: 3000,
            workdir: "/home/container".into(),
            features: vec![],
            file_denylist: vec![],
            variables: vec![EggVariable {
                name: "APP_FILE".into(),
                description: None,
                default_value: "server.js".into(),
                user_viewable: true,
                user_editable: true,
                rules: None,
            }],
            install: None,
            config: None,
        };

        let runtime = egg.resolve(&HashMap::new()).unwrap();
        assert_eq!(runtime.startup, "node server.js");
        assert_eq!(
            runtime.env.get("STARTUP").map(String::as_str),
            Some("node server.js")
        );
    }
}
