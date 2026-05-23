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
        router_source: "application/src/routes/mod.rs",
        routes: &[RouteDoc {
            method: "GET",
            path: "/health",
            operation_id: "get_health",
            title: "Health check",
            description: "Returns daemon status, service name, and build version. No authentication. Use for uptime monitors and readiness probes.",
            auth: "None",
            response: r#"{"status":"ok","service":"featherfly","version":"0.1.0 (abc1234)"}"#,
            source: "application/src/controllers/health.rs",
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
            source: "application/src/controllers/system/root.rs",
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
            source: "application/src/controllers/system/overview.rs",
        }],
    },
    ApiGroup {
        id: "api-system-plugins",
        title: "Plugins",
        summary: "Inspect loaded plugins and reload after installing new .so files.",
        slug: "system-plugins",
        router_source: "application/src/routes/api/system/plugins.rs",
        routes: &[
            RouteDoc {
                method: "GET",
                path: "/api/system/plugins",
                operation_id: "get_system_plugins",
                title: "List plugins",
                description: "Lists every loaded `.so` plugin with name, version, hook count, and path. Returns lifecycle events, JSON targets, request phases, plugin route count, and loaded plugin summaries.",
                auth: "Bearer token",
                response: r#"{"enabled":true,"directory":"/var/lib/featherfly/plugins","hooks":3,"events":["config.loaded",...],"json_targets":["json.response","json.actions"],"request_phases":["request.intercept","middleware.inject"],"plugin_routes":1,"plugins":[...],"actions":[...]}"#,
                source: "application/src/controllers/system/plugins.rs",
            },
            RouteDoc {
                method: "POST",
                path: "/api/system/plugins/reload",
                operation_id: "post_system_plugins_reload",
                title: "Reload plugins",
                description: "Schedules a daemon restart so newly installed or updated `.so` files are loaded. Plugins are only read at startup — this endpoint does not hot-reload in-process. Requires `remote.restart: true`. Returns 202 Accepted.",
                auth: "Bearer token",
                response: r#"{"scheduled":true,"delay_ms":750,"note":"plugins are loaded at startup; a daemon restart is required to pick up changes"}"#,
                source: "application/src/controllers/system/plugins.rs",
            },
        ],
    },
    ApiGroup {
        id: "api-system-config",
        title: "Remote config",
        summary: "Read and edit config.yml from FeatherPanel without SSH.",
        slug: "system-config",
        router_source: "application/src/routes/api/system/config.rs",
        routes: &[
            RouteDoc {
                method: "GET",
                path: "/api/system/config",
                operation_id: "get_system_config",
                title: "Read configuration",
                description: "Returns the active config as YAML. The bearer token is redacted as `***`. Requires `remote.config_edit: true` (also gates writes).",
                auth: "Bearer token",
                response: r#"{"path":"/etc/featherfly/config.yml","yaml":"app_name: FeatherFly\n...","editable":true}"#,
                source: "application/src/controllers/system/config.rs",
            },
            RouteDoc {
                method: "PUT",
                path: "/api/system/config",
                operation_id: "put_system_config",
                title: "Apply configuration",
                description: "Validates and writes a full config YAML snapshot. Set `restart_if_required: true` to auto-restart when listen address, logging, plugins directory, or similar fields change. Token value `***` preserves the existing secret.",
                auth: "Bearer token",
                response: r#"{"applied":true,"requires_restart":false,"restart_reasons":[],"restart_scheduled":false}"#,
                source: "application/src/controllers/system/config.rs",
            },
        ],
    },
    ApiGroup {
        id: "api-system-restart",
        title: "Remote restart",
        summary: "Gracefully restart the daemon from the panel.",
        slug: "system-restart",
        router_source: "application/src/routes/api/system/restart.rs",
        routes: &[RouteDoc {
            method: "POST",
            path: "/api/system/restart",
            operation_id: "post_system_restart",
            title: "Restart daemon",
            description: "Spawns a replacement process with the same argv and exits. Optional `delay_ms` (default 0) and `reason` for audit logs. Requires `remote.restart: true`. Returns 202 Accepted.",
            auth: "Bearer token",
            response: r#"{"scheduled":true,"delay_ms":0}"#,
            source: "application/src/controllers/system/restart.rs",
        }],
    },
    ApiGroup {
        id: "api-system-update",
        title: "Updates",
        summary: "Check GitHub for newer FeatherFly builds and apply updates.",
        slug: "system-update",
        router_source: "application/src/routes/api/system/update.rs",
        routes: &[
            RouteDoc {
                method: "GET",
                path: "/api/system/update",
                operation_id: "get_system_update",
                title: "Check for updates",
                description: "Queries GitHub releases for the configured channel (stable/nightly). Always returns 200 — set `check_ok: false` with a `message` when GitHub is unreachable or no release exists yet. Includes `download_url`, `sha256`, and version/commit fields for the panel to drive upgrades.",
                auth: "Bearer token",
                response: r#"{"check_ok":true,"update_available":false,"current_version":"0.1.0","current_commit":"abc1234","latest_version":null,"download_url":null,"sha256":null,"message":null}"#,
                source: "application/src/controllers/system/update.rs",
            },
            RouteDoc {
                method: "POST",
                path: "/api/system/update/apply",
                operation_id: "post_system_update_apply",
                title: "Apply update from GitHub",
                description: "Downloads the platform binary from the latest GitHub release, verifies SHA256, replaces the running executable, and optionally restarts the daemon. Requires `remote.upgrade: true`. Returns 202 Accepted while the download runs in the background.",
                auth: "Bearer token",
                response: r#"{"scheduled":true,"restart":true,"delay_ms":0,"channel":"stable","update_available":true}"#,
                source: "application/src/controllers/system/update.rs",
            },
        ],
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
            source: "application/src/controllers/system/upgrade.rs",
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
        auth = html::github_source("application/src/middlewares/auth.rs", "Auth middleware"),
        response = html::github_source("application/src/utils/response.rs", "JSON responses"),
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
    if route.method == "POST" && route.path.contains("/update/apply") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push("-d '{\"restart\":true,\"delay_ms\":750}'".to_string());
    } else if route.method == "POST" && route.path.contains("/upgrade") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push(
            "-d '{\"url\":\"https://example.com/featherfly\",\"sha256\":\"...\",\"restart_command\":\"systemctl\",\"restart_command_args\":[\"restart\",\"featherfly\"]}'".to_string(),
        );
    } else if route.method == "PUT" && route.path.contains("/config") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push(
            "-d '{\"yaml\":\"app_name: FeatherFly\\n...\",\"restart_if_required\":true}'"
                .to_string(),
        );
    } else if route.method == "POST" && route.path.contains("/restart") {
        lines.push("-H 'Content-Type: application/json' \\".to_string());
        lines.push("-d '{\"delay_ms\":500,\"reason\":\"panel maintenance\"}'".to_string());
    } else if route.method == "POST" && route.path.contains("/reload") {
        lines.push("-H 'Content-Type: application/json'".to_string());
    }
    lines.join("\n")
}
