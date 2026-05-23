mod api;
mod html;
mod plugins;

use html::PageContext;
use std::fs;
use std::path::{Path, PathBuf};

pub use api::generate_api_docs;
pub use plugins::generate_plugin_docs;

pub fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs")
}

pub fn generate_all(output: &Path, openapi: &utoipa::openapi::OpenApi) -> std::io::Result<()> {
    fs::create_dir_all(output.join("plugins"))?;
    fs::create_dir_all(output.join("api"))?;

    generate_plugin_docs(&output.join("plugins"))?;
    generate_api_docs(&output.join("api"), openapi)?;
    write_hub(&output.join("index.html"))?;

    Ok(())
}

pub fn generate_minimal(output: &Path) -> std::io::Result<()> {
    fs::create_dir_all(output.join("plugins"))?;
    generate_plugin_docs(&output.join("plugins"))?;
    write_hub(&output.join("index.html"))
}

fn write_hub(path: &Path) -> std::io::Result<()> {
    html::write(path, "Home", PageContext::root("home"), &hub_body())
}

fn hub_body() -> String {
    format!(
        "{header}
{plugin_cards}
<h2>HTTP API</h2>
{api_cards}
<p class=\"text-xs text-zinc-500 mt-8\">Plugin API v{version} · run <code>make docs</code> to regenerate</p>",
        header = html::page_header(
            "FeatherFly documentation",
            "Daemon HTTP API and native plugin developer reference.",
        ),
        plugin_cards = html::card_grid(&[
            (
                "plugins/index.html",
                "Plugin guide",
                "Build and extend FeatherFly with native .so plugins.",
            ),
            (
                "plugins/getting-started.html",
                "Getting started",
                "Setup, build, install — start here.",
            ),
            (
                "plugins/events/index.html",
                "Lifecycle events",
                "config.loaded, daemon.started, and more.",
            ),
            (
                "plugins/json-hooks/index.html",
                "JSON hooks",
                "Modify responses and action steps.",
            ),
            (
                "plugins/example.html",
                "Example plugin",
                "Complete source with events and JSON hooks.",
            ),
        ]),
        api_cards = html::card_grid(&[
            (
                "api/index.html",
                "Swagger UI",
                "Interactive API explorer.",
            ),
            (
                "api/endpoints.html",
                "Endpoints",
                "curl examples for every route.",
            ),
            (
                "api/openapi.json",
                "OpenAPI JSON",
                "Machine-readable schema.",
            ),
        ]),
        version = featherfly_plugin_sdk::metadata::plugin_api_version(),
    )
}
