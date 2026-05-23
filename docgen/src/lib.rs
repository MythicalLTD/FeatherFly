mod api;
mod html;
mod plugins;

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
    html::write(path, "FeatherFly Docs", &hub_body())
}

fn hub_body() -> String {
    format!(
        "{nav}<h1>FeatherFly Docs</h1>
<p class=\"lead\">Auto-generated from the daemon OpenAPI spec and plugin SDK metadata. Published on <a href=\"https://mythicalltd.github.io/featherfly/\">GitHub Pages</a>.</p>
<div class=\"grid\">
  <a class=\"card\" href=\"api/index.html\"><h2>HTTP API</h2><p>Swagger UI, OpenAPI JSON, and curl examples for every route.</p></a>
  <a class=\"card\" href=\"plugins/index.html\"><h2>Plugin Hooks</h2><p>Lifecycle events, JSON mutation hooks, SDK macros, and copy-paste examples.</p></a>
  <a class=\"card\" href=\"api/endpoints.html\"><h2>Endpoint Reference</h2><p>Full route list with auth requirements and example requests.</p></a>
</div>
<p class=\"meta\">Plugin API version: {version} · regenerate with <code>make docs</code></p>",
        nav = html::nav_root(),
        version = featherfly_plugin_sdk::metadata::plugin_api_version(),
    )
}
