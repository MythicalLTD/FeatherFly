use crate::{JsonMutateTarget, PluginEvent, FEATHERFLY_PLUGIN_API_VERSION};

#[derive(Debug, Clone, Copy)]
pub struct EventDoc {
    pub event: PluginEvent,
    pub name: &'static str,
    pub summary: &'static str,
    pub when: &'static str,
    pub payload: &'static str,
    pub cancelable: bool,
    pub register_example: &'static str,
    pub handler_example: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct JsonHookDoc {
    pub target: JsonMutateTarget,
    pub name: &'static str,
    pub summary: &'static str,
    pub input: &'static str,
    pub route_matching: &'static str,
    pub register_example: &'static str,
    pub handler_example: &'static str,
}

pub const EVENT_DOCS: &[EventDoc] = &[
    EventDoc {
        event: PluginEvent::ConfigLoaded,
        name: "config.loaded",
        summary: "Fires after the daemon config file has been parsed.",
        when: "During plugin directory scan, before any `.so` files are loaded.",
        payload: "Raw YAML bytes of the active config file.",
        cancelable: true,
        register_example: "hook!(host, PluginEvent::ConfigLoaded, on_config_loaded);",
        handler_example: r#"extern "C" fn on_config_loaded(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let payload = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _yaml = std::str::from_utf8(payload).unwrap_or_default();
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::PluginLoaded,
        name: "plugin.loaded",
        summary: "Fires after each plugin finishes initialization.",
        when: "Immediately after a plugin's `init` returns success.",
        payload: "UTF-8 plugin name bytes.",
        cancelable: true,
        register_example: "hook!(host, PluginEvent::PluginLoaded, on_plugin_loaded);",
        handler_example: r#"extern "C" fn on_plugin_loaded(ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::DaemonStarting,
        name: "daemon.starting",
        summary: "Fires before the HTTP server is constructed.",
        when: "After plugins are loaded and before Axum routes are wired.",
        payload: "Empty.",
        cancelable: true,
        register_example: "hook!(host, PluginEvent::DaemonStarting, on_daemon_starting);",
        handler_example: r#"extern "C" fn on_daemon_starting(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::DaemonStarted,
        name: "daemon.started",
        summary: "Fires when the HTTP server is listening.",
        when: "Right before `axum::serve` begins accepting connections.",
        payload: "Listen address as UTF-8 bytes, e.g. `127.0.0.1:8080`.",
        cancelable: true,
        register_example: "hook!(host, PluginEvent::DaemonStarted, on_daemon_started);",
        handler_example: r#"extern "C" fn on_daemon_started(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let address = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _addr = std::str::from_utf8(address).unwrap_or_default();
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::DaemonStopping,
        name: "daemon.stopping",
        summary: "Fires during graceful shutdown.",
        when: "Before plugin `shutdown` functions are invoked.",
        payload: "Empty.",
        cancelable: true,
        register_example: "hook!(host, PluginEvent::DaemonStopping, on_daemon_stopping);",
        handler_example: r#"extern "C" fn on_daemon_stopping(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}"#,
    },
];

pub const JSON_HOOK_DOCS: &[JsonHookDoc] = &[
    JsonHookDoc {
        target: JsonMutateTarget::ResponseBody,
        name: "json.response",
        summary: "Mutate the full JSON response object before it is sent.",
        input: "The serialized JSON response body as an object.",
        route_matching: "Prefix match on the request path. Use `/api/system` or `*` for all JSON routes.",
        register_example:
            "hook_json!(host, JsonMutateTarget::ResponseBody, \"/api/system\", on_system_body);",
        handler_example: r#"extern "C" fn on_system_body(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    if let Some(map) = value.as_object_mut() {
        map.insert("my_plugin".into(), serde_json::json!(true));
    }
    let Ok(out) = serde_json::to_vec(&value) else {
        return JSON_MUTATE_UNCHANGED;
    };
    write_json_output(ctx, &out)
}"#,
    },
    JsonHookDoc {
        target: JsonMutateTarget::ResponseActions,
        name: "json.actions",
        summary: "Mutate only the top-level `actions` array on API responses.",
        input: "A JSON array of action objects with `id`, `label`, and optional `step` fields.",
        route_matching: "Prefix match on the request path. Runs after body hooks; changes are merged back into the response.",
        register_example:
            "hook_json!(host, JsonMutateTarget::ResponseActions, \"/api/system\", on_system_actions);",
        handler_example: r#"extern "C" fn on_system_actions(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut actions) = serde_json::from_slice::<Vec<serde_json::Value>>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    actions.retain(|a| a.get("id").and_then(|v| v.as_str()) != Some("diagnostics"));
    actions.push(serde_json::json!({
        "id": "custom_step",
        "label": "Open plugin docs",
        "step": "GET /docs/plugins"
    }));
    let Ok(out) = serde_json::to_vec(&actions) else {
        return JSON_MUTATE_UNCHANGED;
    };
    write_json_output(ctx, &out)
}"#,
    },
];

pub fn plugin_api_version() -> u32 {
    FEATHERFLY_PLUGIN_API_VERSION
}
