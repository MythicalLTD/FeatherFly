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
pub struct HookGuideDoc {
    pub name: &'static str,
    pub summary: &'static str,
    pub when: &'static str,
    pub input: &'static str,
    pub output: &'static str,
    pub pipeline: &'static str,
    pub route_matching: &'static str,
    pub details: &'static str,
    pub use_cases: &'static [&'static str],
    pub register_example: &'static str,
    pub handler_example: &'static str,
    pub source_path: &'static str,
}

pub const SOURCE_TREE: &str = r#"Key paths in the FeatherFly repository for plugin developers:

| Path | Purpose |
|------|---------|
| `featherfly-plugin-sdk/src/lib.rs` | Plugin API types, macros, return codes |
| `featherfly-plugin-sdk/src/metadata.rs` | Documentation strings consumed by docgen |
| `application/src/daemon.rs` | Startup, config pipeline, HTTP server wiring |
| `application/src/plugins/mod.rs` | Plugin loader, HostApi trampolines |
| `application/src/plugins/events.rs` | Hook registries, config/request/route dispatch |
| `application/src/plugins/request_middleware.rs` | request.intercept + middleware.inject Axum layers |
| `application/src/plugins/routes.rs` | Plugin route registration and dispatch |
| `application/src/plugins/middleware.rs` | JSON response mutation middleware |
| `application/src/config.rs` | Config parse, preview, apply after mutation |
| `plugins/hello/src/lib.rs` | Reference plugin with all v4 hook types |
"#;

pub const RETURN_CODES: &str = r#"## init

| Code | Meaning |
|------|---------|
| `0` | Plugin loaded successfully |
| non-zero | Plugin load fails; `.so` is skipped |

## Lifecycle hooks

| Return | Meaning |
|--------|---------|
| `HookResult::continue()` | Run remaining handlers for this event |
| `HookResult::cancel()` | Stop remaining handlers for this event |

## config.mutate

| Code | Meaning |
|------|---------|
| `CONFIG_MUTATE_MODIFIED` (0) | Output written via `write_yaml_output` |
| `CONFIG_MUTATE_UNCHANGED` (1) | Keep prior YAML bytes |
| negative | Error; prior YAML kept |

## request.intercept / middleware.inject

| Code | Meaning |
|------|---------|
| `REQUEST_CONTINUE` (0) | Pass to next hook or HTTP handler |
| `REQUEST_RESPOND` (1) | Short-circuit; body from `write_request_response` |
| negative | Error; request continues |

## route.register

| Code | Meaning |
|------|---------|
| `ROUTE_OK` (0) | Response written via `write_route_response` |
| negative | Handler error; 500 returned |

## JSON hooks

| Code | Meaning |
|------|---------|
| `JSON_MUTATE_MODIFIED` (0) | Output written via `write_json_output` |
| `JSON_MUTATE_UNCHANGED` (1) | Keep prior JSON |
| negative | Error; prior JSON kept |

## CloudPanel command hooks

| Code | Meaning |
|------|---------|
| `CLOUDPANEL_CONTINUE` (0) | Keep current CLI args and continue |
| `CLOUDPANEL_MODIFIED` (1) | Output written via `write_cloudpanel_args` |
| `CLOUDPANEL_CANCEL` (2) | Cancel the command; optional reason from `write_cloudpanel_cancel` |
| negative | Error; current args kept |
"#;

pub const REQUEST_PIPELINE: &str = r#"## HTTP request stack

```
Client request
    │
    ▼
response_middleware (pass-through on request; mutates JSON on response)
    │
    ▼
request.intercept hooks (outer — auth, rate limits, early reject)
    │
    ▼
middleware.inject hooks (inner — tracing, header checks)
    │
    ▼
handle_request (debug logging)
    │
    ▼
Axum router (core routes + plugin routes)
```

Request hooks receive method, path, headers as JSON, and body bytes. Headers use lowercase keys. `authorization` is redacted.

Route patterns use prefix matching: `/api/*` matches `/api/system`, `/plugins/hello` matches exactly that path prefix chain.

Plugin routes register during `init` and merge into the same router as core API routes."#;

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
    "Rewrite config YAML before the daemon applies settings (config.mutate)",
    "Intercept inbound HTTP requests before handlers run (request.intercept)",
    "Inject request middleware layers (middleware.inject)",
    "Register new HTTP routes on the daemon router (route.register)",
    "Modify JSON API response bodies before they are sent to clients",
    "Inject, remove, or rewrite action steps in API responses",
    "Inspect, mutate, or cancel CloudPanel CLI commands before execution",
    "Cancel remaining handlers for the same event (lifecycle hooks only)",
];

pub const OVERVIEW: &str = r#"FeatherFly plugins are native shared libraries (`.so` files) loaded at daemon startup. They extend the daemon without recompiling FeatherFly itself.

A plugin exports one symbol: `featherfly_plugin_entry`. The daemon loads each `.so`, checks the plugin API version, calls `init`, and keeps the library in memory until shutdown.

Plugins register hooks during `init`. Hook systems:

1. **Lifecycle events** — callbacks fired at fixed points (config loaded, daemon started, etc.).
2. **Config mutation** — rewrite raw YAML before settings are applied.
3. **Request hooks** — run before HTTP handlers (`request.intercept`, `middleware.inject`).
4. **Plugin routes** — register new HTTP endpoints on the daemon router.
5. **JSON mutation hooks** — callbacks that receive JSON, may rewrite it, and return modified output for HTTP responses.
6. **CloudPanel command hooks** — rewrite or cancel CloudPanel CLI operations before `clpctl` runs.

All hooks for a given target run in **plugin load order** (alphabetical by filename in the plugins directory)."#;

pub const LIFECYCLE: &str = r#"1. Daemon reads config.yml (raw bytes)
2. Each `.so` in the plugins directory is loaded
3. For each plugin: `init(host)` runs — register hooks here (including config.mutate)
4. After each plugin init: `plugin.loaded` event fires (payload = plugin name)
5. Registered config.mutate hooks rewrite the YAML pipeline
6. Final config is parsed and applied (directories, logging, pid file)
7. `config.loaded` fires with the post-mutation YAML bytes
8. `daemon.starting` fires before HTTP routes are wired
9. HTTP server starts (core routes + plugin routes; request middleware stack active)
10. `daemon.started` fires (payload = listen address, e.g. `127.0.0.1:9090`)
11. On shutdown: `daemon.stopping` fires, then each plugin's `shutdown` runs"#;

pub const HOST_API: &str = r#"During init, FeatherFly passes a HostApi struct:

- api_version — must match FEATHERFLY_PLUGIN_API_VERSION
- register_hook — register a lifecycle event callback
- register_config_hook — register a config YAML mutation callback
- register_request_hook — register request.intercept or middleware.inject
- register_route — register a plugin HTTP route (GET/POST/PUT/PATCH/DELETE)
- register_json_hook — register a JSON mutation callback
- register_cloudpanel_hook — inspect, mutate, or cancel CloudPanel CLI commands
- log_info — write an info line to the daemon log

Register hooks only inside init. Callback pointers are kept until shutdown.

init return codes: 0 = success. Any other value fails plugin load.

Return codes:
- CONFIG_MUTATE_MODIFIED / JSON_MUTATE_MODIFIED (0) — output written via write_* helpers
- CONFIG_MUTATE_UNCHANGED / JSON_MUTATE_UNCHANGED (1) — keep input unchanged
- REQUEST_CONTINUE (0) — pass request to next hook/handler
- REQUEST_RESPOND (1) — short-circuit with write_request_response
- ROUTE_OK (0) — route handled via write_route_response
- CLOUDPANEL_CONTINUE / CLOUDPANEL_MODIFIED / CLOUDPANEL_CANCEL — continue, replace args, or block a CloudPanel command
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
- Production config: `/etc/featherfly/config.yml`
- Debug (`--debug` or `debug: true`): `./config.yml` in the working directory
- Plugins: `system.plugins_directory` (default `/var/lib/featherfly/plugins/*.so`)
- Config: `system.plugins_directory`, `plugins.enabled`

**Version mismatch:** if `descriptor.api_version` ≠ daemon API version, the plugin is skipped with a warning."#;

pub const FULL_PLUGIN_EXAMPLE: &str = r##"use featherfly_plugin_sdk::{
    declare_plugin, hook, hook_cloudpanel, hook_config, hook_json, hook_request, log_info, route,
    write_cloudpanel_cancel, write_json_output, write_route_response, write_yaml_output,
    CloudPanelCommandContext, ConfigMutateContext, EventContext, HookResult, HostApi,
    JsonMutateContext, JsonMutateTarget, PluginEvent, RequestHookContext, RequestHookPhase,
    RouteHandlerContext, CONFIG_MUTATE_UNCHANGED, CLOUDPANEL_CONTINUE, HTTP_METHOD_GET,
    JSON_MUTATE_UNCHANGED, REQUEST_CONTINUE,
};

extern "C" fn init(host: *const HostApi) -> i32 {
    hook!(host, PluginEvent::DaemonStarted, on_started);
    hook_config!(host, on_config);
    hook_request!(host, RequestHookPhase::Intercept, "/api/*", on_intercept);
    hook_json!(host, JsonMutateTarget::ResponseBody, "/api/system", on_body);
    hook_cloudpanel!(host, on_cloudpanel);
    route!(host, HTTP_METHOD_GET, "/plugins/hello", on_route);
    unsafe { log_info(host, "ready") };
    0
}

extern "C" fn on_started(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

extern "C" fn on_config(ctx: *const ConfigMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.yaml_in_ptr, ctx.yaml_in_len) };
    if input.starts_with(b"# my-plugin\n") {
        return CONFIG_MUTATE_UNCHANGED;
    }
    let mut out = b"# my-plugin\n".to_vec();
    out.extend_from_slice(input);
    write_yaml_output(ctx, &out)
}

extern "C" fn on_intercept(_ctx: *const RequestHookContext) -> i32 {
    REQUEST_CONTINUE
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

extern "C" fn on_cloudpanel(ctx: *const CloudPanelCommandContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let command = unsafe { std::slice::from_raw_parts(ctx.command_ptr, ctx.command_len) };
    if command == b"site:delete" {
        return write_cloudpanel_cancel(ctx, b"site deletion disabled by my-plugin");
    }
    CLOUDPANEL_CONTINUE
}

extern "C" fn on_route(ctx: *const RouteHandlerContext) -> i32 {
    let ctx = unsafe { &*ctx };
    write_route_response(ctx, 200, br#"{"ok":true}"#)
}

declare_plugin! { name: "my-plugin", version: "0.1.0", init: init }"##;

const JSON_EVENT_HANDLER: &str = r##"extern "C" fn on_event(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let payload = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _json = std::str::from_utf8(payload).unwrap_or_default();
    HookResult::r#continue()
}"##;

pub const EVENT_DOCS: &[EventDoc] = &[
    EventDoc {
        event: PluginEvent::DaemonStarting,
        name: "daemon.starting",
        summary: "Daemon is about to start HTTP.",
        when: "After config mutation and final config apply, before the router is served.",
        payload: "Empty.",
        cancelable: false,
        details: "Use for startup initialization that needs the final daemon config.",
        use_cases: &["Warm plugin caches", "Open external connections"],
        register_example: "hook!(host, PluginEvent::DaemonStarting, on_daemon_starting);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DaemonStarted,
        name: "daemon.started",
        summary: "HTTP server is listening.",
        when: "After the TCP listener is bound and before serving requests.",
        payload: "UTF-8 bytes of the listen address, e.g. 127.0.0.1:9090.",
        cancelable: false,
        details: "This is the safest event for reporting daemon availability.",
        use_cases: &["Emit startup metrics", "Notify external monitoring"],
        register_example: "hook!(host, PluginEvent::DaemonStarted, on_daemon_started);",
        handler_example: r##"extern "C" fn on_daemon_started(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let address = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    println!("daemon listening at {}", String::from_utf8_lossy(address));
    HookResult::continue_()
}"##,
    },
    EventDoc {
        event: PluginEvent::DaemonStopping,
        name: "daemon.stopping",
        summary: "Daemon is shutting down.",
        when: "After the HTTP server stops, before plugin shutdown callbacks run.",
        payload: "Empty.",
        cancelable: false,
        details: "Release resources here if you need ordering before plugin shutdown.",
        use_cases: &["Flush queues", "Close external connections"],
        register_example: "hook!(host, PluginEvent::DaemonStopping, on_daemon_stopping);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ConfigLoaded,
        name: "config.loaded",
        summary: "Config file has been parsed.",
        when: "After plugin config.mutate hooks finish and final YAML is parsed.",
        payload: "Post-mutation raw config.yml bytes.",
        cancelable: false,
        details: "Config hooks run before this event; listeners should treat this payload as read-only.",
        use_cases: &["Validate effective config", "Export config metadata"],
        register_example: "hook!(host, PluginEvent::ConfigLoaded, on_config_loaded);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginLoaded,
        name: "plugin.loaded",
        summary: "A plugin finished initializing.",
        when: "Immediately after that plugin's init returns 0.",
        payload: "UTF-8 bytes of the plugin name from its descriptor.",
        cancelable: false,
        details: "Only plugins loaded earlier can observe later plugin.loaded events.",
        use_cases: &["Cross-plugin diagnostics", "Log plugin inventory"],
        register_example: "hook!(host, PluginEvent::PluginLoaded, on_plugin_loaded);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestReceived,
        name: "request.received",
        summary: "HTTP request entered the daemon middleware stack.",
        when: "Before the request is passed to route handlers.",
        payload: "JSON: { request_id, client_ip, method, path, query }.",
        cancelable: false,
        details: "Use request.intercept hooks to short-circuit; this lifecycle event is observational.",
        use_cases: &["Access logs", "Traffic analytics"],
        register_example: "hook!(host, PluginEvent::RequestReceived, on_request_received);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestCompleted,
        name: "request.completed",
        summary: "HTTP request finished.",
        when: "After a response status is available.",
        payload: "JSON: { request_id, client_ip, method, path, query, status, duration_ms }.",
        cancelable: false,
        details: "Fires for core routes and plugin routes.",
        use_cases: &["Latency metrics", "Audit logs"],
        register_example: "hook!(host, PluginEvent::RequestCompleted, on_request_completed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestNotFound,
        name: "request.not_found",
        summary: "Request returned 404.",
        when: "After the fallback handler responds with not found.",
        payload: "JSON: { request_id, client_ip, method, path, query }.",
        cancelable: false,
        details: "Useful with probe protection to understand noisy paths.",
        use_cases: &["Security analytics", "Missing route debugging"],
        register_example: "hook!(host, PluginEvent::RequestNotFound, on_request_not_found);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestBlocked,
        name: "request.blocked",
        summary: "Probe protection blocked a request.",
        when: "Before a blocked client request is rejected.",
        payload: "JSON: { request_id, client_ip, method, path, query }.",
        cancelable: false,
        details: "Fires when an IP is already blocked by probe protection.",
        use_cases: &["Security alerts", "Blocklist observability"],
        register_example: "hook!(host, PluginEvent::RequestBlocked, on_request_blocked);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ProbeClientBlocked,
        name: "probe.client_blocked",
        summary: "Probe protection blocked a client IP.",
        when: "After the unknown route threshold is reached.",
        payload: "JSON: { request_id, client_ip, unknown_hits, block_secs }.",
        cancelable: false,
        details: "The request that crosses the threshold still receives its normal response.",
        use_cases: &["Security alerts", "Abuse tracking"],
        register_example: "hook!(host, PluginEvent::ProbeClientBlocked, on_probe_client_blocked);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::AuthFailed,
        name: "auth.failed",
        summary: "Bearer authentication failed.",
        when: "After an authenticated route rejects a request.",
        payload: "JSON: { request_id, client_ip, method, path, query }.",
        cancelable: false,
        details: "Authorization header values are never included in the payload.",
        use_cases: &["Security alerts", "Panel integration debugging"],
        register_example: "hook!(host, PluginEvent::AuthFailed, on_auth_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RestartScheduled,
        name: "restart.scheduled",
        summary: "Daemon restart was queued.",
        when: "After plugin reload requests schedule a restart.",
        payload: "JSON: { delay_ms, reason? }.",
        cancelable: false,
        details: "Plugins are loaded at startup, so reload uses daemon restart semantics.",
        use_cases: &["Notify operators", "Flush plugin state"],
        register_example: "hook!(host, PluginEvent::RestartScheduled, on_restart_scheduled);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginReloadRequested,
        name: "plugins.reload_requested",
        summary: "Plugin reload API was called.",
        when: "POST /api/system/plugins/reload accepted.",
        payload: "JSON: { plugin_count, delay_ms }.",
        cancelable: false,
        details: "This event fires before restart.scheduled.",
        use_cases: &["Audit plugin reloads", "Notify external automation"],
        register_example: "hook!(host, PluginEvent::PluginReloadRequested, on_reload);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::NodeIdentityGenerated,
        name: "node.identity_generated",
        summary: "Node identity was generated.",
        when: "First boot with empty uuid/token_id/token fields.",
        payload: "JSON: { uuid, token_id }.",
        cancelable: false,
        details: "The token secret is never emitted.",
        use_cases: &["First boot inventory", "Operator notifications"],
        register_example: "hook!(host, PluginEvent::NodeIdentityGenerated, on_identity);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginConfigMutated,
        name: "plugins.config_mutated",
        summary: "Plugin config mutation changed raw YAML.",
        when: "After config.mutate hooks run and output differs from input.",
        payload: "JSON: { hook_count, input_len, output_len }.",
        cancelable: false,
        details: "Fires once per startup if any config hook changes the YAML.",
        use_cases: &["Config audit", "Plugin diagnostics"],
        register_example: "hook!(host, PluginEvent::PluginConfigMutated, on_config_mutated);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginRequestShortCircuited,
        name: "plugins.request_short_circuited",
        summary: "A request hook returned an HTTP response.",
        when: "request.intercept or middleware.inject returns REQUEST_RESPOND.",
        payload: "JSON: { phase, method, path, status }.",
        cancelable: false,
        details: "No downstream handler runs after this event.",
        use_cases: &["Custom auth", "Maintenance responses"],
        register_example: "hook!(host, PluginEvent::PluginRequestShortCircuited, on_short_circuit);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginJsonResponseMutated,
        name: "plugins.json_response_mutated",
        summary: "A JSON response body hook changed a response.",
        when: "After json.response returns modified JSON.",
        payload: "JSON: { method, path, input_len, output_len }.",
        cancelable: false,
        details: "Fires only when output bytes differ from input bytes.",
        use_cases: &["Response enrichment", "Compatibility shims"],
        register_example: "hook!(host, PluginEvent::PluginJsonResponseMutated, on_json_body);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginJsonActionsMutated,
        name: "plugins.json_actions_mutated",
        summary: "A JSON actions hook changed an actions array.",
        when: "After json.actions returns modified JSON.",
        payload: "JSON: { method, path, input_len, output_len }.",
        cancelable: false,
        details: "The result is inserted into the top-level actions field.",
        use_cases: &["Panel action extensions", "Custom links"],
        register_example: "hook!(host, PluginEvent::PluginJsonActionsMutated, on_json_actions);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginRouteInvoked,
        name: "plugins.route_invoked",
        summary: "A plugin route handled a request.",
        when: "After a route.register handler returns ROUTE_OK.",
        payload: "JSON: { plugin, method, path, status }.",
        cancelable: false,
        details: "Plugin routes use the same bearer auth as /api/system.",
        use_cases: &["Plugin route metrics", "Auditing custom APIs"],
        register_example: "hook!(host, PluginEvent::PluginRouteInvoked, on_route_invoked);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginRouteFailed,
        name: "plugins.route_failed",
        summary: "A plugin route handler failed.",
        when: "After the handler returns a negative route status or request body read fails.",
        payload: "JSON: { plugin, method, path, error }.",
        cancelable: false,
        details: "The daemon returns a 4xx/5xx response for the request.",
        use_cases: &["Plugin error alerts", "Debug custom endpoints"],
        register_example: "hook!(host, PluginEvent::PluginRouteFailed, on_route_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::CloudPanelCommandRequested,
        name: "cloudpanel.command_requested",
        summary: "A CloudPanel CLI command was requested.",
        when: "Before CloudPanel command hooks run.",
        payload: "JSON: { operation, command, args, status?, duration_ms, error?, hook_handlers }.",
        cancelable: false,
        details: "Arguments are redacted before lifecycle payloads are emitted.",
        use_cases: &["Audit CloudPanel API usage", "Track requested operations"],
        register_example: "hook!(host, PluginEvent::CloudPanelCommandRequested, on_cloudpanel_requested);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::CloudPanelCommandMutated,
        name: "cloudpanel.command_mutated",
        summary: "A CloudPanel command hook changed CLI args.",
        when: "After cloudpanel.command returns CLOUDPANEL_MODIFIED.",
        payload: "JSON: { operation, command, args, status?, duration_ms, error?, hook_handlers }.",
        cancelable: false,
        details: "Fires once per command when one or more CloudPanel hooks mutate the argument JSON.",
        use_cases: &["Policy rewrites", "Default argument injection"],
        register_example: "hook!(host, PluginEvent::CloudPanelCommandMutated, on_cloudpanel_mutated);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::CloudPanelCommandCancelled,
        name: "cloudpanel.command_cancelled",
        summary: "A CloudPanel command hook cancelled execution.",
        when: "After cloudpanel.command returns CLOUDPANEL_CANCEL.",
        payload: "JSON: { operation, command, args, status?, duration_ms, error?, hook_handlers }.",
        cancelable: false,
        details: "No clpctl process is spawned after cancellation.",
        use_cases: &["Block destructive commands", "Maintenance windows"],
        register_example: "hook!(host, PluginEvent::CloudPanelCommandCancelled, on_cloudpanel_cancelled);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::CloudPanelCommandSucceeded,
        name: "cloudpanel.command_succeeded",
        summary: "A CloudPanel CLI command succeeded.",
        when: "After clpctl exits successfully.",
        payload: "JSON: { operation, command, args, status?, duration_ms, error?, hook_handlers }.",
        cancelable: false,
        details: "Emitted after successful local CLI execution.",
        use_cases: &["Command metrics", "Operator audit trails"],
        register_example: "hook!(host, PluginEvent::CloudPanelCommandSucceeded, on_cloudpanel_succeeded);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::CloudPanelCommandFailed,
        name: "cloudpanel.command_failed",
        summary: "A CloudPanel CLI command failed.",
        when: "After clpctl exits with a non-zero status or cannot run.",
        payload: "JSON: { operation, command, args, status?, duration_ms, error?, hook_handlers }.",
        cancelable: false,
        details: "The error field contains the sanitized CLI error returned to the API caller.",
        use_cases: &["Failure alerts", "CloudPanel troubleshooting"],
        register_example: "hook!(host, PluginEvent::CloudPanelCommandFailed, on_cloudpanel_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
];

pub const CONFIG_HOOK_DOCS: &[HookGuideDoc] = &[HookGuideDoc {
    name: "config.mutate",
    summary: "Rewrite config YAML before FeatherFly applies settings.",
    when: "After all plugins init, before directories/logging/pid are created.",
    input: "Raw config.yml bytes from disk.",
    output: "Replacement YAML bytes via write_yaml_output.",
    pipeline: "Runs in plugin load order. Each hook receives the previous hook's output.",
    route_matching: "N/A — global config pipeline, not route-scoped.",
    details: "Use to inject defaults, strip keys, or apply environment-specific overrides. Output must remain valid YAML. Invalid output keeps the prior bytes. Register with hook_config! during init.",
    use_cases: &[
        "Inject default plugin paths per environment",
        "Strip development-only keys in production",
        "Merge secrets from external sources into YAML",
    ],
    register_example: "hook_config!(host, on_config_mutate);",
    handler_example: r##"extern "C" fn on_config_mutate(ctx: *const ConfigMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.yaml_in_ptr, ctx.yaml_in_len) };
    let mut out = input.to_vec();
    out.extend_from_slice(b"\nplugins:\n  enabled: true\n");
    write_yaml_output(ctx, &out)
}"##,
    source_path: "application/src/plugins/events.rs",
}];

pub const REQUEST_HOOK_DOCS: &[HookGuideDoc] = &[
    HookGuideDoc {
        name: "request.intercept",
        summary: "Inspect inbound HTTP requests before handlers run.",
        when: "On every matching request, outermost request hook layer.",
        input: "Method code, path, headers JSON (lowercase keys), body bytes.",
        output: "REQUEST_CONTINUE or short-circuit via write_request_response.",
        pipeline: "Runs before middleware.inject and before Axum handlers.",
        route_matching: "Prefix match: `/api/*`, `/plugins/hello`, `*` for all routes.",
        details: "First hook layer on inbound requests. Use for auth gates, rate limits, and early JSON error responses. authorization header value is redacted in headers JSON.",
        use_cases: &[
            "API key or custom header validation",
            "Block requests before they hit core handlers",
            "Audit log method + path for matching routes",
        ],
        register_example: "hook_request!(host, RequestHookPhase::Intercept, \"/api/*\", on_intercept);",
        handler_example: r##"extern "C" fn on_intercept(ctx: *const RequestHookContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let headers = unsafe { std::slice::from_raw_parts(ctx.headers_json_ptr, ctx.headers_json_len) };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(headers) else {
        return REQUEST_CONTINUE;
    };
    if value.get("x-api-key").is_some() {
        return REQUEST_CONTINUE;
    }
    write_request_response(ctx, 401, br#"{"error":"unauthorized"}"#)
}"##,
        source_path: "application/src/plugins/request_middleware.rs",
    },
    HookGuideDoc {
        name: "middleware.inject",
        summary: "Inner request middleware layer before handlers.",
        when: "After request.intercept hooks, before handle_request logging.",
        input: "Same as request.intercept.",
        output: "Same as request.intercept.",
        pipeline: "Runs after intercept hooks on the same route prefix.",
        route_matching: "Same prefix rules as request.intercept.",
        details: "Second request hook phase. Use for cross-cutting checks that should run after outer auth but before route handlers — header normalization, tracing tags, request enrichment.",
        use_cases: &[
            "Validate content-type on POST routes",
            "Attach request context for downstream hooks",
            "Soft limits that allow fallback handlers",
        ],
        register_example: "hook_request!(host, RequestHookPhase::Middleware, \"/api/*\", on_middleware);",
        handler_example: r##"extern "C" fn on_middleware(_ctx: *const RequestHookContext) -> i32 {
    REQUEST_CONTINUE
}"##,
        source_path: "application/src/plugins/request_middleware.rs",
    },
];

pub const JSON_HOOK_DOCS: &[JsonHookDoc] = &[
    JsonHookDoc {
        target: JsonMutateTarget::ResponseBody,
        name: "json.response",
        summary: "Mutate the entire JSON response object.",
        input: "Serialized JSON response body bytes.",
        route_matching: "Prefix match on request path. Examples: `/api/system`, `/api`, `*`.",
        pipeline: "Runs first. Output replaces the response object before action hooks run.",
        details: "Return JSON_MUTATE_MODIFIED after calling write_json_output, or JSON_MUTATE_UNCHANGED to keep the prior value.",
        use_cases: &[
            "Add plugin-specific metadata to status responses",
            "Rewrite fields for panel compatibility",
        ],
        register_example: "hook_json!(host, JsonMutateTarget::ResponseBody, \"/api/system\", on_body);",
        handler_example: r##"extern "C" fn on_body(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    value["plugin"] = serde_json::Value::Bool(true);
    let Ok(out) = serde_json::to_vec(&value) else {
        return JSON_MUTATE_UNCHANGED;
    };
    write_json_output(ctx, &out)
}"##,
    },
    JsonHookDoc {
        target: JsonMutateTarget::ResponseActions,
        name: "json.actions",
        summary: "Mutate only the top-level actions array.",
        input: "Serialized JSON array from the response actions field, or [] when absent.",
        route_matching: "Same prefix rules as json.response.",
        pipeline: "Runs after json.response. Output is inserted into the top-level actions field.",
        details: "Use this for panel steps, links, and plugin-specific operator actions.",
        use_cases: &[
            "Add plugin documentation links",
            "Expose plugin-specific actions to CloudPanel",
        ],
        register_example: "hook_json!(host, JsonMutateTarget::ResponseActions, \"/api/system\", on_actions);",
        handler_example: r##"extern "C" fn on_actions(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut actions) = serde_json::from_slice::<Vec<serde_json::Value>>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };
    actions.push(serde_json::json!({
        "id": "plugin_action",
        "label": "Plugin action",
        "step": "GET /plugins/example"
    }));
    let Ok(out) = serde_json::to_vec(&actions) else {
        return JSON_MUTATE_UNCHANGED;
    };
    write_json_output(ctx, &out)
}"##,
    },
];

pub const ROUTE_HOOK_DOCS: &[HookGuideDoc] = &[HookGuideDoc {
    name: "route.register",
    summary: "Expose new HTTP routes on the daemon router.",
    when: "Registered during init; routes active once HTTP server starts.",
    input: "Request body bytes for matching method + path.",
    output: "Response body + status via write_route_response.",
    pipeline: "Plugin routes merge into the main Axum router alongside core API routes.",
    route_matching: "Exact path per registration. Method must match (GET/POST/PUT/PATCH/DELETE).",
    details: "Paths must start with `/`. Duplicate method+path registrations fail at init. Plugin routes use the same Bearer auth middleware as the system API.",
    use_cases: &[
        "Custom webhooks and panel callbacks",
        "Plugin-specific REST APIs without forking FeatherFly",
        "Static JSON config endpoints for operators",
    ],
    register_example: "route!(host, HTTP_METHOD_GET, \"/plugins/hello\", on_hello);",
    handler_example: r##"extern "C" fn on_hello(ctx: *const RouteHandlerContext) -> i32 {
    let ctx = unsafe { &*ctx };
    write_route_response(ctx, 200, br#"{"plugin":"hello"}"#)
}"##,
    source_path: "application/src/plugins/routes.rs",
}];

pub const CLOUDPANEL_HOOK_DOCS: &[HookGuideDoc] = &[HookGuideDoc {
    name: "cloudpanel.command",
    summary: "Inspect, mutate, or cancel CloudPanel CLI commands before clpctl runs.",
    when: "After an authenticated CloudPanel API handler builds typed args, before process spawn.",
    input: "CloudPanelCommandContext with operation, command, and args JSON.",
    output: "CLOUDPANEL_CONTINUE, CLOUDPANEL_MODIFIED via write_cloudpanel_args, or CLOUDPANEL_CANCEL via write_cloudpanel_cancel.",
    pipeline: "Hooks run in plugin load order. Mutated args become input for the next CloudPanel hook.",
    route_matching: "Applies to all /api/system/cloudpanel routes.",
    details: "Use this hook for policy enforcement and defaults. The command name is read-only; only the JSON args can be changed.",
    use_cases: &[
        "Block destructive CloudPanel operations",
        "Inject default paths",
        "Rewrite domains for staging",
    ],
    register_example: "hook_cloudpanel!(host, on_cloudpanel_command);",
    handler_example: r##"extern "C" fn on_cloudpanel_command(ctx: *const CloudPanelCommandContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let command = unsafe { std::slice::from_raw_parts(ctx.command_ptr, ctx.command_len) };
    if command == b"site:delete" {
        return write_cloudpanel_cancel(ctx, b"site deletion blocked");
    }
    CLOUDPANEL_CONTINUE
}"##,
    source_path: "application/src/controllers/system/cloudpanel.rs",
}];

pub const MACROS: &[(&str, &str)] = &[
    (
        "declare_plugin!",
        "Exports featherfly_plugin_entry with name, version, init, and optional shutdown.",
    ),
    ("hook!", "Registers a lifecycle event handler during init."),
    (
        "hook_config!",
        "Registers a config.mutate handler that rewrites YAML before apply.",
    ),
    (
        "hook_request!",
        "Registers request.intercept or middleware.inject for a route prefix.",
    ),
    (
        "route!",
        "Registers a plugin HTTP route (GET/POST/PUT/PATCH/DELETE).",
    ),
    (
        "hook_json!",
        "Registers a JSON mutation handler for a route prefix during init.",
    ),
    (
        "hook_cloudpanel!",
        "Registers a CloudPanel command hook during init.",
    ),
];

pub const HTTP_METHODS: &[(&str, u32)] = &[
    ("GET", 1),
    ("POST", 2),
    ("PUT", 3),
    ("PATCH", 4),
    ("DELETE", 5),
];

#[derive(Debug, Clone, Copy)]
pub struct PlannedHookDoc {
    pub name: &'static str,
    pub status: &'static str,
    pub summary: &'static str,
    pub mixin_role: &'static str,
}

pub const TERMINOLOGY: &str = r#"FeatherFly plugin vocabulary — read this before writing hooks.

| Term | Meaning |
|------|---------|
| **Plugin** | A native `.so` shared library loaded by the daemon at startup. |
| **Host** | The running FeatherFly daemon. Passes `HostApi` to your `init`. |
| **Hook** | A callback your plugin registers during `init`. |
| **Event** | A lifecycle hook fired at a fixed point (startup, shutdown, config load). |
| **Config mutation hook** | Rewrites raw config YAML bytes before settings are applied (`config.mutate`). |
| **Request hook** | Runs before HTTP handlers — `request.intercept` (outer) or `middleware.inject` (inner). |
| **Plugin route** | A handler registered with `route!` that serves a new HTTP endpoint. |
| **JSON mutation hook** | A hook that receives JSON bytes, may rewrite them, and returns output for HTTP responses. |
| **CloudPanel command hook** | A hook that receives CloudPanel operation metadata and CLI args JSON, then continues, rewrites args, or cancels. |
| **Target** | What a hook listens to — an event name, config pipeline, route prefix, or JSON target. |
| **Pipeline** | Ordered chain of hooks for one target. Each hook sees the previous hook's output. |
| **Mixin** | FeatherFly's model for JSON hooks: multiple plugins stack transformations on the same response, like Minecraft mixins layering behavior. |
| **Load order** | Alphabetical by `.so` filename in the plugins directory. Determines pipeline order. |
| **Cancel** | Lifecycle hooks may call `HookResult::cancel()` to skip remaining handlers for that event. |
| **API version** | Integer in `PluginEntry`. Must match `FEATHERFLY_PLUGIN_API_VERSION` or the plugin is skipped. |
| **Action** | A panel step object `{ id, label, step }` returned in API responses for follow-up work. |"#;

pub const ARCHITECTURE: &str = r#"## Mixin-style hook pipeline

FeatherFly plugins extend the daemon **without recompiling it**, similar to how Minecraft mixins inject behavior into existing code paths.

```
Daemon startup
    │
    ├─ load .so files (alphabetical)
    │       └─ init(host) ── register all hooks
    │
    ├─ config.mutate pipeline ──► [plugin A] ──► [plugin B] ──► final YAML
    │
    ├─ config.loaded (post-mutation YAML bytes)
    │
    └─ HTTP listening
            │
            ├─ request.intercept ──► middleware.inject ──► handler
            │
            ├─ plugin routes (route.register)
            │
            ├─ CloudPanel API ──► cloudpanel.command ──► clpctl
            │
            └─ JSON response for /api/*
                    ├─ json.response pipeline
                    └─ json.actions pipeline
```

### How a JSON mixin chain works

1. FeatherFly builds the base JSON response for a route.
2. Each plugin registered for `json.response` on that route prefix runs **in load order**.
3. Plugin 1 receives the original JSON, may mutate it, writes output.
4. Plugin 2 receives plugin 1's output, may mutate again.
5. After all body hooks, `json.actions` hooks run on the `actions` array only.
6. The final JSON is sent to the client.

This is intentionally **composable**: small plugins each do one job (add a field, inject a step, strip sensitive data) instead of one monolithic plugin patching everything.

### Lifecycle vs mutation

| Hook type | When it runs | Can cancel others? | Typical use |
|-----------|--------------|--------------------|-------------|
| Lifecycle event | Fixed daemon milestones | Yes (same event only) | Startup banners, config validation |
| JSON mutation | Every matching HTTP JSON response | No — always runs in pipeline | Response fields, panel actions |
| CloudPanel command | Before clpctl process spawn | Yes — cancels command execution | Policy checks, defaults |

### Versioning

Plugin API **v7** (current) exposes lifecycle events, config mutation, request hooks, plugin routes, JSON mutation, and CloudPanel command hooks. Future hooks will bump the API version so old plugins keep loading safely when new hook types appear."#;

pub const PLANNED_HOOKS: &[PlannedHookDoc] = &[
    PlannedHookDoc {
        name: "config.loaded",
        status: "available",
        summary: "Read-only notification when config YAML is parsed.",
        mixin_role: "Observe before plugins load; validate or log config.",
    },
    PlannedHookDoc {
        name: "plugin.loaded",
        status: "available",
        summary: "Fires after each plugin finishes init.",
        mixin_role: "Coordinate between plugins; detect dependencies.",
    },
    PlannedHookDoc {
        name: "daemon.starting / started / stopping",
        status: "available",
        summary: "Startup and shutdown lifecycle markers.",
        mixin_role: "Warm caches, bind integrations, graceful teardown.",
    },
    PlannedHookDoc {
        name: "json.response",
        status: "available",
        summary: "Mutate full JSON response objects.",
        mixin_role: "Mixin layer on response body — add/remove/rename fields.",
    },
    PlannedHookDoc {
        name: "json.actions",
        status: "available",
        summary: "Mutate the top-level actions array.",
        mixin_role: "Mixin layer on panel wizard steps.",
    },
    PlannedHookDoc {
        name: "config.mutate",
        status: "available",
        summary: "Rewrite config YAML bytes before FeatherFly applies settings.",
        mixin_role: "Mixin on config load — inject defaults, strip keys, env-specific overrides.",
    },
    PlannedHookDoc {
        name: "request.intercept",
        status: "available",
        summary: "Run before an HTTP handler; inspect method, path, headers.",
        mixin_role: "Mixin on inbound requests — auth plugins, rate limits, audit logging.",
    },
    PlannedHookDoc {
        name: "route.register",
        status: "available",
        summary: "Let plugins expose new HTTP routes through the daemon router.",
        mixin_role: "Extend the API surface without forking FeatherFly.",
    },
    PlannedHookDoc {
        name: "middleware.inject",
        status: "available",
        summary: "Insert Axum middleware layers from plugins.",
        mixin_role: "Cross-cutting concerns (tracing, CORS overrides) as plugins.",
    },
    PlannedHookDoc {
        name: "cloudpanel.command",
        status: "available",
        summary: "Inspect, mutate, or cancel CloudPanel CLI command args.",
        mixin_role: "Mixin on CloudPanel operations — policy, defaults, audit.",
    },
];

pub fn plugin_api_version() -> u32 {
    FEATHERFLY_PLUGIN_API_VERSION
}
