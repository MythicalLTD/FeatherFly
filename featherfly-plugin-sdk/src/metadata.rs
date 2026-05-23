use crate::{FEATHERFLY_PLUGIN_API_VERSION, JsonMutateTarget, PluginEvent};

#[derive(Debug, Clone, Copy)]
pub struct EventDoc {
    pub event: PluginEvent,
    pub name: &'static str,
    pub summary: &'static str,
    pub when: &'static str,
    pub payload: &'static str,
    pub cancelable: bool,
    pub details: &'static str,
    pub use_cases: &'static [&'static str],
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
    pub pipeline: &'static str,
    pub details: &'static str,
    pub use_cases: &'static [&'static str],
    pub register_example: &'static str,
    pub handler_example: &'static str,
}

pub const CAPABILITIES: &[&str] = &[
    "Listen to daemon lifecycle events (startup, shutdown, config load)",
    "Run code when other plugins finish loading",
    "Log messages to the daemon log via log_info",
    "Modify JSON API response bodies before they are sent to clients",
    "Inject, remove, or rewrite action steps in API responses",
    "Cancel remaining handlers for the same event (lifecycle hooks only)",
];

pub const OVERVIEW: &str = r#"FeatherFly plugins are native shared libraries (`.so` files) loaded at daemon startup. They extend the daemon without recompiling FeatherFly itself.

A plugin exports one symbol: `featherfly_plugin_entry`. The daemon loads each `.so`, checks the plugin API version, calls `init`, and keeps the library in memory until shutdown.

Plugins register hooks during `init`. There are two hook systems:

1. **Lifecycle events** — callbacks fired at fixed points (config loaded, daemon started, etc.).
2. **JSON mutation hooks** — callbacks that receive JSON, may rewrite it, and return modified output for HTTP responses.

All hooks for a given target run in **plugin load order** (alphabetical by filename in the plugins directory)."#;

pub const LIFECYCLE: &str = r#"1. Daemon reads config.yml
2. `config.loaded` event fires (payload = raw YAML bytes)
3. Each `.so` in the plugins directory is loaded
4. For each plugin: `init(host)` runs — register hooks here
5. After each plugin init: `plugin.loaded` event fires (payload = plugin name)
6. `daemon.starting` fires before HTTP routes are wired
7. HTTP server starts listening
8. `daemon.started` fires (payload = listen address, e.g. `127.0.0.1:9090`)
9. On shutdown: `daemon.stopping` fires, then each plugin's `shutdown` runs"#;

pub const HOST_API: &str = r#"During init, FeatherFly passes a HostApi struct:

- api_version — must match FEATHERFLY_PLUGIN_API_VERSION
- register_hook — register a lifecycle event callback
- register_json_hook — register a JSON mutation callback
- log_info — write an info line to the daemon log

Register hooks only inside init. Callback pointers are kept until shutdown.

init return codes: 0 = success. Any other value fails plugin load.

JSON hook return codes:
- JSON_MUTATE_MODIFIED (0) — output written via write_json_output
- JSON_MUTATE_UNCHANGED (1) — keep input unchanged
- Negative — error; input is kept"#;

pub const BUILD_AND_INSTALL: &str = r#"**Requirements:** same OS and CPU architecture as the FeatherFly binary (e.g. linux x86_64 musl build needs a musl-linked plugin).

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
featherfly-plugin-sdk = { path = "../featherfly-plugin-sdk" }
serde_json = "1"
```

```bash
featherfly plugin build ./my-plugin
featherfly plugin install ./my-plugin   # copies .so to plugins dir
featherfly plugin ship ./my-plugin      # build + install
make plugin-ship PLUGIN=plugins/hello
```

**Paths**
- Production: `/var/lib/featherfly/plugins/*.so`
- Debug (`--debug`): `./data/plugins/*.so`
- Config: `system.plugins_directory`, `plugins.enabled`

**Version mismatch:** if `descriptor.api_version` ≠ daemon API version, the plugin is skipped with a warning."#;

pub const FULL_PLUGIN_EXAMPLE: &str = r#"use featherfly_plugin_sdk::{
    declare_plugin, hook, hook_json, log_info, write_json_output, EventContext, HookResult,
    HostApi, JsonMutateContext, JsonMutateTarget, PluginEvent, JSON_MUTATE_UNCHANGED,
};

extern "C" fn init(host: *const HostApi) -> i32 {
    hook!(host, PluginEvent::DaemonStarted, on_started);
    hook_json!(host, JsonMutateTarget::ResponseBody, "/api/system", on_body);
    hook_json!(host, JsonMutateTarget::ResponseActions, "/api/system", on_actions);
    unsafe { log_info(host, "my-plugin ready") };
    0
}

extern "C" fn on_started(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

extern "C" fn on_body(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    if let Some(map) = value.as_object_mut() {
        map.insert("my_plugin".into(), true.into());
    }
    let Ok(out) = serde_json::to_vec(&value) else { return JSON_MUTATE_UNCHANGED };
    write_json_output(ctx, &out)
}

extern "C" fn on_actions(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut actions) = serde_json::from_slice::<Vec<serde_json::Value>>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    actions.push(serde_json::json!({
        "id": "my_step",
        "label": "My plugin step",
        "step": "GET /api/system/plugins"
    }));
    let Ok(out) = serde_json::to_vec(&actions) else { return JSON_MUTATE_UNCHANGED };
    write_json_output(ctx, &out)
}

declare_plugin! { name: "my-plugin", version: "0.1.0", init: init }"#;

pub const EVENT_DOCS: &[EventDoc] = &[
    EventDoc {
        event: PluginEvent::ConfigLoaded,
        name: "config.loaded",
        summary: "Config file has been parsed.",
        when: "Before any plugin `.so` is loaded.",
        payload: "Raw bytes of the active config YAML file.",
        cancelable: true,
        details: "Use this to inspect config before plugins initialize. The payload is the exact file contents — parse as UTF-8 YAML in your handler if needed. You cannot change config from this hook; it is read-only notification.",
        use_cases: &[
            "Validate config and log warnings",
            "Record config hash for diagnostics",
            "Skip registering hooks based on config content",
        ],
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
        summary: "A plugin finished initializing.",
        when: "Immediately after that plugin's `init` returns 0.",
        payload: "UTF-8 bytes of the plugin name from its descriptor.",
        cancelable: true,
        details: "Fires once per successfully loaded plugin, including plugins loaded before yours. Useful for coordination between plugins. Does not fire for plugins that failed to load.",
        use_cases: &[
            "Track which plugins are present",
            "Defer work until a dependency plugin loaded",
            "Audit plugin load order",
        ],
        register_example: "hook!(host, PluginEvent::PluginLoaded, on_plugin_loaded);",
        handler_example: r#"extern "C" fn on_plugin_loaded(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let name = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _name = std::str::from_utf8(name).unwrap_or_default();
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::DaemonStarting,
        name: "daemon.starting",
        summary: "Daemon is about to start HTTP.",
        when: "After all plugins loaded, before Axum router is built.",
        payload: "Empty (zero-length).",
        cancelable: true,
        details: "Last chance to register nothing new — init has already run. Use for prep work that must happen before the API exists. Returning cancel stops later handlers on this event only; it does not stop the daemon.",
        use_cases: &[
            "Warm up caches or connections",
            "Log startup banner from plugin",
            "Validate environment before HTTP binds",
        ],
        register_example: "hook!(host, PluginEvent::DaemonStarting, on_daemon_starting);",
        handler_example: r#"extern "C" fn on_daemon_starting(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::DaemonStarted,
        name: "daemon.started",
        summary: "HTTP server is listening.",
        when: "Right before connections are accepted.",
        payload: "Listen address as UTF-8, e.g. `0.0.0.0:9090`.",
        cancelable: true,
        details: "The daemon is fully operational. Safe to assume API routes and plugin JSON hooks are active. Payload is the socket address string logged at startup.",
        use_cases: &[
            "Start background work tied to the running daemon",
            "Register external integrations with the listen URL",
            "Emit startup metrics",
        ],
        register_example: "hook!(host, PluginEvent::DaemonStarted, on_daemon_started);",
        handler_example: r#"extern "C" fn on_daemon_started(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _addr = std::str::from_utf8(addr).unwrap_or_default();
    HookResult::r#continue()
}"#,
    },
    EventDoc {
        event: PluginEvent::DaemonStopping,
        name: "daemon.stopping",
        summary: "Daemon is shutting down.",
        when: "Before plugin `shutdown` callbacks run.",
        payload: "Empty (zero-length).",
        cancelable: true,
        details: "Graceful shutdown in progress. Release resources in your shutdown function; use this event for cross-plugin shutdown coordination. HTTP may still be winding down.",
        use_cases: &[
            "Flush buffers or close connections",
            "Notify external systems",
            "Coordinate ordered shutdown between plugins",
        ],
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
        summary: "Mutate the entire JSON response object.",
        input: "The full response JSON object as bytes.",
        route_matching: "Prefix match on request path. Examples: `/api/system`, `/api`, `*` (all JSON routes).",
        pipeline: "Runs first. Output replaces the response object before action hooks run.",
        details: "Parse input as JSON object, modify fields, serialize back with write_json_output. Invalid JSON or non-object input should return JSON_MUTATE_UNCHANGED. Each plugin receives the previous plugin's output.",
        use_cases: &[
            "Add plugin metadata fields to API responses",
            "Strip sensitive fields from responses",
            "Rename or normalize field names for panels",
        ],
        register_example: "hook_json!(host, JsonMutateTarget::ResponseBody, \"/api/system\", on_body);",
        handler_example: r#"extern "C" fn on_body(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    if let Some(map) = value.as_object_mut() {
        map.insert("my_plugin".into(), true.into());
    }
    let Ok(out) = serde_json::to_vec(&value) else { return JSON_MUTATE_UNCHANGED };
    write_json_output(ctx, &out)
}"#,
    },
    JsonHookDoc {
        target: JsonMutateTarget::ResponseActions,
        name: "json.actions",
        summary: "Mutate only the top-level `actions` array.",
        input: "JSON array of action objects. Empty array `[]` if the route has no actions yet.",
        route_matching: "Same prefix rules as json.response.",
        pipeline: "Runs after body hooks. Only the actions array is passed in; result is merged back into the response under the `actions` key.",
        details: "Action objects use `{ \"id\": \"...\", \"label\": \"...\", \"step\": \"GET /path\" }`. Filter with retain(), push new steps, or rewrite labels. If you inject actions on a route that had none, the merged result adds an actions key.",
        use_cases: &[
            "Remove steps the panel should not show",
            "Inject custom wizard steps for operators",
            "Rewrite step labels for localization",
        ],
        register_example: "hook_json!(host, JsonMutateTarget::ResponseActions, \"/api/system\", on_actions);",
        handler_example: r#"extern "C" fn on_actions(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut actions) = serde_json::from_slice::<Vec<serde_json::Value>>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    actions.retain(|a| a.get("id").and_then(|v| v.as_str()) != Some("remove_me"));
    actions.push(serde_json::json!({
        "id": "custom",
        "label": "Custom step",
        "step": "GET /api/system/plugins"
    }));
    let Ok(out) = serde_json::to_vec(&actions) else { return JSON_MUTATE_UNCHANGED };
    write_json_output(ctx, &out)
}"#,
    },
];

pub const MACROS: &[(&str, &str)] = &[
    (
        "declare_plugin!",
        "Exports featherfly_plugin_entry with name, version, init, and optional shutdown.",
    ),
    (
        "hook!",
        "Registers a lifecycle event handler during init. Returns init error code if registration fails.",
    ),
    (
        "hook_json!",
        "Registers a JSON mutation handler for a route prefix during init.",
    ),
];

pub const HTTP_METHODS: &[(&str, u32)] = &[
    ("GET", 1),
    ("POST", 2),
    ("PUT", 3),
    ("PATCH", 4),
    ("DELETE", 5),
];

pub fn plugin_api_version() -> u32 {
    FEATHERFLY_PLUGIN_API_VERSION
}
