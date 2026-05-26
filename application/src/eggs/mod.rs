mod config_files;
mod definition;
mod install;
mod pterodactyl;

pub use config_files::{ConfigFileContext, apply_process_config};
pub use definition::{
    EggConfigFile, EggConfigReplacement, EggDefinition, EggInstallScript, EggProcessConfig,
    EggRuntime, EggSummary, EggVariable, substitute_placeholders,
};
pub use install::run_install_script;
pub use pterodactyl::import_pterodactyl_egg;

use std::path::Path;

use anyhow::Context;

pub fn load_egg(eggs_dir: &str, id: &str) -> Result<EggDefinition, anyhow::Error> {
    let path = Path::new(eggs_dir).join(format!("{id}.json"));
    if !path.exists() {
        anyhow::bail!("egg not found: {id}");
    }
    load_egg_file(&path)
}

fn load_egg_file(path: &Path) -> Result<EggDefinition, anyhow::Error> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read egg {}", path.display()))?;
    parse_egg_json(path, &raw)
}

fn parse_egg_json(path: &Path, raw: &str) -> Result<EggDefinition, anyhow::Error> {
    let value: serde_json::Value = serde_json::from_str(raw).context("failed to parse egg JSON")?;

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    if value
        .get("meta")
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .is_some_and(|v| v.starts_with("PTDL"))
    {
        return import_pterodactyl_egg(&id, raw);
    }

    let mut egg: EggDefinition =
        serde_json::from_value(value).context("failed to decode FeatherFly egg")?;
    if egg.id.is_empty() {
        egg.id = id;
    }
    Ok(egg)
}

pub fn list_eggs(eggs_dir: &str) -> Result<Vec<EggSummary>, anyhow::Error> {
    let dir = Path::new(eggs_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut eggs = Vec::new();
    for entry in std::fs::read_dir(dir).context("failed to read eggs directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            match load_egg_file(&path) {
                Ok(egg) => eggs.push(egg.summary()),
                Err(err) => tracing::warn!(path = %path.display(), %err, "skipping invalid egg"),
            }
        }
    }
    eggs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(eggs)
}

/// Ensure eggs directory exists. Does not seed hosting templates — eggs are the product surface.
pub fn ensure_eggs_directory(eggs_dir: &str) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(eggs_dir)?;
    Ok(())
}
