//! Plugin hook integration tests — schema API matches SDK catalog.

use featherfly::plugins::{build_hook_schema, events::EventBus};
use featherfly_plugin_sdk::{EventContext, HookResult, PluginEvent};

extern "C" fn noop_hook(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

#[test]
fn hook_schema_matches_sdk_event_count() {
    let schema = build_hook_schema();
    assert_eq!(schema.lifecycle_events.len(), PluginEvent::all().len());
    assert_eq!(
        schema.api_version,
        featherfly_plugin_sdk::FEATHERFLY_PLUGIN_API_VERSION
    );
}

#[test]
fn hook_schema_covers_config_request_json_and_route_hooks() {
    let schema = build_hook_schema();
    assert!(
        schema
            .config_hooks
            .iter()
            .any(|h| h.name == "config.mutate")
    );
    assert!(
        schema
            .request_hooks
            .iter()
            .any(|h| h.name == "request.intercept")
    );
    assert!(schema.json_hooks.iter().any(|h| h.name == "json.response"));
    assert!(
        schema
            .route_hooks
            .iter()
            .any(|h| h.name == "route.register")
    );
}

#[test]
fn plugins_can_hook_every_lifecycle_event() {
    let mut bus = EventBus::new();
    for doc in PluginEvent::all() {
        bus.register("integration-test", doc.event.as_u32(), noop_hook);
    }

    for doc in PluginEvent::all() {
        let outcome = bus.emit(doc.event, b"{}");
        assert!(outcome.handlers_run >= 1, "no handler ran for {}", doc.name);
    }
}

#[test]
fn schema_json_is_panel_friendly() {
    let schema = build_hook_schema();
    let value = serde_json::to_value(&schema).expect("serialize schema");
    let events = value
        .get("lifecycle_events")
        .and_then(|v| v.as_array())
        .expect("lifecycle_events array");
    let site = events
        .iter()
        .find(|e| e.get("name").and_then(|n| n.as_str()) == Some("site.provisioned"))
        .expect("site.provisioned in schema");
    assert!(site.get("payload").is_some());
    assert!(site.get("register_example").is_some());
    assert_eq!(
        site.get("variant").and_then(|v| v.as_str()),
        Some("SiteProvisioned")
    );
}
