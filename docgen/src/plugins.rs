use crate::html::{self, PageContext};
use featherfly_plugin_sdk::metadata::{
    BUILD_AND_INSTALL, EVENT_DOCS, FULL_PLUGIN_EXAMPLE, HOST_API, JSON_HOOK_DOCS, LIFECYCLE,
    MACROS, OVERVIEW, plugin_api_version,
};
use std::path::Path;

pub fn generate_plugin_docs(output: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;
    std::fs::create_dir_all(output.join("events"))?;
    std::fs::create_dir_all(output.join("json-hooks"))?;

    html::write(
        &output.join("index.html"),
        "Plugins",
        PageContext::plugins("plugins"),
        &index_page(),
    )?;
    html::write(
        &output.join("getting-started.html"),
        "Getting started",
        PageContext::plugins("getting-started"),
        &getting_started_page(),
    )?;
    html::write(
        &output.join("overview.html"),
        "Overview",
        PageContext::plugins("overview"),
        &overview_page(),
    )?;
    html::write(
        &output.join("events/index.html"),
        "Lifecycle events",
        PageContext::plugins("events"),
        &events_index_page(),
    )?;
    for doc in EVENT_DOCS {
        let slug = html::event_slug(doc.name);
        html::write(
            &output.join(format!("events/{slug}.html")),
            doc.name,
            PageContext::plugins_nested(event_active_id(doc.name)),
            &event_detail_page(doc),
        )?;
    }
    html::write(
        &output.join("json-hooks/index.html"),
        "JSON hooks",
        PageContext::plugins("json-hooks"),
        &json_hooks_index_page(),
    )?;
    for doc in JSON_HOOK_DOCS {
        let (slug, active) = json_hook_paths(doc.name);
        html::write(
            &output.join(format!("json-hooks/{slug}.html")),
            doc.name,
            PageContext::plugins_nested(active),
            &json_hook_detail_page(doc),
        )?;
    }
    html::write(
        &output.join("host-api.html"),
        "Host API",
        PageContext::plugins("host-api"),
        &host_api_page(),
    )?;
    html::write(
        &output.join("example.html"),
        "Full example",
        PageContext::plugins("example"),
        &example_page(),
    )?;

    // Remove legacy flat pages if present
    let _ = std::fs::remove_file(output.join("events.html"));
    let _ = std::fs::remove_file(output.join("json-hooks.html"));

    Ok(())
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
<p>Plugin API version <code>{version}</code>.</p>
{cards}
<h2>Quick links</h2>
{links}",
        header = html::page_header(
            "Plugin documentation",
            "Build native .so plugins that extend FeatherFly at runtime.",
        ),
        version = plugin_api_version(),
        cards = html::card_grid(&[
            (
                "getting-started.html",
                "Getting started",
                "Project setup, build commands, install paths.",
            ),
            (
                "overview.html",
                "Overview",
                "How plugins load, lifecycle, hook systems.",
            ),
            (
                "events/index.html",
                "Lifecycle events",
                "Startup, shutdown, and config hooks.",
            ),
            (
                "json-hooks/index.html",
                "JSON hooks",
                "Modify API responses before clients see them.",
            ),
            (
                "host-api.html",
                "Host API",
                "HostApi fields, macros, return codes.",
            ),
            (
                "example.html",
                "Full example",
                "Complete plugin with events and JSON hooks.",
            ),
        ]),
        links = html::card_grid(&[(
            "../api/index.html",
            "HTTP API (Swagger)",
            "Interactive explorer for daemon routes.",
        )]),
    )
}

fn overview_page() -> String {
    format!(
        "{header}
{overview}
<h2>Startup sequence</h2>
{lifecycle}
<h2>Hook systems</h2>
<div class=\"grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6\">
  <div class=\"meta-item\">
    <div class=\"meta-label\">Lifecycle events</div>
    <div class=\"meta-value\">Fixed points in startup/shutdown. Register with <code>hook!</code>. See <a href=\"events/index.html\">events</a>.</div>
  </div>
  <div class=\"meta-item\">
    <div class=\"meta-label\">JSON mutation</div>
    <div class=\"meta-value\">Rewrite response JSON per route prefix. Register with <code>hook_json!</code>. See <a href=\"json-hooks/index.html\">JSON hooks</a>.</div>
  </div>
</div>
<p>All hooks for a target run in <strong>plugin load order</strong> (alphabetical by <code>.so</code> filename). Register everything inside <code>init</code>.</p>",
        header = html::page_header("Overview", "How FeatherFly loads and runs plugins."),
        overview = html::text_block(OVERVIEW),
        lifecycle = html::text_block(LIFECYCLE),
    )
}

fn events_index_page() -> String {
    let mut card_html = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    let mut summary_rows = String::new();

    for doc in EVENT_DOCS {
        let slug = html::event_slug(doc.name);
        card_html.push_str(&format!(
            r#"<a href="{slug}.html" class="doc-card"><strong class="block mb-1 text-zinc-900 dark:text-zinc-100"><code>{name}</code></strong><span class="text-sm text-zinc-600 dark:text-zinc-400">{summary}</span></a>"#,
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
            "Callbacks at fixed points in daemon startup and shutdown.",
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
            r#"<a href="{slug}.html" class="doc-card"><strong class="block mb-1 text-zinc-900 dark:text-zinc-100"><code>{name}</code></strong><span class="text-sm text-zinc-600 dark:text-zinc-400">{summary}</span></a>"#,
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
    let mut macro_rows = String::new();
    for (name, purpose) in MACROS {
        macro_rows.push_str(&format!(
            "<tr><td><code>{name}</code></td><td>{purpose}</td></tr>"
        ));
    }

    format!(
        "{header}
{host_api}
<h2>SDK macros</h2>
<table><thead><tr><th>Macro</th><th>Purpose</th></tr></thead><tbody>{macro_rows}</tbody></table>
<h2>Logging</h2>
<pre><code>unsafe {{ log_info(host, \"message\"); }}</code></pre>
<h2>JSON output</h2>
<pre><code>write_json_output(ctx, &amp;json_bytes)  // JSON_MUTATE_MODIFIED
return JSON_MUTATE_UNCHANGED           // keep prior JSON</code></pre>
<h2>Runtime inspection</h2>
<p><code>GET /api/system/plugins</code> — lists loaded plugins, hook counts, and registered targets.</p>",
        header = html::page_header("Host API", "What FeatherFly passes to your plugin at init."),
        host_api = html::text_block(HOST_API),
        macro_rows = macro_rows,
    )
}

fn getting_started_page() -> String {
    format!(
        "{header}
{build}",
        header = html::page_header(
            "Getting started",
            "Create, build, and install your first plugin.",
        ),
        build = html::text_block(BUILD_AND_INSTALL),
    )
}

fn example_page() -> String {
    format!(
        "{header}
<p>Working plugin: logs on startup, adds a JSON field, injects an action step. Source also lives in <code>plugins/hello/</code>.</p>
<pre><code>{example}</code></pre>",
        header = html::page_header(
            "Full plugin example",
            "Events + JSON body + JSON actions in one plugin.",
        ),
        example = html::html_escape(FULL_PLUGIN_EXAMPLE),
    )
}
