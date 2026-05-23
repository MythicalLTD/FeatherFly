use crate::html;
use std::path::Path;

pub fn generate_api_docs(output: &Path, openapi: &utoipa::openapi::OpenApi) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;

    let openapi_json = serde_json::to_string_pretty(openapi).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(output.join("openapi.json"), openapi_json)?;

    html::write(
        &output.join("index.html"),
        "FeatherFly API",
        &swagger_page(),
    )?;

    html::write(
        &output.join("endpoints.html"),
        "API Endpoints",
        &endpoints_page(openapi),
    )?;

    Ok(())
}

fn swagger_page() -> String {
    format!(
        "{nav}
<h1>HTTP API</h1>
<p class=\"lead\">Interactive Swagger UI backed by the auto-generated OpenAPI spec from utoipa route annotations.</p>
<link rel=\"stylesheet\" href=\"https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui.css\">
<div id=\"swagger-ui\"></div>
<script src=\"https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui-bundle.js\"></script>
<script>
window.onload = () => {{
  SwaggerUIBundle({{
    url: \"openapi.json\",
    dom_id: \"#swagger-ui\",
    deepLinking: true,
    persistAuthorization: true,
    presets: [SwaggerUIBundle.presets.apis],
    layout: \"BaseLayout\",
  }});
}};
</script>",
        nav = html::nav_api(),
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

            let example = example_for(path, method, secured);
            sections.push_str(&format!(
                r#"<div class="endpoint"><span class="method">{method}</span> <code>{path}</code>
<h3>{operation_id}</h3>
<p>{summary}</p>
<p><strong>Auth:</strong> {auth}</p>
<h4>Example</h4>
<pre><code>{example}</code></pre>
</div>"#,
            ));
        }
    }

    format!(
        "{nav}<h1>Endpoint Reference</h1>
<p class=\"lead\">Generated from the same OpenAPI document served at <a href=\"openapi.json\">openapi.json</a>. Version <code>{version}</code>.</p>
{sections}",
        nav = html::nav_api(),
        version = openapi.info.version,
        sections = sections,
    )
}

fn operations(
    item: &utoipa::openapi::path::PathItem,
) -> [(&'static str, Option<&utoipa::openapi::path::Operation>); 5] {
    [
        ("get", item.get.as_ref()),
        ("post", item.post.as_ref()),
        ("put", item.put.as_ref()),
        ("patch", item.patch.as_ref()),
        ("delete", item.delete.as_ref()),
    ]
}

fn example_for(path: &str, method: &str, secured: bool) -> String {
    let mut lines = vec![format!("curl -X {method} 'http://127.0.0.1:8080{path}'")];
    if secured {
        lines.push("-H 'Authorization: Bearer YOUR_TOKEN' \\".to_string());
    }
    if method == "post" && path.contains("/upgrade") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push(
            "-d '{\"url\":\"https://example.com/featherfly\",\"sha256\":\"...\",\"restart_command\":\"systemctl\",\"restart_command_args\":[\"restart\",\"featherfly\"]}'".to_string(),
        );
    }
    lines.join("\n")
}
