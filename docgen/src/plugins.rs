use crate::html::{self, Section};
use featherfly_plugin_sdk::metadata::{
    BUILD_AND_INSTALL, CAPABILITIES, EVENT_DOCS, FULL_PLUGIN_EXAMPLE, HOST_API, JSON_HOOK_DOCS,
    LIFECYCLE, MACROS, OVERVIEW, plugin_api_version,
};
use std::path::Path;

pub fn generate_plugin_docs(output: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;

    html::write(
        &output.join("index.html"),
        "Plugins",
        Section::Plugins,
        &index_page(),
    )?;
    html::write(
        &output.join("overview.html"),
        "Overview",
        Section::Plugins,
        &overview_page(),
    )?;
    html::write(
        &output.join("events.html"),
        "Events",
        Section::Plugins,
        &events_page(),
    )?;
    html::write(
        &output.join("json-hooks.html"),
        "JSON hooks",
        Section::Plugins,
        &json_hooks_page(),
    )?;
    html::write(
        &output.join("host-api.html"),
        "Host API",
        Section::Plugins,
        &host_api_page(),
    )?;
    html::write(
        &output.join("getting-started.html"),
        "Getting started",
        Section::Plugins,
        &getting_started_page(),
    )?;
    html::write(
        &output.join("example.html"),
        "Full example",
        Section::Plugins,
        &example_page(),
    )?;

    Ok(())
}

fn index_page() -> String {
    format!(
        "{title}
<p>Complete reference for FeatherFly native plugins. Plugin API version <code>{version}</code>.</p>
<h2>What plugins can do</h2>
{capabilities}
<h2>Documentation</h2>
{links}",
        title = html::page_title(
            "Plugin documentation",
            "Everything you need to build, install, and extend FeatherFly with native .so plugins.",
        ),
        version = plugin_api_version(),
        capabilities = capabilities_list(),
        links = html::link_list(&[
            (
                "overview.html",
                "Overview",
                "What plugins are, lifecycle, hook systems, load order.",
            ),
            (
                "getting-started.html",
                "Getting started",
                "Project setup, build, install paths, version matching.",
            ),
            (
                "events.html",
                "Lifecycle events",
                "Every event, payload, cancel behavior, and examples.",
            ),
            (
                "json-hooks.html",
                "JSON mutation hooks",
                "Modify API responses and action steps.",
            ),
            (
                "host-api.html",
                "Host API reference",
                "HostApi fields, return codes, macros, HTTP method codes.",
            ),
            (
                "example.html",
                "Full example",
                "Complete plugin with events and JSON hooks.",
            ),
        ]),
    )
}

fn overview_page() -> String {
    format!(
        "{title}
{overview}
<h2>Capabilities</h2>
{capabilities}
<h2>Daemon lifecycle (hook order)</h2>
{lifecycle}
<h2>Hook systems</h2>
<p><strong>Lifecycle events</strong> — fixed points in daemon startup/shutdown. Callback signature: <code>extern \"C\" fn(*const EventContext) -&gt; HookResult</code>.</p>
<p><strong>JSON mutation</strong> — runs on every JSON HTTP response matching your route prefix. Callback signature: <code>extern \"C\" fn(*const JsonMutateContext) -&gt; i32</code>.</p>
<p>Register all hooks inside <code>init</code>. Hooks run in plugin load order (sorted by filename in the plugins directory).</p>",
        title = html::page_title("Overview", "How FeatherFly plugins work."),
        overview = html::text_block(OVERVIEW),
        capabilities = capabilities_list(),
        lifecycle = html::text_block(LIFECYCLE),
    )
}

fn events_page() -> String {
    let mut summary_rows = String::new();
    let mut full = String::new();

    for doc in EVENT_DOCS {
        summary_rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            doc.name,
            doc.summary,
            doc.when,
            doc.payload,
            if doc.cancelable { "yes" } else { "no" }
        ));

        full.push_str(&format!(
            "<h2><code>{}</code></h2>
<p>{summary}</p>
<p><strong>When:</strong> {when}</p>
<p><strong>Payload:</strong> {payload}</p>
<p><strong>Cancelable:</strong> {cancelable}</p>
<p>{details}</p>
<h3>Use cases</h3>
{use_cases}
{example}",
            doc.name,
            summary = doc.summary,
            when = doc.when,
            payload = doc.payload,
            cancelable = if doc.cancelable {
                "yes — HookResult::cancel() stops later handlers for this event"
            } else {
                "no"
            },
            details = doc.details,
            use_cases = html::use_cases_list(doc.use_cases),
            example = html::example_block(
                doc.name,
                "Registration and handler template",
                doc.register_example,
                doc.handler_example,
            ),
        ));
    }

    format!(
        "{title}
<p>Register with <code>hook!(host, PluginEvent::Variant, callback)</code> during <code>init</code>.</p>
<h2>Quick reference</h2>
<table><thead><tr><th>Event</th><th>Summary</th><th>When</th><th>Payload</th><th>Cancel</th></tr></thead><tbody>{summary_rows}</tbody></table>
<hr>
<h2>Full reference</h2>
{full}",
        title = html::page_title(
            "Lifecycle events",
            "All events plugins can subscribe to, with payloads and examples.",
        ),
        summary_rows = summary_rows,
        full = full,
    )
}

fn json_hooks_page() -> String {
    let mut summary_rows = String::new();
    let mut full = String::new();

    for doc in JSON_HOOK_DOCS {
        summary_rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            doc.name, doc.summary, doc.input, doc.route_matching
        ));

        full.push_str(&format!(
            "<h2><code>{}</code></h2>
<p>{summary}</p>
<p><strong>Input:</strong> {input}</p>
<p><strong>Route matching:</strong> {routes}</p>
<p><strong>Pipeline:</strong> {pipeline}</p>
<p>{details}</p>
<h3>Use cases</h3>
{use_cases}
{example}",
            doc.name,
            summary = doc.summary,
            input = doc.input,
            routes = doc.route_matching,
            pipeline = doc.pipeline,
            details = doc.details,
            use_cases = html::use_cases_list(doc.use_cases),
            example = html::example_block(
                doc.name,
                "Registration and handler template",
                doc.register_example,
                doc.handler_example,
            ),
        ));
    }

    format!(
        "{title}
<p>Register with <code>hook_json!(host, JsonMutateTarget::Variant, route_pattern, callback)</code> during <code>init</code>.</p>
<h2>Action object shape</h2>
<pre><code>{{
  \"id\": \"check_update\",
  \"label\": \"Check for updates\",
  \"step\": \"GET /api/system/update\"
}}</code></pre>
<h2>Quick reference</h2>
<table><thead><tr><th>Target</th><th>Summary</th><th>Input</th><th>Routes</th></tr></thead><tbody>{summary_rows}</tbody></table>
<hr>
<h2>Full reference</h2>
{full}",
        title = html::page_title(
            "JSON mutation hooks",
            "Modify API response bodies and action arrays before clients receive them.",
        ),
        summary_rows = summary_rows,
        full = full,
    )
}

fn host_api_page() -> String {
    let mut macro_rows = String::new();
    for (name, purpose) in MACROS {
        macro_rows.push_str(&format!("<tr><td><code>{name}</code></td><td>{purpose}</td></tr>"));
    }

    format!(
        "{title}
{host_api}
<h2>SDK macros</h2>
<table><thead><tr><th>Macro</th><th>Purpose</th></tr></thead><tbody>{macro_rows}</tbody></table>
<h2>Logging</h2>
<pre><code>unsafe {{ log_info(host, \"message\"); }}</code></pre>
<p>Writes to the daemon log with target <code>plugin</code>.</p>
<h2>JSON output helper</h2>
<pre><code>write_json_output(ctx, &amp;json_bytes) // returns JSON_MUTATE_MODIFIED
// or return JSON_MUTATE_UNCHANGED to keep prior JSON</code></pre>
<h2>Inspect loaded plugins at runtime</h2>
<pre><code>GET /api/system/plugins</code></pre>
<p>Returns plugin names, versions, hook counts, event names, and JSON target names from the SDK.</p>",
        title = html::page_title("Host API", "Functions and structs passed to your plugin at init."),
        host_api = html::text_block(HOST_API),
        macro_rows = macro_rows,
    )
}

fn getting_started_page() -> String {
    format!(
        "{title}
{build}",
        title = html::page_title(
            "Getting started",
            "Create, build, and install your first plugin.",
        ),
        build = html::text_block(BUILD_AND_INSTALL),
    )
}

fn example_page() -> String {
    format!(
        "{title}
<p>Working plugin that logs on startup, adds a JSON field, and injects an action step. See also <code>plugins/hello/</code> in the repository.</p>
<pre><code>{example}</code></pre>",
        title = html::page_title(
            "Full plugin example",
            "Events + JSON body + JSON actions in one plugin.",
        ),
        example = html::html_escape(FULL_PLUGIN_EXAMPLE),
    )
}

fn capabilities_list() -> String {
    let mut out = String::from("<ul>");
    for cap in CAPABILITIES {
        out.push_str(&format!("<li>{cap}</li>"));
    }
    out.push_str("</ul>");
    out
}
