use featherfly_plugin_sdk::metadata::{
    CONFIG_HOOK_DOCS, EVENT_DOCS, JSON_HOOK_DOCS, MACROS, REQUEST_HOOK_DOCS, ROUTE_HOOK_DOCS,
    plugin_api_version,
};
use serde::Serialize;
use utoipa::ToSchema;

/// OpenAPI-style catalog of every plugin hook FeatherFly exposes.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginHookSchema {
    pub api_version: u32,
    pub reference: PluginReferenceLinks,
    pub lifecycle_events: Vec<LifecycleEventSchema>,
    pub config_hooks: Vec<HookGuideSchema>,
    pub request_hooks: Vec<HookGuideSchema>,
    pub json_hooks: Vec<JsonHookSchema>,
    pub route_hooks: Vec<HookGuideSchema>,
    pub macros: Vec<MacroSchema>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginReferenceLinks {
    /// Plain-text hook catalog (`docs/plugins-reference.txt` after `make docs`).
    pub txt: &'static str,
    /// HTML plugin docs index.
    pub html: &'static str,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LifecycleEventSchema {
    pub id: u32,
    /// Wire name, e.g. `site.provisioned`.
    pub name: String,
    /// Rust enum variant for `hook!(host, PluginEvent::..., handler)`.
    pub variant: String,
    pub summary: String,
    pub when: String,
    pub payload: String,
    pub cancelable: bool,
    pub details: String,
    pub use_cases: Vec<String>,
    pub register_example: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HookGuideSchema {
    pub name: String,
    pub summary: String,
    pub when: String,
    pub input: String,
    pub output: String,
    pub pipeline: String,
    pub route_matching: String,
    pub register_example: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonHookSchema {
    pub name: String,
    pub target_id: u32,
    pub summary: String,
    pub input: String,
    pub pipeline: String,
    pub route_matching: String,
    pub register_example: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MacroSchema {
    pub name: String,
    pub description: String,
}

#[must_use]
pub fn build_hook_schema() -> PluginHookSchema {
    PluginHookSchema {
        api_version: plugin_api_version(),
        reference: PluginReferenceLinks {
            txt: "docs/plugins-reference.txt",
            html: "docs/plugins/index.html",
        },
        lifecycle_events: EVENT_DOCS
            .iter()
            .map(|doc| LifecycleEventSchema {
                id: doc.event.as_u32(),
                name: doc.name.to_string(),
                variant: format!("{:?}", doc.event),
                summary: doc.summary.to_string(),
                when: doc.when.to_string(),
                payload: doc.payload.to_string(),
                cancelable: doc.cancelable,
                details: doc.details.to_string(),
                use_cases: doc.use_cases.iter().map(|s| (*s).to_string()).collect(),
                register_example: doc.register_example.to_string(),
            })
            .collect(),
        config_hooks: hook_guides(CONFIG_HOOK_DOCS),
        request_hooks: hook_guides(REQUEST_HOOK_DOCS),
        json_hooks: JSON_HOOK_DOCS
            .iter()
            .map(|doc| JsonHookSchema {
                name: doc.name.to_string(),
                target_id: doc.target.as_u32(),
                summary: doc.summary.to_string(),
                input: doc.input.to_string(),
                pipeline: doc.pipeline.to_string(),
                route_matching: doc.route_matching.to_string(),
                register_example: doc.register_example.to_string(),
            })
            .collect(),
        route_hooks: hook_guides(ROUTE_HOOK_DOCS),
        macros: MACROS
            .iter()
            .map(|(name, desc)| MacroSchema {
                name: (*name).to_string(),
                description: (*desc).to_string(),
            })
            .collect(),
    }
}

fn hook_guides(docs: &[featherfly_plugin_sdk::metadata::HookGuideDoc]) -> Vec<HookGuideSchema> {
    docs.iter()
        .map(|doc| HookGuideSchema {
            name: doc.name.to_string(),
            summary: doc.summary.to_string(),
            when: doc.when.to_string(),
            input: doc.input.to_string(),
            output: doc.output.to_string(),
            pipeline: doc.pipeline.to_string(),
            route_matching: doc.route_matching.to_string(),
            register_example: doc.register_example.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use featherfly_plugin_sdk::PluginEvent;

    #[test]
    fn schema_lists_all_lifecycle_events() {
        let schema = build_hook_schema();
        assert_eq!(schema.lifecycle_events.len(), PluginEvent::all().len());

        let mut ids: Vec<u32> = schema.lifecycle_events.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            (1..=PluginEvent::all().len() as u32).collect::<Vec<_>>()
        );
    }

    #[test]
    fn schema_includes_all_hook_types() {
        let schema = build_hook_schema();
        assert!(!schema.config_hooks.is_empty());
        assert!(!schema.request_hooks.is_empty());
        assert_eq!(schema.json_hooks.len(), 2);
        assert!(!schema.route_hooks.is_empty());
        assert!(!schema.macros.is_empty());
    }

    #[test]
    fn schema_serializes_to_json() {
        let schema = build_hook_schema();
        let json = serde_json::to_string(&schema).expect("schema must serialize");
        assert!(json.contains("site.provisioned"));
        assert!(json.contains("config.mutate"));
        assert!(json.contains("json.response"));
    }

    #[test]
    fn every_event_has_register_example() {
        for event in build_hook_schema().lifecycle_events {
            assert!(
                event.register_example.contains("hook!"),
                "missing hook! example for {}",
                event.name
            );
        }
    }
}
