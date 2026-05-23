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
        SearchEntry {
            title: "Home".into(),
            url: "index.html".into(),
            text: "FeatherFly documentation hub plugins API".into(),
        },
        SearchEntry {
            title: "Plugin guide".into(),
            url: "plugins/index.html".into(),
            text: "plugin documentation native so hooks".into(),
        },
        SearchEntry {
            title: "Getting started".into(),
            url: "plugins/getting-started.html".into(),
            text: "build install ship cdylib cargo toml paths".into(),
        },
        SearchEntry {
            title: "Overview".into(),
            url: "plugins/overview.html".into(),
            text: "lifecycle hooks load order init shutdown".into(),
        },
        SearchEntry {
            title: "Lifecycle events".into(),
            url: "plugins/events/index.html".into(),
            text: "hook PluginEvent startup shutdown config".into(),
        },
        SearchEntry {
            title: "JSON hooks".into(),
            url: "plugins/json-hooks/index.html".into(),
            text: "json mutation response body actions hook_json".into(),
        },
        SearchEntry {
            title: "Host API".into(),
            url: "plugins/host-api.html".into(),
            text: "HostApi register_hook register_json_hook log_info macros".into(),
        },
        SearchEntry {
            title: "Full example".into(),
            url: "plugins/example.html".into(),
            text: "complete plugin source declare_plugin hook".into(),
        },
        SearchEntry {
            title: "Swagger UI".into(),
            url: "api/index.html".into(),
            text: "openapi interactive http api explorer".into(),
        },
        SearchEntry {
            title: "Endpoints".into(),
            url: "api/endpoints.html".into(),
            text: "curl routes GET POST auth bearer".into(),
        },
    ];

    for doc in EVENT_DOCS {
        let slug = doc.name.replace('.', "-");
        entries.push(SearchEntry {
            title: doc.name.to_string(),
            url: format!("plugins/events/{slug}.html"),
            text: format!(
                "{} {} {} lifecycle event hook",
                doc.summary, doc.when, doc.payload
            ),
        });
    }

    for doc in JSON_HOOK_DOCS {
        let slug = match doc.name {
            "json.response" => "response-body",
            "json.actions" => "response-actions",
            _ => "index",
        };
        entries.push(SearchEntry {
            title: doc.name.to_string(),
            url: format!("plugins/json-hooks/{slug}.html"),
            text: format!(
                "{} {} {} json hook mutation",
                doc.summary, doc.input, doc.route_matching
            ),
        });
    }

    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(output.join("search-index.json"), json)
}
