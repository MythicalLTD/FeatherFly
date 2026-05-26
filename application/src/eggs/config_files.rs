use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use tokio::process::Command;

use super::definition::{EggConfigFile, EggProcessConfig, substitute_placeholders};

pub struct ConfigFileContext<'a> {
    pub site_id: &'a str,
    pub port: u16,
    pub env: &'a HashMap<String, String>,
}

pub async fn apply_process_config(
    volume: &str,
    config: &EggProcessConfig,
    ctx: &ConfigFileContext<'_>,
) -> Result<(), anyhow::Error> {
    for file in &config.files {
        apply_config_file(volume, file, ctx).await?;
    }
    Ok(())
}

async fn apply_config_file(
    volume: &str,
    file: &EggConfigFile,
    ctx: &ConfigFileContext<'_>,
) -> Result<(), anyhow::Error> {
    let rel_path = sanitize_relative_path(&file.file)?;
    let existing = read_volume_file(volume, &rel_path).await?;

    let content = match existing {
        Some(c) if !c.is_empty() => c,
        Some(_) if file.create_new => String::new(),
        Some(_) => return Ok(()),
        None if file.create_new => String::new(),
        None => return Ok(()),
    };

    let updated = match file.parser.as_str() {
        "properties" => apply_properties_parser(&content, file, ctx),
        "file" | "plain" => apply_plain_parser(&content, file, ctx),
        other => anyhow::bail!("unsupported egg config parser: {other}"),
    };

    write_volume_file(volume, &rel_path, &updated).await?;
    tracing::debug!(site = ctx.site_id, path = %rel_path, "applied egg config file");
    Ok(())
}

fn sanitize_relative_path(path: &str) -> Result<String, anyhow::Error> {
    let path = path.trim().trim_start_matches('/');
    if path.is_empty() || path.contains("..") {
        anyhow::bail!("invalid config file path");
    }
    Ok(path.into())
}

fn lookup_value(key: &str, ctx: &ConfigFileContext<'_>) -> String {
    if key.starts_with("env.") {
        let var = key.trim_start_matches("env.");
        return ctx.env.get(var).cloned().unwrap_or_default();
    }
    match key {
        "site.id" => ctx.site_id.to_string(),
        "site.port" | "server.build.default.port" => ctx.port.to_string(),
        _ => ctx.env.get(key).cloned().unwrap_or_default(),
    }
}

fn resolve_template(input: &str, ctx: &ConfigFileContext<'_>) -> String {
    let mut result = substitute_placeholders(input, ctx.env);
    while let Some(start) = result.find("{{") {
        let Some(end) = result[start..].find("}}") else {
            break;
        };
        let end = start + end;
        let token = result[start + 2..end].trim();
        let value = lookup_value(token, ctx);
        result.replace_range(start..=end + 1, &value);
    }
    result
}

fn apply_plain_parser(content: &str, file: &EggConfigFile, ctx: &ConfigFileContext<'_>) -> String {
    let mut lines = Vec::new();
    let mut matched = std::collections::HashSet::new();

    for line in content.lines() {
        let mut replaced = false;
        for replacement in &file.replace {
            if line.starts_with(&replacement.r#match) && replacement.update_existing {
                lines.push(resolve_template(&replacement.replace_with, ctx));
                matched.insert(replacement.r#match.clone());
                replaced = true;
                break;
            }
        }
        if !replaced {
            lines.push(line.to_string());
        }
    }

    for replacement in &file.replace {
        if !matched.contains(&replacement.r#match) {
            lines.push(resolve_template(&replacement.replace_with, ctx));
        }
    }

    if lines.is_empty() && content.is_empty() {
        for replacement in &file.replace {
            lines.push(resolve_template(&replacement.replace_with, ctx));
        }
    }

    format!("{}\n", lines.join("\n").trim_end())
}

fn apply_properties_parser(
    content: &str,
    file: &EggConfigFile,
    ctx: &ConfigFileContext<'_>,
) -> String {
    apply_plain_parser(content, file, ctx)
}

async fn read_volume_file(volume: &str, rel_path: &str) -> Result<Option<String>, anyhow::Error> {
    let status = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{volume}:/data"),
            "alpine:3",
            "cat",
            &format!("/data/{rel_path}"),
        ])
        .output()
        .await
        .context("failed to read file from site volume")?;

    if status.status.success() {
        return Ok(Some(String::from_utf8_lossy(&status.stdout).into_owned()));
    }

    let stderr = String::from_utf8_lossy(&status.stderr);
    if stderr.contains("No such file") || stderr.contains("not found") {
        Ok(None)
    } else {
        anyhow::bail!("failed to read {rel_path} from volume: {stderr}");
    }
}

async fn write_volume_file(
    volume: &str,
    rel_path: &str,
    content: &str,
) -> Result<(), anyhow::Error> {
    let parent = Path::new(rel_path).parent().unwrap_or(Path::new(""));
    let parent_arg = if parent.as_os_str().is_empty() {
        "/data".to_string()
    } else {
        format!("/data/{}", parent.display())
    };

    let mut child = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-i",
            "-v",
            &format!("{volume}:/data"),
            "alpine:3",
            "sh",
            "-c",
            &format!("mkdir -p '{parent_arg}' && cat > '/data/{rel_path}'"),
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("failed to write file into site volume")?;

    use tokio::io::AsyncWriteExt;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes()).await?;
    }

    let status = child.wait().await?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("docker write into volume failed for {rel_path}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eggs::{EggConfigFile, EggConfigReplacement};

    #[test]
    fn plain_parser_replaces_matching_lines() {
        let file = EggConfigFile {
            file: "server.properties".into(),
            create_new: true,
            parser: "file".into(),
            replace: vec![EggConfigReplacement {
                r#match: "server-port=".into(),
                update_existing: true,
                replace_with: "server-port={{site.port}}".into(),
            }],
        };
        let env = HashMap::new();
        let ctx = ConfigFileContext {
            site_id: "demo",
            port: 25565,
            env: &env,
        };
        let out = apply_plain_parser("server-port=8080\nmotd=hi", &file, &ctx);
        assert!(out.contains("server-port=25565"));
        assert!(out.contains("motd=hi"));
    }

    #[test]
    fn sanitize_rejects_traversal() {
        assert!(sanitize_relative_path("../etc/passwd").is_err());
    }
}
