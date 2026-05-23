use crate::html::{self, PageContext, PageMeta};
use featherfly_plugin_sdk::metadata::{
    ARCHITECTURE, BUILD_AND_INSTALL, CONFIG_HOOK_DOCS, EVENT_DOCS, FULL_PLUGIN_EXAMPLE, HOST_API,
    JSON_HOOK_DOCS, LIFECYCLE, MACROS, OVERVIEW, PLANNED_HOOKS, REQUEST_HOOK_DOCS,
    REQUEST_PIPELINE, RETURN_CODES, ROUTE_HOOK_DOCS, SOURCE_TREE, TERMINOLOGY, plugin_api_version,
};
use std::path::Path;

pub fn generate_plugin_docs(output: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;
    std::fs::create_dir_all(output.join("events"))?;
    std::fs::create_dir_all(output.join("json-hooks"))?;
    std::fs::create_dir_all(output.join("config-hooks"))?;
    std::fs::create_dir_all(output.join("request-hooks"))?;
    std::fs::create_dir_all(output.join("routes"))?;

    write_page(
        &output.join("index.html"),
        "Plugins",
        PageContext::plugins("plugins"),
        PageMeta::new(
            "Native FeatherFly plugin documentation — hooks, routes, config mutation, JSON mixins.",
            "plugins/index.html",
        )
        .with_source("Plugin SDK", "featherfly-plugin-sdk/src/lib.rs"),
        &index_page(),
    )?;
    write_page(
        &output.join("getting-started.html"),
        "Getting started",
        PageContext::plugins("getting-started"),
        PageMeta::new(
            "Build, install, and ship FeatherFly plugins as native .so libraries.",
            "plugins/getting-started.html",
        )
        .with_source("Hello plugin", "plugins/hello/src/lib.rs"),
        &getting_started_page(),
    )?;
    write_page(
        &output.join("overview.html"),
        "Overview",
        PageContext::plugins("overview"),
        PageMeta::new(
            "How FeatherFly loads plugins, hook pipelines, and startup lifecycle.",
            "plugins/overview.html",
        )
        .with_source("Plugin loader", "application/src/plugins/mod.rs"),
        &overview_page(),
    )?;
    write_page(
        &output.join("terminology.html"),
        "Terminology",
        PageContext::plugins("terminology"),
        PageMeta::new(
            "Plugin development vocabulary — hooks, mixins, pipelines, load order.",
            "plugins/terminology.html",
        ),
        &terminology_page(),
    )?;
    write_page(
        &output.join("architecture.html"),
        "Architecture",
        PageContext::plugins("architecture"),
        PageMeta::new(
            "Mixin-style hook pipelines for config, HTTP requests, routes, and JSON responses.",
            "plugins/architecture.html",
        )
        .with_source("Event bus", "application/src/plugins/events.rs"),
        &architecture_page(),
    )?;
    write_page(
        &output.join("hooks-roadmap.html"),
        "Hooks roadmap",
        PageContext::plugins("hooks-roadmap"),
        PageMeta::new(
            "All FeatherFly plugin hooks — available in API v4.",
            "plugins/hooks-roadmap.html",
        ),
        &hooks_roadmap_page(),
    )?;

    write_page(
        &output.join("events/index.html"),
        "Lifecycle events",
        PageContext::plugin_events("events"),
        PageMeta::new(
            "Lifecycle event hooks — config.loaded, daemon.started, plugin.loaded.",
            "plugins/events/index.html",
        )
        .with_source("Event dispatch", "application/src/plugins/events.rs"),
        &events_index_page(),
    )?;
    for doc in EVENT_DOCS {
        let slug = html::event_slug(doc.name);
        write_page(
            &output.join(format!("events/{slug}.html")),
            doc.name,
            PageContext::plugin_events(event_active_id(doc.name)),
            PageMeta::new(
                format!("{} — FeatherFly lifecycle event hook", doc.name),
                format!("plugins/events/{slug}.html"),
            )
            .with_source("Events", "application/src/plugins/events.rs"),
            &event_detail_page(doc),
        )?;
    }

    write_page(
        &output.join("config-hooks/index.html"),
        "Config hooks",
        PageContext::plugin_config_hooks("config-hooks"),
        PageMeta::new(
            "Config mutation hooks — rewrite YAML before FeatherFly applies settings.",
            "plugins/config-hooks/index.html",
        )
        .with_source("Config pipeline", "application/src/daemon.rs"),
        &config_hooks_index_page(),
    )?;
    for doc in CONFIG_HOOK_DOCS {
        write_page(
            &output.join("config-hooks/mutate.html"),
            doc.name,
            PageContext::plugin_config_hooks("config-mutate"),
            PageMeta::new(
                format!("{} — rewrite config YAML before apply", doc.name),
                "plugins/config-hooks/mutate.html",
            )
            .with_source("Implementation", doc.source_path),
            &hook_guide_page(doc),
        )?;
    }

    write_page(
        &output.join("request-hooks/index.html"),
        "Request hooks",
        PageContext::plugin_request_hooks("request-hooks"),
        PageMeta::new(
            "HTTP request hooks — request.intercept and middleware.inject.",
            "plugins/request-hooks/index.html",
        )
        .with_source(
            "Middleware",
            "application/src/plugins/request_middleware.rs",
        ),
        &request_hooks_index_page(),
    )?;
    for doc in REQUEST_HOOK_DOCS {
        let (slug, active) = request_hook_paths(doc.name);
        write_page(
            &output.join(format!("request-hooks/{slug}.html")),
            doc.name,
            PageContext::plugin_request_hooks(active),
            PageMeta::new(
                format!("{} — inbound HTTP request hook", doc.name),
                format!("plugins/request-hooks/{slug}.html"),
            )
            .with_source("Implementation", doc.source_path),
            &hook_guide_page(doc),
        )?;
    }

    write_page(
        &output.join("routes/index.html"),
        "Plugin routes",
        PageContext::plugin_routes("plugin-routes"),
        PageMeta::new(
            "Register new HTTP routes on the FeatherFly daemon router.",
            "plugins/routes/index.html",
        )
        .with_source("Route dispatch", "application/src/plugins/routes.rs"),
        &routes_index_page(),
    )?;
    for doc in ROUTE_HOOK_DOCS {
        write_page(
            &output.join("routes/register.html"),
            doc.name,
            PageContext::plugin_routes("route-register"),
            PageMeta::new(
                format!("{} — plugin HTTP route registration", doc.name),
                "plugins/routes/register.html",
            )
            .with_source("Implementation", doc.source_path),
            &hook_guide_page(doc),
        )?;
    }

    write_page(
        &output.join("json-hooks/index.html"),
        "JSON hooks",
        PageContext::plugin_json_hooks("json-hooks"),
        PageMeta::new(
            "JSON mutation hooks — rewrite API response bodies and action steps.",
            "plugins/json-hooks/index.html",
        )
        .with_source(
            "Response middleware",
            "application/src/plugins/middleware.rs",
        ),
        &json_hooks_index_page(),
    )?;
    for doc in JSON_HOOK_DOCS {
        let (slug, active) = json_hook_paths(doc.name);
        write_page(
            &output.join(format!("json-hooks/{slug}.html")),
            doc.name,
            PageContext::plugin_json_hooks(active),
            PageMeta::new(
                format!("{} — JSON response mutation hook", doc.name),
                format!("plugins/json-hooks/{slug}.html"),
            )
            .with_source("JSON middleware", "application/src/plugins/middleware.rs"),
            &json_hook_detail_page(doc),
        )?;
    }

    write_page(
        &output.join("host-api.html"),
        "Host API",
        PageContext::plugins("host-api"),
        PageMeta::new(
            "HostApi struct — register hooks, routes, and log from plugin init.",
            "plugins/host-api.html",
        )
        .with_source("HostApi", "featherfly-plugin-sdk/src/lib.rs"),
        &host_api_page(),
    )?;
    write_page(
        &output.join("macros.html"),
        "Macros",
        PageContext::plugins("macros"),
        PageMeta::new(
            "FeatherFly plugin SDK macros — hook!, hook_config!, route!, and more.",
            "plugins/macros.html",
        )
        .with_source("Macro definitions", "featherfly-plugin-sdk/src/lib.rs"),
        &macros_page(),
    )?;
    write_page(
        &output.join("return-codes.html"),
        "Return codes",
        PageContext::plugins("return-codes"),
        PageMeta::new(
            "Plugin hook return codes — CONFIG_MUTATE, REQUEST_CONTINUE, ROUTE_OK, JSON_MUTATE.",
            "plugins/return-codes.html",
        )
        .with_source("Constants", "featherfly-plugin-sdk/src/lib.rs"),
        &return_codes_page(),
    )?;
    write_page(
        &output.join("source-tree.html"),
        "Source tree",
        PageContext::plugins("source-tree"),
        PageMeta::new(
            "FeatherFly repository map for plugin developers — where to read the implementation.",
            "plugins/source-tree.html",
        ),
        &source_tree_page(),
    )?;
    write_page(
        &output.join("example.html"),
        "Full example",
        PageContext::plugins("example"),
        PageMeta::new(
            "Complete FeatherFly v4 plugin — config, request, route, and JSON hooks.",
            "plugins/example.html",
        )
        .with_source("Hello plugin", "plugins/hello/src/lib.rs"),
        &example_page(),
    )?;

    let _ = std::fs::remove_file(output.join("events.html"));
    let _ = std::fs::remove_file(output.join("json-hooks.html"));

    Ok(())
}

fn write_page(
    path: &Path,
    title: &str,
    ctx: PageContext,
    meta: PageMeta,
    body: &str,
) -> std::io::Result<()> {
    html::write_page(path, title, ctx, &meta, body)
}

fn request_hook_paths(name: &str) -> (&'static str, &'static str) {
    match name {
        "request.intercept" => ("intercept", "request-intercept"),
        "middleware.inject" => ("middleware", "middleware-inject"),
        _ => ("index", "request-hooks"),
    }
}

fn json_hook_paths(name: &str) -> (&'static str, &'static str) {
    match name {
        "json.response" => ("response-body", "json-response"),
        "json.actions" => ("response-actions", "json-actions"),
        _ => ("index", "json-hooks"),
    }
}

fn event_active_id(name: &str) -> &'static str {
    match name {
        "config.loaded" => "event-config-loaded",
        "plugin.loaded" => "event-plugin-loaded",
        "daemon.starting" => "event-daemon-starting",
        "daemon.started" => "event-daemon-started",
        "daemon.stopping" => "event-daemon-stopping",
        _ => "events",
    }
}

fn index_page() -> String {
    format!(
        "{header}
<p>Plugin API version <code>{version}</code>. All hooks below ship in v4.</p>
{cards}
<h2>Quick links</h2>
<ul>
<li>{sdk}</li>
<li>{hello}</li>
<li>{daemon}</li>
<li>{github}</li>
</ul>",
        header = html::page_header(
            "Plugin documentation",
            "Build native .so plugins that extend FeatherFly at runtime — config, HTTP, routes, JSON.",
        ),
        version = plugin_api_version(),
        cards = html::card_grid(&[
            (
                "getting-started.html",
                "Getting started",
                "Project setup, build commands, install paths.",
            ),
            (
                "terminology.html",
                "Terminology",
                "Hooks, mixins, pipeline, load order.",
            ),
            (
                "architecture.html",
                "Architecture",
                "Mixin-style pipelines and hook composition.",
            ),
            (
                "config-hooks/index.html",
                "Config hooks",
                "Rewrite YAML before settings apply.",
            ),
            (
                "request-hooks/index.html",
                "Request hooks",
                "Intercept and middleware.inject layers.",
            ),
            (
                "routes/index.html",
                "Plugin routes",
                "Register new HTTP endpoints.",
            ),
            (
                "json-hooks/index.html",
                "JSON hooks",
                "Modify API responses before clients see them.",
            ),
            (
                "events/index.html",
                "Lifecycle events",
                "Startup, shutdown, config notification.",
            ),
            ("host-api.html", "Host API", "HostApi fields, return codes.",),
            (
                "example.html",
                "Full example",
                "v4 plugin with every hook type.",
            ),
        ]),
        sdk = html::github_source("featherfly-plugin-sdk/src/lib.rs", "Plugin SDK source"),
        hello = html::github_source("plugins/hello/src/lib.rs", "Hello reference plugin"),
        daemon = html::github_source("application/src/daemon.rs", "Daemon startup"),
        github = html::github_source("", "Full repository"),
    )
}

fn terminology_page() -> String {
    format!(
        "{header}
{body}",
        header = html::page_header(
            "Terminology",
            "Shared vocabulary for FeatherFly plugin development.",
        ),
        body = html::text_block(TERMINOLOGY),
    )
}

fn architecture_page() -> String {
    format!(
        "{header}
{body}
<h2>HTTP request stack</h2>
{pipeline}
<p>See also <a href=\"request-hooks/index.html\">request hooks</a>, <a href=\"config-hooks/index.html\">config hooks</a>, and <a href=\"hooks-roadmap.html\">hooks roadmap</a>.</p>",
        header = html::page_header(
            "Architecture",
            "How mixin-style hook pipelines compose across plugins.",
        ),
        body = html::text_block(ARCHITECTURE),
        pipeline = html::text_block(REQUEST_PIPELINE),
    )
}

fn hooks_roadmap_page() -> String {
    let mut rows = String::new();
    for hook in PLANNED_HOOKS {
        rows.push_str(&format!(
            "<tr><td><code>{name}</code></td><td>{status}</td><td>{summary}</td><td>{role}</td></tr>",
            name = hook.name,
            status = hook.status,
            summary = hook.summary,
            role = hook.mixin_role,
        ));
    }

    format!(
        "{header}
<p>Plugin API v{version}. All hooks listed as <code>available</code> ship today in v4.</p>
<table><thead><tr><th>Hook</th><th>Status</th><th>Summary</th><th>Mixin role</th></tr></thead><tbody>{rows}</tbody></table>
<p>Documentation: <a href=\"config-hooks/index.html\">config</a> · <a href=\"request-hooks/index.html\">request</a> · <a href=\"routes/index.html\">routes</a> · <a href=\"json-hooks/index.html\">JSON</a> · <a href=\"events/index.html\">lifecycle</a></p>",
        header = html::page_header(
            "Hooks roadmap",
            "Every plugin hook type and where to read how it works.",
        ),
        version = plugin_api_version(),
        rows = rows,
    )
}

fn overview_page() -> String {
    format!(
        "{header}
{overview}
<h2>Startup sequence</h2>
{lifecycle}
<h2>Logging</h2>
<p>Plugin discovery, per-library load, route registration, and <code>HostApi::log_info</code> output use the <code>plugin</code> tracing target at <strong>DEBUG</strong>. At INFO you get one consolidated <code>FeatherFly ready</code> line with plugin/hook/route counts. Set <code>logging.level: debug</code> (or run with <code>--debug</code>) to see loader detail in <code>latest.log</code>.</p>
<h2>Remote management</h2>
<p>FeatherPanel can read/edit <code>config.yml</code>, restart the daemon, and trigger plugin reloads over the HTTP API. See <a href=\"../api/system-config.html\">remote config</a>, <a href=\"../api/system-restart.html\">remote restart</a>, and <a href=\"../api/system-plugins.html\">plugin reload</a>. Toggle capabilities with <code>remote.config_edit</code> and <code>remote.restart</code> in config.</p>
<h2>Hook systems</h2>
<div class=\"meta-grid\">
  <div class=\"meta-item\"><div class=\"meta-label\">Lifecycle</div><div class=\"meta-value\"><code>hook!</code> — <a href=\"events/index.html\">events</a></div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Config</div><div class=\"meta-value\"><code>hook_config!</code> — <a href=\"config-hooks/index.html\">config.mutate</a></div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Request</div><div class=\"meta-value\"><code>hook_request!</code> — <a href=\"request-hooks/index.html\">intercept / middleware</a></div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">Routes</div><div class=\"meta-value\"><code>route!</code> — <a href=\"routes/index.html\">route.register</a></div></div>
  <div class=\"meta-item\"><div class=\"meta-label\">JSON</div><div class=\"meta-value\"><code>hook_json!</code> — <a href=\"json-hooks/index.html\">response / actions</a></div></div>
</div>
<p>All hooks for a target run in <strong>plugin load order</strong> (alphabetical by <code>.so</code> filename). Register everything inside <code>init</code>.</p>",
        header = html::page_header("Overview", "How FeatherFly loads and runs plugins."),
        overview = html::text_block(OVERVIEW),
        lifecycle = html::text_block(LIFECYCLE),
    )
}

fn hook_guide_page(doc: &featherfly_plugin_sdk::metadata::HookGuideDoc) -> String {
    format!(
        "{header}
{meta}
<p>{details}</p>
<h2>Pipeline</h2>
<p>{pipeline}</p>
<h2>Use cases</h2>
{use_cases}
<h2>Example</h2>
{example}
<p><a href=\"index.html\">← Back to index</a></p>",
        header = html::page_header(doc.name, doc.summary),
        meta = html::meta_grid(&[
            ("When", doc.when),
            ("Input", doc.input),
            ("Output", doc.output),
            ("Route matching", doc.route_matching),
        ]),
        details = doc.details,
        pipeline = doc.pipeline,
        use_cases = html::use_cases_list(doc.use_cases),
        example = html::example_block(
            "Registration and handler",
            doc.register_example,
            doc.handler_example,
        ),
    )
}

fn config_hooks_index_page() -> String {
    hook_index(CONFIG_HOOK_DOCS, "config-hooks", "Config mutation hooks")
}

fn request_hooks_index_page() -> String {
    format!(
        "{index}
<h2>Request stack diagram</h2>
{pipeline}",
        index = hook_index(REQUEST_HOOK_DOCS, "request-hooks", "Request hooks"),
        pipeline = html::text_block(REQUEST_PIPELINE),
    )
}

fn routes_index_page() -> String {
    hook_index(ROUTE_HOOK_DOCS, "routes", "Plugin HTTP routes")
}

fn hook_index(
    docs: &[featherfly_plugin_sdk::metadata::HookGuideDoc],
    section: &str,
    title: &str,
) -> String {
    let mut cards = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    for doc in docs {
        let slug = doc.name.split('.').next_back().unwrap_or("index");
        cards.push_str(&format!(
            r#"<a href="{slug}.html" class="doc-card"><strong class="block mb-1 text-white"><code>{name}</code></strong><span class="text-sm text-[#888888]">{summary}</span></a>"#,
            slug = if section == "routes" { "register" } else { slug },
            name = doc.name,
            summary = doc.summary,
        ));
    }
    cards.push_str("</div>");

    format!(
        "{header}
{cards}",
        header = html::page_header(title, "Register during init — see each page for examples."),
        cards = cards,
    )
}

fn events_index_page() -> String {
    let mut card_html = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    let mut summary_rows = String::new();

    for doc in EVENT_DOCS {
        let slug = html::event_slug(doc.name);
        card_html.push_str(&format!(
            r#"<a href="{slug}.html" class="doc-card"><strong class="block mb-1 text-white"><code>{name}</code></strong><span class="text-sm text-[#888888]">{summary}</span></a>"#,
            slug = slug,
            name = doc.name,
            summary = doc.summary,
        ));
        summary_rows.push_str(&format!(
            "<tr><td><a href=\"{slug}.html\"><code>{name}</code></a></td><td>{summary}</td><td>{when}</td><td>{cancel}</td></tr>",
            slug = slug,
            name = doc.name,
            summary = doc.summary,
            when = doc.when,
            cancel = if doc.cancelable { "yes" } else { "no" },
        ));
    }
    card_html.push_str("</div>");

    format!(
        "{header}
<p>Register during <code>init</code> with <code>hook!(host, PluginEvent::Variant, callback)</code>.</p>
{card_grid}
<h2>Quick reference</h2>
<table><thead><tr><th>Event</th><th>Summary</th><th>When</th><th>Cancel</th></tr></thead><tbody>{rows}</tbody></table>",
        header = html::page_header(
            "Lifecycle events",
            "Automatic and lifecycle hooks — 21 events, JSON payloads for HTTP and system actions.",
        ),
        card_grid = card_html,
        rows = summary_rows,
    )
}

fn event_detail_page(doc: &featherfly_plugin_sdk::metadata::EventDoc) -> String {
    format!(
        "{header}
{meta}
<p>{details}</p>
<h2>Use cases</h2>
{use_cases}
<h2>Example</h2>
{example}
<p><a href=\"index.html\">← All lifecycle events</a></p>",
        header = html::page_header(doc.name, doc.summary),
        meta = html::meta_grid(&[
            ("When", doc.when),
            ("Payload", doc.payload),
            (
                "Cancelable",
                if doc.cancelable {
                    "Yes — HookResult::cancel() stops later handlers"
                } else {
                    "No"
                },
            ),
        ]),
        details = doc.details,
        use_cases = html::use_cases_list(doc.use_cases),
        example = html::example_block(
            "Registration and handler",
            doc.register_example,
            doc.handler_example,
        ),
    )
}

fn json_hooks_index_page() -> String {
    let mut card_html = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    let mut summary_rows = String::new();

    for doc in JSON_HOOK_DOCS {
        let (slug, _) = json_hook_paths(doc.name);
        card_html.push_str(&format!(
            r#"<a href="{slug}.html" class="doc-card"><strong class="block mb-1 text-white"><code>{name}</code></strong><span class="text-sm text-[#888888]">{summary}</span></a>"#,
            slug = slug,
            name = doc.name,
            summary = doc.summary,
        ));
        summary_rows.push_str(&format!(
            "<tr><td><a href=\"{slug}.html\"><code>{name}</code></a></td><td>{summary}</td><td>{routes}</td></tr>",
            slug = slug,
            name = doc.name,
            summary = doc.summary,
            routes = doc.route_matching,
        ));
    }
    card_html.push_str("</div>");

    format!(
        "{header}
<p>Register during <code>init</code> with <code>hook_json!(host, JsonMutateTarget::Variant, route, callback)</code>.</p>
{card_grid}
<h2>Action object shape</h2>
<pre><code>{{
  \"id\": \"check_update\",
  \"label\": \"Check for updates\",
  \"step\": \"GET /api/system/update\"
}}</code></pre>
<h2>Quick reference</h2>
<table><thead><tr><th>Target</th><th>Summary</th><th>Route matching</th></tr></thead><tbody>{rows}</tbody></table>",
        header = html::page_header(
            "JSON mutation hooks",
            "Rewrite API response bodies and action arrays per route.",
        ),
        card_grid = card_html,
        rows = summary_rows,
    )
}

fn json_hook_detail_page(doc: &featherfly_plugin_sdk::metadata::JsonHookDoc) -> String {
    format!(
        "{header}
{meta}
<p>{details}</p>
<h2>Use cases</h2>
{use_cases}
<h2>Example</h2>
{example}
<p><a href=\"index.html\">← All JSON hooks</a></p>",
        header = html::page_header(doc.name, doc.summary),
        meta = html::meta_grid(&[
            ("Input", doc.input),
            ("Route matching", doc.route_matching),
            ("Pipeline", doc.pipeline),
        ]),
        details = doc.details,
        use_cases = html::use_cases_list(doc.use_cases),
        example = html::example_block(
            "Registration and handler",
            doc.register_example,
            doc.handler_example,
        ),
    )
}

fn host_api_page() -> String {
    format!(
        "{header}
{host_api}
<h2>Output helpers</h2>
<table><thead><tr><th>Function</th><th>Hook</th></tr></thead><tbody>
<tr><td><code>write_yaml_output</code></td><td>config.mutate</td></tr>
<tr><td><code>write_request_response</code></td><td>request.intercept / middleware.inject</td></tr>
<tr><td><code>write_route_response</code></td><td>route.register</td></tr>
<tr><td><code>write_json_output</code></td><td>json.response / json.actions</td></tr>
</tbody></table>
<p>See <a href=\"macros.html\">macros</a> and <a href=\"return-codes.html\">return codes</a>.</p>
<h2>Runtime inspection</h2>
<p><code>GET /api/system/plugins</code> — lists loaded plugins, hook counts, request phases, and route count. {source}</p>",
        header = html::page_header("Host API", "What FeatherFly passes to your plugin at init."),
        host_api = html::text_block(HOST_API),
        source = html::github_source(
            "application/src/routes/api/system/plugins.rs",
            "Handler source",
        ),
    )
}

fn macros_page() -> String {
    let mut rows = String::new();
    for (name, purpose) in MACROS {
        rows.push_str(&format!(
            "<tr><td><code>{name}</code></td><td>{purpose}</td></tr>"
        ));
    }

    format!(
        "{header}
<table><thead><tr><th>Macro</th><th>Purpose</th></tr></thead><tbody>{rows}</tbody></table>
<h2>Logging</h2>
<pre><code>unsafe {{ log_info(host, \"message\"); }}</code></pre>
<p>{source}</p>",
        header = html::page_header(
            "SDK macros",
            "Register hooks during init with compile-time checks."
        ),
        rows = rows,
        source = html::github_source("featherfly-plugin-sdk/src/lib.rs", "Macro definitions"),
    )
}

fn return_codes_page() -> String {
    format!(
        "{header}
{body}",
        header = html::page_header("Return codes", "What to return from every hook callback.",),
        body = html::text_block(RETURN_CODES),
    )
}

fn source_tree_page() -> String {
    format!(
        "{header}
{body}
<p>{repo}</p>",
        header = html::page_header(
            "Source tree",
            "Where to read FeatherFly implementation code.",
        ),
        body = html::text_block(SOURCE_TREE),
        repo = html::github_source("", "Open repository on GitHub"),
    )
}

fn getting_started_page() -> String {
    format!(
        "{header}
{build}
<h2>Reference plugin</h2>
<p>After install, inspect <code>plugins/hello/src/lib.rs</code> — it registers every v4 hook type. {hello}</p>",
        header = html::page_header(
            "Getting started",
            "Create, build, and install your first plugin.",
        ),
        build = html::text_block(BUILD_AND_INSTALL),
        hello = html::github_source("plugins/hello/src/lib.rs", "View hello plugin"),
    )
}

fn example_page() -> String {
    format!(
        "{header}
<p>Complete v4 plugin with config, request, route, and JSON hooks. Production source: {hello}</p>
<pre><code>{example}</code></pre>",
        header = html::page_header("Full plugin example", "Every hook type in one plugin.",),
        hello = html::github_source("plugins/hello/src/lib.rs", "plugins/hello"),
        example = html::html_escape(FULL_PLUGIN_EXAMPLE),
    )
}
