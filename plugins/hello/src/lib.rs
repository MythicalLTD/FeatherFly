use featherfly_plugin_sdk::{
    declare_plugin, hook, hook_config, hook_json, hook_request, log_info, route,
    write_json_output, write_request_response, write_route_response, write_yaml_output,
    ConfigMutateContext, EventContext, HookResult, HostApi, JsonMutateContext, JsonMutateTarget,
    PluginEvent, RequestHookContext, RequestHookPhase, RouteHandlerContext, CONFIG_MUTATE_UNCHANGED,
    HTTP_METHOD_GET, JSON_MUTATE_UNCHANGED, REQUEST_CONTINUE,
};

extern "C" fn init(host: *const HostApi) -> i32 {
    hook!(host, PluginEvent::DaemonStarted, on_daemon_started);
    hook!(host, PluginEvent::DaemonStopping, on_daemon_stopping);
    hook_config!(host, on_config_mutate);
    hook_request!(
        host,
        RequestHookPhase::Intercept,
        "/plugins/hello",
        on_hello_intercept
    );
    hook_request!(
        host,
        RequestHookPhase::Middleware,
        "/api/*",
        on_api_middleware
    );
    hook_json!(
        host,
        JsonMutateTarget::ResponseBody,
        "/api/system",
        on_system_body
    );
    hook_json!(
        host,
        JsonMutateTarget::ResponseActions,
        "/api/system",
        on_system_actions
    );
    route!(
        host,
        HTTP_METHOD_GET,
        "/plugins/hello",
        on_hello_route
    );
    unsafe {
        log_info(host, "hello plugin registered v4 hooks");
    }
    0
}

extern "C" fn on_daemon_started(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

extern "C" fn on_daemon_stopping(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

extern "C" fn on_config_mutate(ctx: *const ConfigMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.yaml_in_ptr, ctx.yaml_in_len) };
    let marker = b"# hello_plugin: active\n";

    if input.starts_with(marker) {
        return CONFIG_MUTATE_UNCHANGED;
    }

    let mut out = Vec::with_capacity(marker.len() + input.len());
    out.extend_from_slice(marker);
    out.extend_from_slice(input);

    write_yaml_output(ctx, &out)
}

extern "C" fn on_hello_intercept(ctx: *const RequestHookContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let headers =
        unsafe { std::slice::from_raw_parts(ctx.headers_json_ptr, ctx.headers_json_len) };

    let Ok(value) = serde_json::from_slice::<serde_json::Value>(headers) else {
        return REQUEST_CONTINUE;
    };

    if value.get("x-hello").and_then(|v| v.as_str()) == Some("world") {
        return REQUEST_CONTINUE;
    }

    let body = br#"{"error":"missing X-Hello header","hint":"send X-Hello: world"}"#;
    write_request_response(ctx, 401, body)
}

extern "C" fn on_api_middleware(_ctx: *const RequestHookContext) -> i32 {
    REQUEST_CONTINUE
}

extern "C" fn on_hello_route(ctx: *const RouteHandlerContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let body = br#"{"plugin":"hello","api_version":4,"route":"GET /plugins/hello"}"#;
    write_route_response(ctx, 200, body)
}

extern "C" fn on_system_body(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };

    if let Some(map) = value.as_object_mut() {
        map.insert("hello_plugin".into(), serde_json::Value::Bool(true));
    }

    let Ok(out) = serde_json::to_vec(&value) else {
        return JSON_MUTATE_UNCHANGED;
    };

    write_json_output(ctx, &out)
}

extern "C" fn on_system_actions(ctx: *const JsonMutateContext) -> i32 {
    let ctx = unsafe { &*ctx };
    let input = unsafe { std::slice::from_raw_parts(ctx.json_in_ptr, ctx.json_in_len) };
    let Ok(mut actions) = serde_json::from_slice::<Vec<serde_json::Value>>(input) else {
        return JSON_MUTATE_UNCHANGED;
    };

    actions.retain(|action| action.get("id").and_then(|id| id.as_str()) != Some("diagnostics"));
    actions.push(serde_json::json!({
        "id": "hello_docs",
        "label": "Hello plugin docs",
        "step": "GET /docs/plugins"
    }));
    actions.push(serde_json::json!({
        "id": "hello_route",
        "label": "Hello plugin route",
        "step": "GET /plugins/hello (header X-Hello: world)"
    }));

    let Ok(out) = serde_json::to_vec(&actions) else {
        return JSON_MUTATE_UNCHANGED;
    };

    write_json_output(ctx, &out)
}

declare_plugin! {
    name: "hello",
    version: "0.2.0",
    init: init,
}
