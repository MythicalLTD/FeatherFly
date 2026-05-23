use featherfly_plugin_sdk::metadata::{EVENT_DOCS, JSON_HOOK_DOCS};
use std::path::Path;

#[derive(serde::Serialize)]
struct SearchEntry {
    title: String,
    url: String,
    text: String,
}

pub fn write_index(output: &Path) -> std::io::Result<()> {
    let mut entries = vec![
        entry(
            "Home",
            "index.html",
            "FeatherFly documentation hub plugins API",
        ),
        entry(
            "Plugin guide",
            "plugins/index.html",
            "plugin documentation native so hooks",
        ),
        entry(
            "Getting started",
            "plugins/getting-started.html",
            "build install ship cdylib cargo toml paths",
        ),
        entry(
            "Terminology",
            "plugins/terminology.html",
            "hooks mixin pipeline load order vocabulary",
        ),
        entry(
            "Architecture",
            "plugins/architecture.html",
            "mixin pipeline composition json hooks lifecycle",
        ),
        entry(
            "Hooks roadmap",
            "plugins/hooks-roadmap.html",
            "planned hooks config mutate request intercept route register",
        ),
        entry(
            "Overview",
            "plugins/overview.html",
            "lifecycle hooks load order init shutdown",
        ),
        entry(
            "Lifecycle events",
            "plugins/events/index.html",
            "hook PluginEvent startup shutdown config",
        ),
        entry(
            "JSON hooks",
            "plugins/json-hooks/index.html",
            "json mutation response body actions hook_json",
        ),
        entry(
            "Host API",
            "plugins/host-api.html",
            "HostApi register_hook register_json_hook log_info macros",
        ),
        entry(
            "Full example",
            "plugins/example.html",
            "complete plugin source declare_plugin hook",
        ),
        entry(
            "HTTP API",
            "api/index.html",
            "rest routes authentication bearer token",
        ),
        entry(
            "Health",
            "api/health.html",
            "liveness probe unauthenticated uptime",
        ),
        entry(
            "System API",
            "api/system.html",
            "host architecture cpu kernel version actions",
        ),
        entry(
            "System overview",
            "api/system-overview.html",
            "memory cpu metrics dashboard",
        ),
        entry(
            "System plugins",
            "api/system-plugins.html",
            "loaded plugins hooks json targets",
        ),
        entry(
            "System update",
            "api/system-update.html",
            "github release check upgrade channel",
        ),
        entry(
            "System upgrade",
            "api/system-upgrade.html",
            "download binary sha256 restart",
        ),
        entry(
            "Unit tests",
            "tests/index.html",
            "cargo test ci inventory passed failed",
        ),
    ];

    for doc in EVENT_DOCS {
        let slug = doc.name.replace('.', "-");
        entries.push(entry(
            doc.name,
            &format!("plugins/events/{slug}.html"),
            &format!(
                "{} {} {} lifecycle event hook",
                doc.summary, doc.when, doc.payload
            ),
        ));
    }

    for doc in JSON_HOOK_DOCS {
        let slug = match doc.name {
            "json.response" => "response-body",
            "json.actions" => "response-actions",
            _ => "index",
        };
        entries.push(entry(
            doc.name,
            &format!("plugins/json-hooks/{slug}.html"),
            &format!(
                "{} {} {} json hook mutation",
                doc.summary, doc.input, doc.route_matching
            ),
        ));
    }

    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(output.join("search-index.json"), json)
}

fn entry(title: &str, url: &str, text: &str) -> SearchEntry {
    SearchEntry {
        title: title.into(),
        url: url.into(),
        text: text.into(),
    }
}
