use std::path::Path;

pub fn write(path: &Path, title: &str, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(
        path,
        format!(
            r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{ color-scheme: light dark; font-family: Inter, system-ui, sans-serif; line-height: 1.6; }}
    body {{ margin: 0; background: #0b1020; color: #eef2ff; }}
    main {{ max-width: 980px; margin: 0 auto; padding: 32px 24px 80px; }}
    a {{ color: #8fb0ff; }}
    h1, h2, h3 {{ line-height: 1.25; }}
    code, pre {{ font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.92rem; }}
    pre {{ background: #10182d; border: 1px solid #243055; border-radius: 10px; padding: 16px; overflow-x: auto; }}
    code {{ background: #10182d; padding: 2px 6px; border-radius: 6px; }}
    .nav {{ margin-bottom: 24px; color: #b8c0dc; }}
    .lead {{ color: #b8c0dc; font-size: 1.05rem; }}
    .meta {{ color: #8a93b0; font-size: 0.9rem; }}
    table {{ width: 100%; border-collapse: collapse; margin: 16px 0; }}
    th, td {{ border: 1px solid #243055; padding: 10px 12px; text-align: left; vertical-align: top; }}
    th {{ background: #141b31; }}
    .grid {{ display: grid; gap: 16px; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); margin-top: 24px; }}
    a.card {{ display: block; padding: 20px; border-radius: 14px; background: #141b31; border: 1px solid #243055; text-decoration: none; color: inherit; }}
    a.card:hover {{ border-color: #5b7cfa; }}
    a.card h2 {{ margin: 0 0 8px; font-size: 1.15rem; }}
    a.card p {{ margin: 0; color: #b8c0dc; font-size: 0.95rem; }}
    .endpoint {{ border: 1px solid #243055; border-radius: 12px; padding: 16px; margin: 16px 0; background: #10182d; }}
    .method {{ display: inline-block; padding: 2px 8px; border-radius: 6px; background: #243055; font-weight: 600; text-transform: uppercase; font-size: 0.8rem; }}
  </style>
</head>
<body>
  <main>
    {body}
  </main>
</body>
</html>"##
        ),
    )
}

pub fn nav_root() -> String {
    r#"<p class="nav"><a href="plugins/index.html">Plugin hooks</a> · <a href="api/index.html">HTTP API</a> · <a href="api/endpoints.html">Endpoint reference</a></p>"#.to_string()
}

pub fn nav_plugins() -> String {
    r#"<p class="nav"><a href="../index.html">Docs home</a> · <a href="events.html">Events</a> · <a href="json-hooks.html">JSON hooks</a> · <a href="getting-started.html">Getting started</a> · <a href="../api/index.html">HTTP API</a></p>"#.to_string()
}

pub fn nav_api() -> String {
    r#"<p class="nav"><a href="../index.html">Docs home</a> · <a href="endpoints.html">Endpoints</a> · <a href="openapi.json">OpenAPI JSON</a> · <a href="../plugins/index.html">Plugin hooks</a></p>"#.to_string()
}
