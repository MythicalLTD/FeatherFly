//! Plugin event wiring — every lifecycle event has documentation and is hookable.

use featherfly::plugins::{build_hook_schema, events::EventBus};
use featherfly_plugin_sdk::{EventContext, HookResult, PluginEvent};

extern "C" fn noop_hook(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

#[test]
fn lifecycle_catalog_has_83_unique_events() {
    assert_eq!(PluginEvent::all().len(), 83);
    let names = PluginEvent::all_names();
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 83);
}

#[test]
fn new_infrastructure_events_are_documented() {
    let schema = build_hook_schema();
    for name in [
        "config.upgraded",
        "hosting.reconciled",
        "hosting.shared_stack_synced",
        "hosting.shared_database_servers_ready",
        "hosting.phpmyadmin_provisioned",
        "hosting.error_pages_ready",
        "proxy.traefik_provisioned",
        "hosting.images_pulled",
    ] {
        assert!(
            schema.lifecycle_events.iter().any(|e| e.name == name),
            "schema missing {name}"
        );
    }
}

#[test]
fn hosting_events_group_in_schema() {
    let schema = build_hook_schema();
    let hosting: Vec<_> = schema
        .lifecycle_events
        .iter()
        .filter(|e| e.name.starts_with("hosting."))
        .collect();
    assert!(
        hosting.len() >= 7,
        "expected hosting hook coverage, got {}",
        hosting.len()
    );
}

#[test]
fn every_event_id_is_contiguous() {
    let mut ids: Vec<u32> = PluginEvent::all()
        .iter()
        .map(|d| d.event.as_u32())
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, (1..=83).collect::<Vec<_>>());
}

#[test]
fn plugins_can_hook_all_83_events() {
    let mut bus = EventBus::new();
    for doc in PluginEvent::all() {
        bus.register("wiring-test", doc.event.as_u32(), noop_hook);
    }
    for doc in PluginEvent::all() {
        let outcome = bus.emit(doc.event, b"{}");
        assert!(outcome.handlers_run >= 1, "no handler for {}", doc.name);
    }
}
