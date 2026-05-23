use crate::html::{self, PageContext};
use std::path::Path;

pub fn generate_api_docs(output: &Path, openapi: &utoipa::openapi::OpenApi) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;

    let openapi_json = serde_json::to_string_pretty(openapi).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(output.join("openapi.json"), openapi_json)?;

    html::write(
        &output.join("index.html"),
        "HTTP API",
        PageContext::api("api-swagger"),
        &swagger_page(),
    )?;

    html::write(
        &output.join("endpoints.html"),
        "Endpoints",
        PageContext::api("api-endpoints"),
        &endpoints_page(openapi),
    )?;

    Ok(())
}

fn swagger_page() -> String {
    format!(
        r##"{title}
<link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui.css">
<div class="border border-zinc-300 dark:border-zinc-700 rounded my-4 -mx-4 sm:mx-0">
  <div id="swagger-ui"></div>
</div>
<script src="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui-bundle.js"></script>
<script>
window.onload = () => {{
  SwaggerUIBundle({{
    url: "openapi.json",
    dom_id: "#swagger-ui",
    deepLinking: true,
    persistAuthorization: true,
    presets: [SwaggerUIBundle.presets.apis],
    layout: "BaseLayout",
  }});
}};
</script>"##,
        title = html::page_header("HTTP API", "Interactive OpenAPI documentation."),
    )
}

fn endpoints_page(openapi: &utoipa::openapi::OpenApi) -> String {
    let mut sections = String::new();

    for (path, item) in &openapi.paths.paths {
        for (method, operation) in operations(item) {
            let Some(operation) = operation else { continue };
            let summary = operation.summary.as_deref().unwrap_or("—");
            let operation_id = operation.operation_id.as_deref().unwrap_or("—");
            let secured = operation
                .security
                .as_ref()
                .is_some_and(|items| !items.is_empty());
            let auth = if secured {
                "Bearer token required"
            } else {
                "No auth"
            };

            sections.push_str(&html::endpoint_block(
                method,
                path,
                operation_id,
                summary,
                auth,
                &example_for(path, method, secured),
            ));
        }
    }

    format!(
        "{title}
<p>OpenAPI <code>{version}</code> · <a href=\"openapi.json\">Download JSON</a></p>
{sections}",
        title = html::page_header("Endpoints", "All routes with curl examples."),
        version = openapi.info.version,
        sections = sections,
    )
}

fn operations(
    item: &utoipa::openapi::path::PathItem,
) -> [(&'static str, Option<&utoipa::openapi::path::Operation>); 5] {
    [
        ("GET", item.get.as_ref()),
        ("POST", item.post.as_ref()),
        ("PUT", item.put.as_ref()),
        ("PATCH", item.patch.as_ref()),
        ("DELETE", item.delete.as_ref()),
    ]
}

fn example_for(path: &str, method: &str, secured: bool) -> String {
    let mut lines = vec![format!("curl -X {method} 'http://127.0.0.1:8080{path}'")];
    if secured {
        lines.push("-H 'Authorization: Bearer YOUR_TOKEN' \\".to_string());
    }
    if method == "POST" && path.contains("/upgrade") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push(
            "-d '{\"url\":\"https://example.com/featherfly\",\"sha256\":\"...\",\"restart_command\":\"systemctl\",\"restart_command_args\":[\"restart\",\"featherfly\"]}'".to_string(),
        );
    }
    lines.join("\n")
}
