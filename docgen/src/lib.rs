mod api;
mod html;
mod plugins;

use html::Section;
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
    html::write(path, "Home", Section::Root, &hub_body())
}

fn hub_body() -> String {
    format!(
        "{title}
<p>Auto-generated from the OpenAPI spec and plugin SDK. <a href=\"https://mythicalltd.github.io/featherfly/\">GitHub Pages</a>.</p>
<h2>Plugin documentation</h2>
{plugin_links}
<h2>HTTP API</h2>
{api_links}
<p class=\"text-sm text-zinc-500\">Plugin API version {version} · run <code>make docs</code> to regenerate</p>",
        title = html::page_title("FeatherFly documentation", "Daemon API and plugin developer reference."),
        plugin_links = html::link_list(&[
            ("plugins/index.html", "Plugin guide", "Full plugin documentation index"),
            ("plugins/events.html", "Events", "All lifecycle events plugins can use"),
            ("plugins/json-hooks.html", "JSON hooks", "Modify responses and actions"),
            ("plugins/example.html", "Example", "Complete plugin source"),
        ]),
        api_links = html::link_list(&[
            ("api/index.html", "Swagger UI", "Interactive API explorer"),
            ("api/endpoints.html", "Endpoints", "curl examples for every route"),
            ("api/openapi.json", "OpenAPI JSON", "Machine-readable schema"),
        ]),
        version = featherfly_plugin_sdk::metadata::plugin_api_version(),
    )
}
