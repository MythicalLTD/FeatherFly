mod api;
mod html;
mod plugins;
mod plugins_txt;
mod search;
mod sitemap;
mod tests;

use html::PageContext;
use std::fs;
use std::path::{Path, PathBuf};

pub use api::generate_api_docs;
pub use plugins::generate_plugin_docs;
pub use plugins_txt::write_plugins_reference;
pub use tests::generate_test_docs;

pub fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs")
}

pub fn generate_all(output: &Path, openapi: &utoipa::openapi::OpenApi) -> std::io::Result<()> {
    clean_output_dir(output)?;
    fs::create_dir_all(output.join("plugins"))?;
    fs::create_dir_all(output.join("api"))?;

    generate_plugin_docs(&output.join("plugins"))?;
    write_plugins_reference(&output.join("plugins-reference.txt"))?;
    generate_api_docs(&output.join("api"), openapi)?;
    generate_test_docs(&output.join("tests"))?;
    write_hub(&output.join("index.html"))?;
    search::write_index(output)?;
    sitemap::write(output)?;
    write_robots(output)?;

    fs::write(output.join(".nojekyll"), "")?;

    Ok(())
}

pub fn generate_minimal(output: &Path) -> std::io::Result<()> {
    clean_output_dir(output)?;
    fs::create_dir_all(output.join("plugins"))?;
    generate_plugin_docs(&output.join("plugins"))?;
    write_plugins_reference(&output.join("plugins-reference.txt"))?;
    write_hub(&output.join("index.html"))?;
    search::write_index(output)?;
    sitemap::write(output)?;
    write_robots(output)?;
    fs::write(output.join(".nojekyll"), "")
}

fn clean_output_dir(output: &Path) -> std::io::Result<()> {
    if output.exists() {
        fs::remove_dir_all(output)?;
    }
    fs::create_dir_all(output)
}

fn write_robots(output: &Path) -> std::io::Result<()> {
    let site = std::env::var("DOCS_SITE_URL")
        .unwrap_or_else(|_| "https://mythicaltld.github.io/featherfly".into());
    let site = site.trim_end_matches('/');
    fs::write(
        output.join("robots.txt"),
        format!("User-agent: *\nAllow: /\nSitemap: {site}/sitemap.xml\n"),
    )
}

fn write_hub(path: &Path) -> std::io::Result<()> {
    html::write_page(
        path,
        "Home",
        PageContext::root("home"),
        &html::PageMeta::new(
            "FeatherFly documentation — bare daemon and native plugin SDK reference.",
            "index.html",
        )
        .with_source("Repository", ""),
        &hub_body(),
    )
}

fn hub_body() -> String {
    format!(
        "{header}
{plugin_cards}
<h2>Reference</h2>
{ref_cards}
<p class=\"text-xs text-[#555555] mt-8\">Plugin API v{version} · {repo} · run <code>make docs</code> to regenerate</p>",
        header = html::page_header(
            "FeatherFly documentation",
            "Bare daemon for CloudPanel status integration — native plugin SDK reference.",
        ),
        plugin_cards = html::card_grid(&[
            (
                "plugins/getting-started.html",
                "Getting started",
                "Setup, build, install — start here.",
            ),
            (
                "plugins/terminology.html",
                "Terminology",
                "Hooks, mixins, pipeline vocabulary.",
            ),
            (
                "plugins/architecture.html",
                "Architecture",
                "Config, request, route, and JSON pipelines.",
            ),
            (
                "plugins/config-hooks/index.html",
                "Config hooks",
                "Rewrite YAML before apply.",
            ),
            (
                "plugins/request-hooks/index.html",
                "Request hooks",
                "Intercept and middleware layers.",
            ),
            (
                "plugins/routes/index.html",
                "Plugin routes",
                "Register HTTP endpoints.",
            ),
            (
                "plugins/json-hooks/index.html",
                "JSON hooks",
                "Modify responses and action steps.",
            ),
            (
                "plugins/events/index.html",
                "Lifecycle events",
                "config.loaded, daemon.started, and more.",
            ),
            (
                "plugins/host-api.html",
                "Host API",
                "HostApi, macros, return codes.",
            ),
            (
                "plugins/source-tree.html",
                "Source tree",
                "Where to read implementation code.",
            ),
            (
                "plugins/example.html",
                "Full example",
                "v4 plugin with every hook type.",
            ),
        ]),
        ref_cards = html::card_grid(&[
            (
                "plugins/macros.html",
                "Macros",
                "hook!, hook_config!, route!, hook_json!",
            ),
            (
                "plugins/return-codes.html",
                "Return codes",
                "Every hook return value explained.",
            ),
            (
                "plugins/hooks-roadmap.html",
                "Hooks roadmap",
                "All available plugin hooks.",
            ),
            (
                "plugins-reference.txt",
                "Plugin reference (TXT)",
                "Complete plain-text hook catalog — every lifecycle event.",
            ),
        ]),
        version = featherfly_plugin_sdk::metadata::plugin_api_version(),
        repo = html::github_source("", "GitHub repository"),
    )
}
