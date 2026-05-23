use crate::html;
use featherfly_plugin_sdk::metadata::{JSON_HOOK_DOCS, EVENT_DOCS};
use std::path::Path;

pub fn generate_plugin_docs(output: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;

    html::write(
        &output.join("index.html"),
        "Plugin Hooks",
        &format!(
            "{nav}<h1>Plugin Developer Guide</h1>
<p class=\"lead\">Generated from <code>featherfly-plugin-sdk</code> metadata. Plugin API version <code>{version}</code>.</p>
<div class=\"grid\">
  <a class=\"card\" href=\"events.html\"><h2>Lifecycle events</h2><p>{event_count} daemon events with payloads and handler examples.</p></a>
  <a class=\"card\" href=\"json-hooks.html\"><h2>JSON mutation hooks</h2><p>{json_count} targets for response bodies and action steps.</p></a>
  <a class=\"card\" href=\"getting-started.html\"><h2>Getting started</h2><p>Build, install, and ship native <code>.so</code> plugins.</p></a>
</div>",
            nav = html::nav_plugins(),
            version = featherfly_plugin_sdk::metadata::plugin_api_version(),
            event_count = EVENT_DOCS.len(),
            json_count = JSON_HOOK_DOCS.len(),
        ),
    )?;

    html::write(
        &output.join("events.html"),
        "Plugin Events",
        &events_page(),
    )?;

    html::write(
        &output.join("json-hooks.html"),
        "JSON Mutation Hooks",
        &json_hooks_page(),
    )?;

    html::write(
        &output.join("getting-started.html"),
        "Plugin Getting Started",
        &getting_started_page(),
    )?;

    Ok(())
}

fn events_page() -> String {
    let mut rows = String::new();
    for doc in EVENT_DOCS {
        rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            doc.name,
            doc.summary,
            doc.when,
            doc.payload,
            if doc.cancelable { "yes" } else { "no" }
        ));
    }

    let mut examples = String::new();
    for doc in EVENT_DOCS {
        examples.push_str(&format!(
            "<h3><code>{}</code></h3><p>{summary} {when}</p><h4>Register</h4><pre><code>{register}</code></pre><h4>Handler</h4><pre><code>{handler}</code></pre>",
            doc.name,
            summary = doc.summary,
            when = doc.when,
            register = html_escape(doc.register_example),
            handler = html_escape(doc.handler_example),
        ));
    }

    format!(
        "{nav}<h1>Lifecycle Events</h1>
<p class=\"lead\">Register with <code>hook!(host, PluginEvent::Variant, callback)</code>. Hooks run in plugin load order. Return <code>HookResult::cancel()</code> to stop remaining handlers for that event.</p>
<table>
<tr><th>Event</th><th>Summary</th><th>When</th><th>Payload</th><th>Cancelable</th></tr>
{rows}
</table>
<h2>Examples</h2>
{examples}",
        nav = html::nav_plugins(),
        rows = rows,
        examples = examples,
    )
}

fn json_hooks_page() -> String {
    let mut rows = String::new();
    for doc in JSON_HOOK_DOCS {
        rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            doc.name, doc.summary, doc.input, doc.route_matching
        ));
    }

    let mut examples = String::new();
    for doc in JSON_HOOK_DOCS {
        examples.push_str(&format!(
            "<h3><code>{}</code></h3><p>{summary}</p><h4>Register</h4><pre><code>{register}</code></pre><h4>Handler</h4><pre><code>{handler}</code></pre>",
            doc.name,
            summary = doc.summary,
            register = html_escape(doc.register_example),
            handler = html_escape(doc.handler_example),
        ));
    }

    format!(
        "{nav}<h1>JSON Mutation Hooks</h1>
<p class=\"lead\">Register with <code>hook_json!(host, JsonMutateTarget::Variant, route_pattern, callback)</code>. Return <code>JSON_MUTATE_MODIFIED</code> after writing output with <code>write_json_output</code>, or <code>JSON_MUTATE_UNCHANGED</code> to pass through.</p>
<h2>Action object shape</h2>
<pre><code>{{
  \"id\": \"check_update\",
  \"label\": \"Check for updates\",
  \"step\": \"GET /api/system/update\"
}}</code></pre>
<table>
<tr><th>Target</th><th>Summary</th><th>Input JSON</th><th>Route matching</th></tr>
{rows}
</table>
<h2>Examples</h2>
{examples}",
        nav = html::nav_plugins(),
        rows = rows,
        examples = examples,
    )
}

fn getting_started_page() -> String {
    format!(
        "{nav}<h1>Getting Started</h1>
<p class=\"lead\">Plugins are native shared libraries loaded from the configured plugins directory.</p>
<h2>Project setup</h2>
<pre><code>cargo new --lib my-plugin
# Cargo.toml
[lib]
crate-type = [\"cdylib\"]

[dependencies]
featherfly-plugin-sdk = {{ path = \"../featherfly-plugin-sdk\" }}
serde_json = \"1\"</code></pre>
<h2>Build and install</h2>
<pre><code>featherfly plugin build ./my-plugin
featherfly plugin install ./my-plugin
featherfly plugin ship ./my-plugin</code></pre>
<h2>Paths</h2>
<ul>
<li>Production plugins: <code>/var/lib/featherfly/plugins/*.so</code></li>
<li>Debug plugins: <code>./data/plugins/*.so</code></li>
<li>Config keys: <code>system.plugins_directory</code>, <code>plugins.enabled</code></li>
</ul>
<h2>Inspect at runtime</h2>
<pre><code>curl -H \"Authorization: Bearer $TOKEN\" http://127.0.0.1:8080/api/system/plugins</code></pre>
<p>Returns loaded plugins, hook counts, and the current event / JSON target names from the SDK.</p>
<h2>SDK macros</h2>
<table>
<tr><th>Macro</th><th>Purpose</th></tr>
<tr><td><code>declare_plugin!</code></td><td>Export <code>featherfly_plugin_entry</code> with name, version, init, shutdown.</td></tr>
<tr><td><code>hook!</code></td><td>Register a lifecycle event callback during <code>init</code>.</td></tr>
<tr><td><code>hook_json!</code></td><td>Register a JSON mutation callback during <code>init</code>.</td></tr>
</table>
<p>Example source: repository path <code>plugins/hello/</code>.</p>",
        nav = html::nav_plugins(),
    )
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
