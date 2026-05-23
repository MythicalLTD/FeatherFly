use crate::html::{self, PageContext, PageMeta};
use std::path::Path;

pub struct RouteDoc {
    pub method: &'static str,
    pub path: &'static str,
    pub operation_id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub auth: &'static str,
    pub response: &'static str,
    pub source: &'static str,
}

pub struct ApiGroup {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub slug: &'static str,
    pub router_source: &'static str,
    pub routes: &'static [RouteDoc],
}

pub const GROUPS: &[ApiGroup] = &[
    ApiGroup {
        id: "api-health",
        title: "Health",
        summary: "Unauthenticated liveness probe for load balancers and panels.",
        slug: "health",
        router_source: "application/src/routes/health.rs",
        routes: &[RouteDoc {
            method: "GET",
            path: "/health",
            operation_id: "get_health",
            title: "Health check",
            description: "Returns daemon status, service name, and build version. No authentication. Use for uptime monitors and readiness probes.",
            auth: "None",
            response: r#"{"status":"ok","service":"featherfly","version":"0.1.0 (abc1234)"}"#,
            source: "application/src/routes/health.rs",
        }],
    },
    ApiGroup {
        id: "api-system",
        title: "System",
        summary: "Core system information exposed to the FeatherPanel.",
        slug: "system",
        router_source: "application/src/routes/api/system/mod.rs",
        routes: &[RouteDoc {
            method: "GET",
            path: "/api/system",
            operation_id: "get_system",
            title: "System summary",
            description: "High-level host facts: architecture, CPU count, kernel, OS, daemon version. Includes an `actions` array the panel uses for follow-up steps (overview, update check, plugins, diagnostics).",
            auth: "Bearer token",
            response: r#"{"architecture":"x86_64","cpu_count":4,"kernel_version":"...","os":"linux","version":"0.1.0","actions":[...]}"#,
            source: "application/src/routes/api/system/mod.rs",
        }],
    },
    ApiGroup {
        id: "api-system-overview",
        title: "System overview",
        summary: "Detailed CPU and memory metrics for dashboards.",
        slug: "system-overview",
        router_source: "application/src/routes/api/system/overview.rs",
        routes: &[RouteDoc {
            method: "GET",
            path: "/api/system/overview",
            operation_id: "get_system_overview",
            title: "System overview",
            description: "Live CPU brand/frequency, memory totals, process RSS, container detection, and local time. Heavier than `/api/system` — use when the panel needs charts.",
            auth: "Bearer token",
            response: r#"{"version":"0.1.0","local_time":"...","container_type":"None","cpu":{...},"memory":{...}}"#,
            source: "application/src/routes/api/system/overview.rs",
        }],
    },
    ApiGroup {
        id: "api-system-plugins",
        title: "Plugins",
        summary: "Inspect loaded plugins and available hook targets.",
        slug: "system-plugins",
        router_source: "application/src/routes/api/system/plugins.rs",
        routes: &[RouteDoc {
            method: "GET",
            path: "/api/system/plugins",
            operation_id: "get_system_plugins",
            title: "List plugins",
            description: "Lists every loaded `.so` plugin with name, version, hook count, and path. Returns lifecycle events, JSON targets, request phases, plugin route count, and loaded plugin summaries.",
            auth: "Bearer token",
            response: r#"{"enabled":true,"directory":"/var/lib/featherfly/plugins","hooks":3,"events":["config.loaded",...],"json_targets":["json.response","json.actions"],"request_phases":["request.intercept","middleware.inject"],"plugin_routes":1,"plugins":[...]}"#,
            source: "application/src/routes/api/system/plugins.rs",
        }],
    },
    ApiGroup {
        id: "api-system-update",
        title: "Updates",
        summary: "Check GitHub for newer FeatherFly builds.",
        slug: "system-update",
        router_source: "application/src/routes/api/system/update.rs",
        routes: &[RouteDoc {
            method: "GET",
            path: "/api/system/update",
            operation_id: "get_system_update",
            title: "Check for updates",
            description: "Compares the running version against GitHub releases (stable or nightly channel from config). Returns whether an update exists, latest version string, and download URL.",
            auth: "Bearer token",
            response: r#"{"update_available":false,"current_version":"0.1.0","latest_version":"0.1.0","url":null}"#,
            source: "application/src/routes/api/system/update.rs",
        }],
    },
    ApiGroup {
        id: "api-system-upgrade",
        title: "Upgrade",
        summary: "Remote binary upgrade initiated by the panel.",
        slug: "system-upgrade",
        router_source: "application/src/routes/api/system/upgrade.rs",
        routes: &[RouteDoc {
            method: "POST",
            path: "/api/system/upgrade",
            operation_id: "post_system_upgrade",
            title: "Apply upgrade",
            description: "Downloads a replacement binary from `url`, verifies SHA-256, replaces the running executable, and spawns a restart command. Rejected in containerized environments. Returns 202 Accepted when the background upgrade task starts.",
            auth: "Bearer token",
            response: r#"{"applied":true}"#,
            source: "application/src/routes/api/system/upgrade.rs",
        }],
    },
];

pub fn generate_api_docs(output: &Path, openapi: &utoipa::openapi::OpenApi) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;

    let openapi_json = serde_json::to_string_pretty(openapi).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(output.join("openapi.json"), openapi_json)?;

    html::write_page(
        &output.join("index.html"),
        "HTTP API",
        PageContext::api("api-overview"),
        &PageMeta::new(
            "FeatherFly HTTP API — REST endpoints for FeatherPanel and automation.",
            "api/index.html",
        )
        .with_source("Router", "application/src/routes/mod.rs"),
        &overview_page(openapi),
    )?;

    for group in GROUPS {
        html::write_page(
            &output.join(format!("{}.html", group.slug)),
            group.title,
            PageContext::api(group.id),
            &PageMeta::new(
                format!("{} — FeatherFly HTTP API", group.title),
                format!("api/{}.html", group.slug),
            )
            .with_source("Router", group.router_source),
            &group_page(group),
        )?;
    }

    let _ = std::fs::remove_file(output.join("endpoints.html"));

    Ok(())
}

fn overview_page(openapi: &utoipa::openapi::OpenApi) -> String {
    let mut cards = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    for group in GROUPS {
        cards.push_str(&format!(
            r#"<a href="{}.html" class="doc-card"><strong class="block mb-1 text-white">{title}</strong><span class="text-sm text-[#888888]">{summary}</span></a>"#,
            group.slug, title = group.title, summary = group.summary,
        ));
    }
    cards.push_str("</div>");

    format!(
        "{header}
<p>FeatherFly exposes a JSON HTTP API consumed by FeatherPanel. OpenAPI <code>{version}</code> · <a href=\"openapi.json\">Download JSON</a> · {router}</p>
<h2>Authentication</h2>
<p>All routes except <code>/health</code> require <code>Authorization: Bearer &lt;token&gt;</code> matching <code>api.token</code> in config. {auth}</p>
<h2>Response shape</h2>
<p>Successful responses wrap data in the standard envelope. JSON mutation hooks run on the inner object before it is sent. {response}</p>
<h2>Route groups</h2>
{cards}
<h2>Plugin integration</h2>
<p>Plugins can add routes via <a href=\"../plugins/routes/index.html\">route.register</a>, intercept requests, and mutate JSON responses. See <a href=\"../plugins/index.html\">plugin docs</a>.</p>",
        header = html::page_header(
            "HTTP API",
            "REST endpoints for panels, automation, and diagnostics.",
        ),
        version = openapi.info.version,
        cards = cards,
        router = html::github_source("application/src/routes/mod.rs", "Route tree"),
        auth = html::github_source("application/src/auth.rs", "Auth middleware"),
        response = html::github_source("application/src/plugins/middleware.rs", "JSON hook middleware"),
    )
}

fn group_page(group: &ApiGroup) -> String {
    let mut routes = String::new();
    for route in group.routes {
        routes.push_str(&route_section(route));
    }

    format!(
        "{header}
<p>{summary}</p>
{routes}
<p><a href=\"index.html\">← HTTP API overview</a></p>",
        header = html::page_header(group.title, group.summary),
        summary = group.summary,
        routes = routes,
    )
}

fn route_section(route: &RouteDoc) -> String {
    format!(
        "<h2>{title} — <code>{method} {path}</code></h2>
{meta}
<p>{description}</p>
<p>{source}</p>
<h3>Example response</h3>
<pre><code>{response}</code></pre>
<h3>curl</h3>
<pre><code>{curl}</code></pre>",
        method = route.method,
        path = route.path,
        title = route.title,
        meta = html::meta_grid(&[("Operation", route.operation_id), ("Auth", route.auth),]),
        description = route.description,
        source = html::github_source(route.source, "View handler source"),
        response = html::html_escape(route.response),
        curl = html::html_escape(&curl_example(route)),
    )
}

fn curl_example(route: &RouteDoc) -> String {
    let mut lines = vec![format!(
        "curl -X {} 'http://127.0.0.1:9090{}'",
        route.method, route.path
    )];
    if route.auth.contains("Bearer") {
        lines.push("-H 'Authorization: Bearer YOUR_TOKEN' \\".to_string());
    }
    if route.method == "POST" && route.path.contains("/upgrade") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push(
            "-d '{\"url\":\"https://example.com/featherfly\",\"sha256\":\"...\",\"restart_command\":\"systemctl\",\"restart_command_args\":[\"restart\",\"featherfly\"]}'".to_string(),
        );
    }
    lines.join("\n")
}
