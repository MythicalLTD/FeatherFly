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
- log_info — write an info line to the daemon log

Register hooks only inside init. Callback pointers are kept until shutdown.

init return codes: 0 = success. Any other value fails plugin load.

Return codes:
- CONFIG_MUTATE_MODIFIED / JSON_MUTATE_MODIFIED (0) — output written via write_* helpers
- CONFIG_MUTATE_UNCHANGED / JSON_MUTATE_UNCHANGED (1) — keep input unchanged
- REQUEST_CONTINUE (0) — pass request to next hook/handler
- REQUEST_RESPOND (1) — short-circuit with write_request_response
- ROUTE_OK (0) — route handled via write_route_response
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
    declare_plugin, hook, hook_config, hook_json, hook_request, log_info, route,
    write_json_output, write_route_response, write_yaml_output,
    ConfigMutateContext, EventContext, HookResult, HostApi, JsonMutateContext, JsonMutateTarget,
    PluginEvent, RequestHookContext, RequestHookPhase, RouteHandlerContext,
    CONFIG_MUTATE_UNCHANGED, HTTP_METHOD_GET, JSON_MUTATE_UNCHANGED, REQUEST_CONTINUE,
};

extern "C" fn init(host: *const HostApi) -> i32 {
    hook!(host, PluginEvent::DaemonStarted, on_started);
    hook_config!(host, on_config);
    hook_request!(host, RequestHookPhase::Intercept, "/api/*", on_intercept);
    hook_json!(host, JsonMutateTarget::ResponseBody, "/api/system", on_body);
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
        event: PluginEvent::ConfigLoaded,
        name: "config.loaded",
        summary: "Config file has been parsed.",
        when: "After all plugins load and config.mutate hooks finish.",
        payload: "Post-mutation raw bytes of config.yml.",
        cancelable: true,
        details: "Fires once per startup with the YAML bytes that were actually applied — after config.mutate hooks ran. Parse as UTF-8 YAML in your handler. This is read-only; use config.mutate to change settings before apply.",
        use_cases: &[
            "Validate config and log warnings",
            "Record config hash for diagnostics",
            "Skip registering hooks based on config content",
        ],
        register_example: "hook!(host, PluginEvent::ConfigLoaded, on_config_loaded);",
        handler_example: r##"extern "C" fn on_config_loaded(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let payload = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _yaml = std::str::from_utf8(payload).unwrap_or_default();
    HookResult::r#continue()
}"##,
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
        handler_example: r##"extern "C" fn on_plugin_loaded(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let name = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _name = std::str::from_utf8(name).unwrap_or_default();
    HookResult::r#continue()
}"##,
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
        handler_example: r##"extern "C" fn on_daemon_starting(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}"##,
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
        handler_example: r##"extern "C" fn on_daemon_started(ctx: *const EventContext) -> HookResult {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { std::slice::from_raw_parts(ctx.payload_ptr, ctx.payload_len) };
    let _addr = std::str::from_utf8(addr).unwrap_or_default();
    HookResult::r#continue()
}"##,
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
        handler_example: r##"extern "C" fn on_daemon_stopping(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}"##,
    },
    EventDoc {
        event: PluginEvent::RequestReceived,
        name: "request.received",
        summary: "An HTTP request reached the daemon.",
        when: "Automatically, before routing — every request including /health.",
        payload: "JSON: `{ client_ip, method, path, query? }`",
        cancelable: true,
        details: "Fires once per inbound request. Use for auditing, metrics, or rate-limit coordination. Emitted only when at least one plugin registered this event.",
        use_cases: &[
            "Log or count requests by client IP",
            "Feed external analytics",
            "Trigger plugin-side caching keyed by path",
        ],
        register_example: "hook!(host, PluginEvent::RequestReceived, on_request);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestCompleted,
        name: "request.completed",
        summary: "An HTTP request finished.",
        when: "Automatically, after the response status is known.",
        payload: "JSON: `{ client_ip, method, path, query?, status, duration_ms }`",
        cancelable: true,
        details: "Always paired with request.received for the same request. duration_ms is wall time inside FeatherFly middleware stack.",
        use_cases: &[
            "Latency histograms per route",
            "Audit successful API calls",
            "Alert on slow requests",
        ],
        register_example: "hook!(host, PluginEvent::RequestCompleted, on_request_done);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestNotFound,
        name: "request.not_found",
        summary: "Request returned HTTP 404.",
        when: "Automatically when no route matched.",
        payload: "JSON: `{ client_ip, method, path, query? }`",
        cancelable: true,
        details: "Typical source is internet scanners probing random paths. Often co-occurring with probe.client_blocked when protection is enabled.",
        use_cases: &[
            "Detect scanner traffic",
            "Custom 404 analytics",
            "Feed SIEM with probe paths",
        ],
        register_example: "hook!(host, PluginEvent::RequestNotFound, on_not_found);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RequestBlocked,
        name: "request.blocked",
        summary: "Probe protection rejected a request.",
        when: "When a blocked client IP hits the API (HTTP 429).",
        payload: "JSON: `{ client_ip, method, path, query? }`",
        cancelable: false,
        details: "The handler never runs — this fires instead. Client was blocked by security.probe_protection after too many 404s.",
        use_cases: &[
            "Audit blocked scanner IPs",
            "Notify operators of active probes",
        ],
        register_example: "hook!(host, PluginEvent::RequestBlocked, on_blocked);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ProbeClientBlocked,
        name: "probe.client_blocked",
        summary: "An IP was blocked for probe bursts.",
        when: "When unknown-route count exceeds security.probe_protection.max_404s.",
        payload: "JSON: `{ client_ip, unknown_hits, block_secs }`",
        cancelable: false,
        details: "Fires once when the block starts, not on every subsequent request.",
        use_cases: &[
            "Pager alerts on scanner blocks",
            "Write block list to external firewall",
        ],
        register_example: "hook!(host, PluginEvent::ProbeClientBlocked, on_probe_block);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::AuthFailed,
        name: "auth.failed",
        summary: "Bearer authentication failed on /api/system/*.",
        when: "Missing, malformed, or wrong Authorization header.",
        payload: "JSON: `{ client_ip, method, path, query?, reason }`",
        cancelable: false,
        details: "Only authenticated API routes emit this — public /health does not.",
        use_cases: &["Detect brute-force token guessing", "Security audit trail"],
        register_example: "hook!(host, PluginEvent::AuthFailed, on_auth_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ConfigApplied,
        name: "config.applied",
        summary: "Remote config was written via API.",
        when: "After PUT /api/system/config succeeds.",
        payload: "JSON: `{ path, requires_restart, restart_reasons[], restart_scheduled }`",
        cancelable: false,
        details: "Fires after YAML is validated and saved. restart_scheduled is true when restart_if_required was set and a restart was queued.",
        use_cases: &[
            "Audit config changes from FeatherPanel",
            "Notify external config sync",
        ],
        register_example: "hook!(host, PluginEvent::ConfigApplied, on_config_applied);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ConfigExported,
        name: "config.exported",
        summary: "Remote config was read via API.",
        when: "After GET /api/system/config succeeds.",
        payload: "JSON: `{ path }`",
        cancelable: false,
        details: "Does not include YAML bytes (use config.loaded at startup for full content).",
        use_cases: &["Audit panel config reads"],
        register_example: "hook!(host, PluginEvent::ConfigExported, on_config_exported);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RestartScheduled,
        name: "restart.scheduled",
        summary: "Daemon restart was queued.",
        when: "POST /api/system/restart, config apply with restart, plugin reload, or update apply.",
        payload: "JSON: `{ delay_ms, reason? }`",
        cancelable: false,
        details: "The process re-execs itself after delay_ms. reason is set when provided by the API caller.",
        use_cases: &[
            "Coordinate graceful shutdown with plugins",
            "Log operator-initiated restarts",
        ],
        register_example: "hook!(host, PluginEvent::RestartScheduled, on_restart);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PluginReloadRequested,
        name: "plugins.reload_requested",
        summary: "Plugin reload API was called.",
        when: "POST /api/system/plugins/reload accepted.",
        payload: "JSON: `{ plugin_count, delay_ms }`",
        cancelable: false,
        details: "Plugins only load at startup — this always schedules a full daemon restart.",
        use_cases: &["Audit plugin reload operations"],
        register_example: "hook!(host, PluginEvent::PluginReloadRequested, on_reload);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::UpdateChecked,
        name: "update.checked",
        summary: "GitHub update check finished.",
        when: "After GET /api/system/update (and returns the same JSON as the API).",
        payload: "UpdateStatus JSON (check_ok, update_available, download_url, sha256, …)",
        cancelable: false,
        details: "Also useful to mirror startup update checks if you hook from another plugin after daemon.started.",
        use_cases: &["Panel-side update notifications", "Custom update banners"],
        register_example: "hook!(host, PluginEvent::UpdateChecked, on_update_check);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::UpdateApplied,
        name: "update.applied",
        summary: "GitHub update was installed.",
        when: "Background apply from POST /api/system/update/apply succeeded.",
        payload: "JSON: `{ channel, installed_version?, installed_commit?, binary_path }`",
        cancelable: false,
        details: "Binary replacement is complete; restart may follow if requested.",
        use_cases: &["Post-update hooks", "Version reporting to external systems"],
        register_example: "hook!(host, PluginEvent::UpdateApplied, on_update_applied);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::UpdateApplyFailed,
        name: "update.apply_failed",
        summary: "GitHub update apply failed.",
        when: "Background apply from POST /api/system/update/apply failed.",
        payload: "JSON: `{ error }`",
        cancelable: false,
        details: "Check daemon logs for full error chain.",
        use_cases: &["Alert operators on failed auto-updates"],
        register_example: "hook!(host, PluginEvent::UpdateApplyFailed, on_update_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::UpgradeStarted,
        name: "upgrade.started",
        summary: "Panel-initiated binary upgrade started.",
        when: "POST /api/system/upgrade accepted.",
        payload: "JSON: `{ url }`",
        cancelable: false,
        details: "Legacy panel upgrade path with explicit URL and SHA256. Prefer update.applied for GitHub channel updates.",
        use_cases: &["Audit manual upgrade downloads"],
        register_example: "hook!(host, PluginEvent::UpgradeStarted, on_upgrade_start);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::UpgradeCompleted,
        name: "upgrade.completed",
        summary: "Panel-initiated upgrade finished.",
        when: "Background upgrade task succeeded.",
        payload: "JSON: `{ applied: true }`",
        cancelable: false,
        details: "Binary replaced; restart command from panel payload may have been spawned.",
        use_cases: &["Confirm upgrade success to external monitoring"],
        register_example: "hook!(host, PluginEvent::UpgradeCompleted, on_upgrade_done);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::UpgradeFailed,
        name: "upgrade.failed",
        summary: "Panel-initiated upgrade failed.",
        when: "Background upgrade task failed.",
        payload: "JSON: `{ error }`",
        cancelable: false,
        details: "Download, checksum, or replace step failed.",
        use_cases: &["Alert on failed panel upgrades"],
        register_example: "hook!(host, PluginEvent::UpgradeFailed, on_upgrade_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::NodeIdentityGenerated,
        name: "node.identity_generated",
        summary: "Node UUID and token_id were auto-generated.",
        when: "First boot when uuid/token_id/token were empty in config.yml.",
        payload: "JSON: `{ uuid, token_id }` (secret token is never included).",
        cancelable: false,
        details: "The new identity is persisted to config.yml before the daemon continues startup.",
        use_cases: &[
            "Register node with FeatherPanel using the generated uuid",
            "Audit first-time provisioning",
        ],
        register_example: "hook!(host, PluginEvent::NodeIdentityGenerated, on_identity);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DockerReady,
        name: "docker.ready",
        summary: "Docker engine is connected and the bridge network is ready.",
        when: "After successful docker ping and network ensure during startup.",
        payload: "JSON: `{ socket, network, network_created, api_version? }`",
        cancelable: false,
        details: "network_created is true when featherfly_nw (or configured name) was created this boot.",
        use_cases: &[
            "Start container-dependent background work",
            "Report Docker version to external monitoring",
        ],
        register_example: "hook!(host, PluginEvent::DockerReady, on_docker_ready);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DockerUnavailable,
        name: "docker.unavailable",
        summary: "Docker was enabled but could not be prepared.",
        when: "docker.enabled is true but connect, ping, or network setup failed.",
        payload: "JSON: `{ socket, error }`",
        cancelable: false,
        details: "Container API routes return HTTP 503 until Docker is fixed and the daemon restarted.",
        use_cases: &["Alert operators when Docker socket is missing"],
        register_example: "hook!(host, PluginEvent::DockerUnavailable, on_docker_down);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerCreated,
        name: "container.created",
        summary: "A container was created via the API.",
        when: "POST /api/containers/create succeeded.",
        payload: "JSON: `{ id, name, image, started }`",
        cancelable: false,
        details: "started reflects whether the container was started immediately after create.",
        use_cases: &[
            "Audit site provisioning",
            "Sync inventory with FeatherPanel",
        ],
        register_example: "hook!(host, PluginEvent::ContainerCreated, on_created);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerStarted,
        name: "container.started",
        summary: "A container was started via the API.",
        when: "POST /api/containers/{id}/start succeeded.",
        payload: "JSON: `{ id }`",
        cancelable: false,
        details: "Also emitted when create is called with start: true (after create succeeds).",
        use_cases: &["Track container uptime", "Trigger health checks"],
        register_example: "hook!(host, PluginEvent::ContainerStarted, on_started);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerStopped,
        name: "container.stopped",
        summary: "A container was stopped via the API.",
        when: "POST /api/containers/{id}/stop succeeded.",
        payload: "JSON: `{ id }`",
        cancelable: false,
        details: "Uses Docker stop semantics (graceful then kill).",
        use_cases: &["Audit maintenance stops", "Pause billing or metrics"],
        register_example: "hook!(host, PluginEvent::ContainerStopped, on_stopped);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerRemoved,
        name: "container.removed",
        summary: "A container was removed via the API.",
        when: "DELETE /api/containers/{id} succeeded.",
        payload: "JSON: `{ id, force }`",
        cancelable: false,
        details: "force matches the query parameter on the delete request.",
        use_cases: &[
            "Clean up external inventory",
            "Audit destructive operations",
        ],
        register_example: "hook!(host, PluginEvent::ContainerRemoved, on_removed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerRestarted,
        name: "container.restarted",
        summary: "A container was restarted via the API.",
        when: "POST /api/containers/{id}/restart succeeded.",
        payload: "JSON: `{ id }`",
        cancelable: false,
        details: "Uses Docker restart semantics.",
        use_cases: &["Audit maintenance restarts", "Track uptime resets"],
        register_example: "hook!(host, PluginEvent::ContainerRestarted, on_restarted);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerInspected,
        name: "container.inspected",
        summary: "A container was inspected via the API.",
        when: "GET /api/containers/{id} succeeded.",
        payload: "JSON: `{ id }` (light payload; full inspect is in the HTTP response only).",
        cancelable: false,
        details: "Plugins receive the container id only to avoid large inspect blobs.",
        use_cases: &[
            "Audit inspect requests",
            "Trigger sync with panel inventory",
        ],
        register_example: "hook!(host, PluginEvent::ContainerInspected, on_inspected);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ImagePulled,
        name: "image.pulled",
        summary: "An image was pulled via the API.",
        when: "POST /api/containers/{id}/pull succeeded.",
        payload: "JSON: `{ image, container_id? }`",
        cancelable: false,
        details: "container_id is set when pull was triggered for a specific container.",
        use_cases: &["Track image updates", "Audit deployment prep"],
        register_example: "hook!(host, PluginEvent::ImagePulled, on_image_pulled);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerOperationFailed,
        name: "container.operation_failed",
        summary: "A container operation failed.",
        when: "Any container API operation (restart, logs, exec, pull, etc.) returns an error.",
        payload: "JSON: `{ id, operation, error }`",
        cancelable: false,
        details: "Shared failure event; operation names match API actions (restart, logs, exec, pull).",
        use_cases: &["Alert on failed ops", "Feed panel error reporting"],
        register_example: "hook!(host, PluginEvent::ContainerOperationFailed, on_op_failed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerLogsStreamed,
        name: "container.logs_streamed",
        summary: "Container log streaming was opened.",
        when: "GET /api/containers/{id}/logs succeeded and stream started.",
        payload: "JSON: `{ id, follow, tail? }`",
        cancelable: false,
        details: "Emitted once when the stream opens, not per log line.",
        use_cases: &["Audit log access", "Rate-limit log streaming"],
        register_example: "hook!(host, PluginEvent::ContainerLogsStreamed, on_logs);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerExecStarted,
        name: "container.exec_started",
        summary: "An exec session was created for a container.",
        when: "POST /api/containers/{id}/exec succeeded.",
        payload: "JSON: `{ container_id, exec_id, tty }`",
        cancelable: false,
        details: "Attach via WebSocket at GET /api/containers/{id}/exec/ws?exec_id=...",
        use_cases: &["Audit shell access", "Session tracking"],
        register_example: "hook!(host, PluginEvent::ContainerExecStarted, on_exec_start);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ContainerExecEnded,
        name: "container.exec_ended",
        summary: "An exec WebSocket session ended.",
        when: "Exec WebSocket attach closes (normal or error).",
        payload: "JSON: `{ container_id, exec_id }`",
        cancelable: false,
        details: "Emitted when the attach stream finishes.",
        use_cases: &["Audit session duration", "Cleanup session state"],
        register_example: "hook!(host, PluginEvent::ContainerExecEnded, on_exec_end);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PanelConnected,
        name: "panel.connected",
        summary: "Outbound WebSocket to FeatherPanel connected.",
        when: "Panel client successfully connects when remote is configured.",
        payload: "JSON: `{ remote, uuid }`",
        cancelable: false,
        details: "Only active when config.remote is non-empty.",
        use_cases: &["Start panel-dependent work", "Update node status UI"],
        register_example: "hook!(host, PluginEvent::PanelConnected, on_panel_up);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PanelDisconnected,
        name: "panel.disconnected",
        summary: "Outbound WebSocket to FeatherPanel disconnected.",
        when: "Panel connection lost; client will reconnect with backoff.",
        payload: "JSON: `{ remote, reason? }`",
        cancelable: false,
        details: "Automatic reconnect is handled by the daemon.",
        use_cases: &["Alert operators", "Pause panel sync"],
        register_example: "hook!(host, PluginEvent::PanelDisconnected, on_panel_down);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PanelMessageReceived,
        name: "panel.message_received",
        summary: "Instruction received from FeatherPanel over WebSocket.",
        when: "Panel sends a JSON message to the node.",
        payload: "JSON: `{ type, correlation_id? }` (full body in logs at debug level).",
        cancelable: false,
        details: "Dispatcher routes known types to Docker operations.",
        use_cases: &["Extend panel protocol", "Audit remote commands"],
        register_example: "hook!(host, PluginEvent::PanelMessageReceived, on_panel_in);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::PanelMessageSent,
        name: "panel.message_sent",
        summary: "Event or response sent to FeatherPanel over WebSocket.",
        when: "Daemon forwards a lifecycle event or instruction result to the panel.",
        payload: "JSON: `{ type, event? }`",
        cancelable: false,
        details: "Includes fan-out of container and daemon events.",
        use_cases: &["Debug panel sync", "Verify event delivery"],
        register_example: "hook!(host, PluginEvent::PanelMessageSent, on_panel_out);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::StatsCollected,
        name: "stats.collected",
        summary: "Host or container stats were collected via the API.",
        when: "GET /api/stats or GET /api/containers/{id}/stats succeeded.",
        payload: "JSON: `{ scope, id? }` — scope is host or container.",
        cancelable: false,
        details: "Emitted once per API request.",
        use_cases: &["Metrics pipelines", "Billing usage snapshots"],
        register_example: "hook!(host, PluginEvent::StatsCollected, on_stats);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::SiteProvisioned,
        name: "site.provisioned",
        summary: "A hosted site was fully provisioned.",
        when: "POST /api/sites succeeded.",
        payload: "JSON: `{ id, name, domain, template, container_id }`",
        cancelable: false,
        details: "Includes network, volume, proxy labels, and container creation.",
        use_cases: &["Sync site inventory with FeatherPanel"],
        register_example: "hook!(host, PluginEvent::SiteProvisioned, on_site);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::SiteDestroyed,
        name: "site.destroyed",
        summary: "A hosted site was destroyed.",
        when: "DELETE /api/sites/{id} succeeded.",
        payload: "JSON: `{ id }`",
        cancelable: false,
        details: "Removes container and volume unless force keeps running resources.",
        use_cases: &["Audit site teardown"],
        register_example: "hook!(host, PluginEvent::SiteDestroyed, on_destroyed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::ProxyRegistered,
        name: "proxy.registered",
        summary: "Traefik routing labels were applied to a site.",
        when: "Site provision with proxy.enabled.",
        payload: "JSON: `{ site_id, domains, provider }`",
        cancelable: false,
        details: "Requires Traefik on the configured proxy.network.",
        use_cases: &["Verify domain routing setup"],
        register_example: "hook!(host, PluginEvent::ProxyRegistered, on_proxy);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::SslProvisioned,
        name: "ssl.provisioned",
        summary: "TLS cert resolver labels were applied for a site.",
        when: "Site provision with proxy.tls_enabled.",
        payload: "JSON: `{ site_id, domains, cert_resolver }`",
        cancelable: false,
        details: "Actual certificate issuance is handled by Traefik/Let's Encrypt.",
        use_cases: &["Track SSL automation"],
        register_example: "hook!(host, PluginEvent::SslProvisioned, on_ssl);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::VolumeCreated,
        name: "volume.created",
        summary: "A Docker volume was created for a site.",
        when: "Site provision creates site-vol-{id}.",
        payload: "JSON: `{ name }`",
        cancelable: false,
        details: "Persistent site files live in this volume.",
        use_cases: &["Storage auditing"],
        register_example: "hook!(host, PluginEvent::VolumeCreated, on_volume);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::VolumeRemoved,
        name: "volume.removed",
        summary: "A Docker volume was removed.",
        when: "Site destroy removes the site volume.",
        payload: "JSON: `{ name }`",
        cancelable: false,
        details: "Destructive operation.",
        use_cases: &["Audit data deletion"],
        register_example: "hook!(host, PluginEvent::VolumeRemoved, on_volume_removed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::SiteNetworkCreated,
        name: "site.network_created",
        summary: "Isolated site Docker network was created.",
        when: "First provision of a site on this node.",
        payload: "JSON: `{ id }` (site id)",
        cancelable: false,
        details: "Network name format: site-nw-{id}.",
        use_cases: &["Network inventory"],
        register_example: "hook!(host, PluginEvent::SiteNetworkCreated, on_net);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DeployStarted,
        name: "deploy.started",
        summary: "Git deployment started for a site.",
        when: "POST /api/sites/{id}/deploy or webhook deploy.",
        payload: "JSON: `{ site_id, git_ref? }`",
        cancelable: false,
        details: "Clones or pulls repository then syncs into site volume.",
        use_cases: &["Deploy notifications"],
        register_example: "hook!(host, PluginEvent::DeployStarted, on_deploy);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DeployFinished,
        name: "deploy.finished",
        summary: "Git deployment completed successfully.",
        when: "Deploy pipeline succeeded.",
        payload: "JSON: `{ site_id, ok: true }`",
        cancelable: false,
        details: "Container is restarted after file sync.",
        use_cases: &["Mark deploy complete in panel"],
        register_example: "hook!(host, PluginEvent::DeployFinished, on_deploy_done);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DeployFailed,
        name: "deploy.failed",
        summary: "Git deployment failed.",
        when: "Deploy pipeline returned an error.",
        payload: "JSON: `{ site_id, ok: false, error }`",
        cancelable: false,
        details: "Check git and docker availability.",
        use_cases: &["Alert on failed deploys"],
        register_example: "hook!(host, PluginEvent::DeployFailed, on_deploy_fail);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::BackupStarted,
        name: "backup.started",
        summary: "Site backup started.",
        when: "POST /api/sites/{id}/backups.",
        payload: "JSON: `{ site_id }`",
        cancelable: false,
        details: "Container is stopped during backup.",
        use_cases: &["Backup progress UI"],
        register_example: "hook!(host, PluginEvent::BackupStarted, on_backup);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::BackupCompleted,
        name: "backup.completed",
        summary: "Site backup completed.",
        when: "Backup archive written successfully.",
        payload: "JSON: `{ site_id, backup_id }`",
        cancelable: false,
        details: "Archive stored under system.backup_directory.",
        use_cases: &["Backup inventory sync"],
        register_example: "hook!(host, PluginEvent::BackupCompleted, on_backup_done);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::BackupFailed,
        name: "backup.failed",
        summary: "Site backup failed.",
        when: "Backup pipeline error.",
        payload: "JSON: `{ site_id, error }`",
        cancelable: false,
        details: "Container is restarted if it was stopped.",
        use_cases: &["Alert operators"],
        register_example: "hook!(host, PluginEvent::BackupFailed, on_backup_fail);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::BackupRestored,
        name: "backup.restored",
        summary: "Site backup was restored.",
        when: "POST /api/sites/{id}/backups/{backup_id}/restore succeeded.",
        payload: "JSON: `{ site_id, backup_id }`",
        cancelable: false,
        details: "Replaces volume contents from archive.",
        use_cases: &["Audit restore operations"],
        register_example: "hook!(host, PluginEvent::BackupRestored, on_restore);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::WebhookReceived,
        name: "webhook.received",
        summary: "Deploy webhook received for a site.",
        when: "POST /api/sites/{id}/deploy/webhook.",
        payload: "JSON: `{ site_id, source }`",
        cancelable: false,
        details: "Triggers the same deploy pipeline as manual deploy.",
        use_cases: &["GitHub/GitLab push hooks"],
        register_example: "hook!(host, PluginEvent::WebhookReceived, on_webhook);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DatabaseCreated,
        name: "database.created",
        summary: "Database addon provisioned for a site.",
        when: "POST /api/sites/{id}/databases succeeds.",
        payload: "JSON: `{ site_id, database_id, engine, host, port }`",
        cancelable: false,
        details: "Sidecar container on the site network with persisted volume.",
        use_cases: &["Panel database inventory", "Connection string sync"],
        register_example: "hook!(host, PluginEvent::DatabaseCreated, on_db);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DatabaseRemoved,
        name: "database.removed",
        summary: "Database addon removed from a site.",
        when: "DELETE /api/sites/{id}/databases/{db_id} succeeds.",
        payload: "JSON: `{ site_id, database_id, engine }`",
        cancelable: false,
        details: "Container and volume are removed when force is set.",
        use_cases: &["Cleanup external DNS or billing hooks"],
        register_example: "hook!(host, PluginEvent::DatabaseRemoved, on_db_removed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::TaskScheduled,
        name: "task.scheduled",
        summary: "Cron, once, or on-demand task registered for a site.",
        when: "POST /api/sites/{id}/tasks succeeds.",
        payload: "JSON: `{ site_id, task_id, kind, action }`",
        cancelable: false,
        details: "Background scheduler polls every 30 seconds.",
        use_cases: &["Panel cron UI sync"],
        register_example: "hook!(host, PluginEvent::TaskScheduled, on_task);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::TaskExecuted,
        name: "task.executed",
        summary: "Scheduled or manual task completed successfully.",
        when: "Scheduler or POST /api/sites/{id}/tasks/{task_id}/run succeeds.",
        payload: "JSON: `{ site_id, task_id, action }`",
        cancelable: false,
        details: "Actions include deploy, backup, restart, and shell.",
        use_cases: &["Audit scheduled jobs"],
        register_example: "hook!(host, PluginEvent::TaskExecuted, on_task_done);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::TaskFailed,
        name: "task.failed",
        summary: "Scheduled or manual task failed.",
        when: "Task execution returns an error.",
        payload: "JSON: `{ site_id, task_id, error }`",
        cancelable: false,
        details: "Failure does not disable the task; cron tasks retry on the next tick.",
        use_cases: &["Alert operators on failed cron jobs"],
        register_example: "hook!(host, PluginEvent::TaskFailed, on_task_fail);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DnsSynced,
        name: "dns.synced",
        summary: "DNS records synced for a site.",
        when: "PUT /api/sites/{id}/dns succeeds.",
        payload: "JSON: `{ site_id, record_count }`",
        cancelable: false,
        details: "Records are stored in cache and written to a zone file under system.data/dns.",
        use_cases: &["External DNS provider sync", "Panel DNS editor"],
        register_example: "hook!(host, PluginEvent::DnsSynced, on_dns);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DeployBlueGreenStarted,
        name: "deploy.blue_green_started",
        summary: "Zero-downtime deploy started for a site.",
        when: "POST /api/sites/{id}/deploy with zero_downtime=true.",
        payload: "JSON: `{ site_id }`",
        cancelable: false,
        details: "Syncs source, starts a new container, then removes the old one.",
        use_cases: &["Production deploy progress UI"],
        register_example: "hook!(host, PluginEvent::DeployBlueGreenStarted, on_bg_start);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DeployBlueGreenFinished,
        name: "deploy.blue_green_finished",
        summary: "Zero-downtime deploy finished for a site.",
        when: "Blue/green container swap completes.",
        payload: "JSON: `{ site_id }`",
        cancelable: false,
        details: "Site cache is updated with the new container ID.",
        use_cases: &["Notify panel that traffic cutover completed"],
        register_example: "hook!(host, PluginEvent::DeployBlueGreenFinished, on_bg_done);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::MailAccountCreated,
        name: "mail.account_created",
        summary: "Mailbox account created for a site.",
        when: "POST /api/sites/{id}/mail/accounts succeeds.",
        payload: "JSON: `{ site_id, account_id, address }`",
        cancelable: false,
        details: "Registered in cache and mail manifest; pushed to mailserver when running.",
        use_cases: &["Panel mailbox inventory"],
        register_example: "hook!(host, PluginEvent::MailAccountCreated, on_mail);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::MailAccountRemoved,
        name: "mail.account_removed",
        summary: "Mailbox account removed from a site.",
        when: "DELETE /api/sites/{id}/mail/accounts/{account_id} succeeds.",
        payload: "JSON: `{ site_id, account_id, address }`",
        cancelable: false,
        details: "Removed from mailserver when the sidecar is running.",
        use_cases: &["Billing and quota sync"],
        register_example: "hook!(host, PluginEvent::MailAccountRemoved, on_mail_removed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::MailSynced,
        name: "mail.synced",
        summary: "Mail stack synced for a site.",
        when: "PUT /api/sites/{id}/mail/sync succeeds.",
        payload: "JSON: `{ site_id, account_count, mail_host }`",
        cancelable: false,
        details: "May provision mailserver and Roundcube sidecars plus recommended DNS records.",
        use_cases: &["External MX/SPF automation"],
        register_example: "hook!(host, PluginEvent::MailSynced, on_mail_sync);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::RoundcubeProvisioned,
        name: "mail.roundcube_provisioned",
        summary: "Roundcube webmail provisioned for a site.",
        when: "Mail sync with provision_roundcube enabled.",
        payload: "JSON: `{ site_id, webmail_host }`",
        cancelable: false,
        details: "Roundcube container is routed via Traefik on webmail.{domain}.",
        use_cases: &["Panel webmail URL sync"],
        register_example: "hook!(host, PluginEvent::RoundcubeProvisioned, on_roundcube);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::FtpAccountCreated,
        name: "ftp.account_created",
        summary: "FTP/SFTP account created for a site.",
        when: "POST /api/sites/{id}/ftp/accounts succeeds.",
        payload: "JSON: `{ site_id, account_id, username, protocol }`",
        cancelable: false,
        details: "Account manifest updated; run ftp sync to apply gateway containers.",
        use_cases: &["Panel file access credentials"],
        register_example: "hook!(host, PluginEvent::FtpAccountCreated, on_ftp);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::FtpAccountRemoved,
        name: "ftp.account_removed",
        summary: "FTP/SFTP account removed from a site.",
        when: "DELETE /api/sites/{id}/ftp/accounts/{account_id} succeeds.",
        payload: "JSON: `{ site_id, account_id, username }`",
        cancelable: false,
        details: "Re-run ftp sync to rebuild gateway containers.",
        use_cases: &["Revoke file access"],
        register_example: "hook!(host, PluginEvent::FtpAccountRemoved, on_ftp_removed);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::FtpGatewaySynced,
        name: "ftp.gateway_synced",
        summary: "FTP/SFTP gateway containers synced for a site.",
        when: "PUT /api/sites/{id}/ftp/sync succeeds.",
        payload: "JSON: `{ site_id, account_count }`",
        cancelable: false,
        details: "SFTP uses atmoz/sftp; FTP uses pure-ftpd mounting the site volume.",
        use_cases: &["Apply credential changes to live gateways"],
        register_example: "hook!(host, PluginEvent::FtpGatewaySynced, on_ftp_sync);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::SiteExecCompleted,
        name: "site.exec_completed",
        summary: "One-shot command executed in a site container.",
        when: "POST /api/sites/{id}/exec succeeds.",
        payload: "JSON: `{ site_id, container_id, exit_output_len }`",
        cancelable: false,
        details: "Non-interactive exec; output returned in the HTTP response body.",
        use_cases: &["Panel terminal shortcuts", "Deploy hooks"],
        register_example: "hook!(host, PluginEvent::SiteExecCompleted, on_site_exec);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::FileManagerChanged,
        name: "file_manager.changed",
        summary: "Site file manager action completed.",
        when: "File list/upload/download/delete/rename/move/copy/archive succeeds.",
        payload: "JSON: `{ site_id, action, path }`",
        cancelable: false,
        details: "Actions: list, upload, download, delete, rename, move, copy, mkdir, archive.",
        use_cases: &["Panel file browser audit", "Backup plugins on upload"],
        register_example: "hook!(host, PluginEvent::FileManagerChanged, on_files);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::SiteStatsCollected,
        name: "site.stats_collected",
        summary: "Per-site resource usage snapshot collected.",
        when: "GET /api/sites/{id}/stats succeeds.",
        payload: "JSON: `{ site_id }`",
        cancelable: false,
        details: "Includes memory, disk usage, container count from Docker and volume du.",
        use_cases: &["Billing meters", "Quota enforcement plugins"],
        register_example: "hook!(host, PluginEvent::SiteStatsCollected, on_site_stats);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::DnsDnssecUpdated,
        name: "dns.dnssec_updated",
        summary: "DNSSEC settings or DS records updated for a site zone.",
        when: "DNS sync includes dnssec_enabled or ds_records.",
        payload: "JSON: `{ site_id, enabled, ds_count }`",
        cancelable: false,
        details: "Zone files written under data/dns/{site_id}/ with dnssec.json metadata.",
        use_cases: &["Registrar DS record sync", "DNSSEC validation alerts"],
        register_example: "hook!(host, PluginEvent::DnsDnssecUpdated, on_dnssec);",
        handler_example: JSON_EVENT_HANDLER,
    },
    EventDoc {
        event: PluginEvent::HostingLimitsUpdated,
        name: "hosting.limits_updated",
        summary: "Workspace memory/CPU/disk/bandwidth limits changed.",
        when: "PUT /api/sites/{id}/hosting/limits succeeds.",
        payload: "JSON: `{ site_id }`",
        cancelable: false,
        details: "Limits stored on the site record; enforce via panel or future cgroup hooks.",
        use_cases: &["Quota billing", "Upgrade workflows"],
        register_example: "hook!(host, PluginEvent::HostingLimitsUpdated, on_limits);",
        handler_example: JSON_EVENT_HANDLER,
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
        handler_example: r##"extern "C" fn on_body(ctx: *const JsonMutateContext) -> i32 {
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
}"##,
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
        handler_example: r##"extern "C" fn on_actions(ctx: *const JsonMutateContext) -> i32 {
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
}"##,
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

pub const ROUTE_HOOK_DOCS: &[HookGuideDoc] = &[HookGuideDoc {
    name: "route.register",
    summary: "Expose new HTTP routes on the daemon router.",
    when: "Registered during init; routes active once HTTP server starts.",
    input: "Request body bytes for matching method + path.",
    output: "Response body + status via write_route_response.",
    pipeline: "Plugin routes merge into the main Axum router alongside core API routes.",
    route_matching: "Exact path per registration. Method must match (GET/POST/PUT/PATCH/DELETE).",
    details: "Paths must start with `/`. Duplicate method+path registrations fail at init. Plugin routes do not require Bearer auth unless your handler or intercept hooks enforce it.",
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

### Versioning

Plugin API **v4** (current) exposes lifecycle events, config mutation, request hooks, plugin routes, and JSON mutation. Future hooks will bump the API version so old plugins keep loading safely when new hook types appear."#;

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
];

pub fn plugin_api_version() -> u32 {
    FEATHERFLY_PLUGIN_API_VERSION
}
