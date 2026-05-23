use featherfly_plugin_sdk::{
    declare_plugin, hook, hook_json, log_info, write_json_output, EventContext, HookResult,
    HostApi, JsonMutateContext, JsonMutateTarget, PluginEvent, JSON_MUTATE_UNCHANGED,
};

extern "C" fn init(host: *const HostApi) -> i32 {
    hook!(host, PluginEvent::DaemonStarted, on_daemon_started);
    hook!(host, PluginEvent::DaemonStopping, on_daemon_stopping);
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
    unsafe {
        log_info(host, "hello plugin registered hooks");
    }
    0
}

extern "C" fn on_daemon_started(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
}

extern "C" fn on_daemon_stopping(_ctx: *const EventContext) -> HookResult {
    HookResult::r#continue()
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

    let Ok(out) = serde_json::to_vec(&actions) else {
        return JSON_MUTATE_UNCHANGED;
    };

    write_json_output(ctx, &out)
}

declare_plugin! {
    name: "hello",
    version: "0.1.0",
    init: init,
}
