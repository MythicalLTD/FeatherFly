use featherfly_plugin_sdk::metadata::{
    CONFIG_HOOK_DOCS, EVENT_DOCS, JSON_HOOK_DOCS, REQUEST_HOOK_DOCS, ROUTE_HOOK_DOCS,
};
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
            "FeatherFly documentation hub plugins API web hosting daemon",
        ),
        entry(
            "Plugin guide",
            "plugins/index.html",
            "plugin documentation native so hooks config request routes json",
        ),
        entry(
            "Getting started",
            "plugins/getting-started.html",
            "build install ship cdylib cargo toml paths hello plugin",
        ),
        entry(
            "Terminology",
            "plugins/terminology.html",
            "hooks mixin pipeline load order vocabulary config request route",
        ),
        entry(
            "Architecture",
            "plugins/architecture.html",
            "mixin pipeline composition json hooks lifecycle request intercept middleware",
        ),
        entry(
            "Hooks roadmap",
            "plugins/hooks-roadmap.html",
            "hooks config mutate request intercept route register middleware inject json",
        ),
        entry(
            "Overview",
            "plugins/overview.html",
            "lifecycle hooks load order init shutdown config mutate",
        ),
        entry(
            "Config hooks",
            "plugins/config-hooks/index.html",
            "config mutate yaml rewrite defaults overrides",
        ),
        entry(
            "config.mutate",
            "plugins/config-hooks/mutate.html",
            "rewrite config yaml before apply hook_config write_yaml_output",
        ),
        entry(
            "Request hooks",
            "plugins/request-hooks/index.html",
            "request intercept middleware inject auth rate limit headers",
        ),
        entry(
            "request.intercept",
            "plugins/request-hooks/intercept.html",
            "inbound HTTP auth short circuit write_request_response",
        ),
        entry(
            "middleware.inject",
            "plugins/request-hooks/middleware.html",
            "inner request middleware tracing headers validation",
        ),
        entry(
            "Plugin routes",
            "plugins/routes/index.html",
            "route register HTTP endpoint plugin API",
        ),
        entry(
            "route.register",
            "plugins/routes/register.html",
            "register route GET POST write_route_response plugin handler",
        ),
        entry(
            "Lifecycle events",
            "plugins/events/index.html",
            "hook PluginEvent startup shutdown config loaded",
        ),
        entry(
            "JSON hooks",
            "plugins/json-hooks/index.html",
            "json mutation response body actions hook_json",
        ),
        entry(
            "Host API",
            "plugins/host-api.html",
            "HostApi register_hook register_config_hook register_route log_info",
        ),
        entry(
            "Macros",
            "plugins/macros.html",
            "declare_plugin hook hook_config hook_request route hook_json",
        ),
        entry(
            "Return codes",
            "plugins/return-codes.html",
            "CONFIG_MUTATE REQUEST_CONTINUE ROUTE_OK JSON_MUTATE init status",
        ),
        entry(
            "Source tree",
            "plugins/source-tree.html",
            "repository paths daemon events middleware routes hello plugin sdk",
        ),
        entry(
            "Full example",
            "plugins/example.html",
            "complete plugin source declare_plugin v4 all hooks",
        ),
        entry(
            "HTTP API",
            "api/index.html",
            "rest routes authentication bearer token featherpanel",
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
            "loaded plugins hooks json targets request phases routes",
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

    for doc in CONFIG_HOOK_DOCS {
        entries.push(entry(
            doc.name,
            "plugins/config-hooks/mutate.html",
            &format!("{} {} config yaml mutation", doc.summary, doc.when),
        ));
    }

    for doc in REQUEST_HOOK_DOCS {
        let slug = match doc.name {
            "request.intercept" => "intercept",
            "middleware.inject" => "middleware",
            _ => "index",
        };
        entries.push(entry(
            doc.name,
            &format!("plugins/request-hooks/{slug}.html"),
            &format!("{} {} request hook HTTP", doc.summary, doc.pipeline),
        ));
    }

    for doc in ROUTE_HOOK_DOCS {
        entries.push(entry(
            doc.name,
            "plugins/routes/register.html",
            &format!("{} {} plugin HTTP route", doc.summary, doc.pipeline),
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
