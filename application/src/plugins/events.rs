use featherfly_plugin_sdk::{
    CONFIG_MUTATE_MODIFIED, ConfigMutateCallback, EventContext, HookCallback, JsonMutateCallback,
    PluginEvent, REQUEST_RESPOND, RequestHookCallback, RequestHookPhase, RouteHandlerCallback,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct EventBus {
    hooks: HashMap<u32, Vec<RegisteredHook>>,
    json_hooks: Vec<RegisteredJsonHook>,
    config_hooks: Vec<RegisteredConfigHook>,
    request_hooks: Vec<RegisteredRequestHook>,
    routes: Vec<RegisteredRoute>,
}

#[derive(Debug, Clone)]
struct RegisteredHook {
    plugin: String,
    callback: HookCallback,
}

#[derive(Debug, Clone)]
struct RegisteredJsonHook {
    _plugin: String,
    target: u32,
    route_pattern: String,
    callback: JsonMutateCallback,
}

#[derive(Debug, Clone)]
struct RegisteredConfigHook {
    _plugin: String,
    callback: ConfigMutateCallback,
}

#[derive(Debug, Clone)]
struct RegisteredRequestHook {
    _plugin: String,
    phase: u32,
    route_pattern: String,
    callback: RequestHookCallback,
}

#[derive(Debug, Clone)]
pub struct RegisteredRoute {
    pub plugin: String,
    pub method: u32,
    pub path: String,
    pub callback: RouteHandlerCallback,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EmitOutcome {
    pub cancelled: bool,
    pub handlers_run: usize,
}

#[derive(Debug, Clone)]
pub struct RequestHookOutcome {
    pub respond: bool,
    pub status: u16,
    pub body: Vec<u8>,
}

impl EventBus {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin_name: impl Into<String>, event: u32, callback: HookCallback) {
        self.hooks.entry(event).or_default().push(RegisteredHook {
            plugin: plugin_name.into(),
            callback,
        });
    }

    pub fn register_json_hook(
        &mut self,
        plugin_name: impl Into<String>,
        target: u32,
        route_pattern: impl Into<String>,
        callback: JsonMutateCallback,
    ) {
        self.json_hooks.push(RegisteredJsonHook {
            _plugin: plugin_name.into(),
            target,
            route_pattern: route_pattern.into(),
            callback,
        });
    }

    pub fn register_config_hook(
        &mut self,
        plugin_name: impl Into<String>,
        callback: ConfigMutateCallback,
    ) {
        self.config_hooks.push(RegisteredConfigHook {
            _plugin: plugin_name.into(),
            callback,
        });
    }

    pub fn register_request_hook(
        &mut self,
        plugin_name: impl Into<String>,
        phase: u32,
        route_pattern: impl Into<String>,
        callback: RequestHookCallback,
    ) {
        self.request_hooks.push(RegisteredRequestHook {
            _plugin: plugin_name.into(),
            phase,
            route_pattern: route_pattern.into(),
            callback,
        });
    }

    pub fn register_route(
        &mut self,
        plugin_name: impl Into<String>,
        method: u32,
        path: impl Into<String>,
        callback: RouteHandlerCallback,
    ) -> Result<(), String> {
        let path = path.into();
        if !path.starts_with('/') {
            return Err(format!("plugin route must be absolute: {path}"));
        }

        if self
            .routes
            .iter()
            .any(|route| route.method == method && route.path == path)
        {
            return Err(format!("duplicate plugin route: {method} {path}"));
        }

        self.routes.push(RegisteredRoute {
            plugin: plugin_name.into(),
            method,
            path,
            callback,
        });
        Ok(())
    }

    pub fn routes(&self) -> &[RegisteredRoute] {
        &self.routes
    }

    pub fn emit(&self, event: PluginEvent, payload: &[u8]) -> EmitOutcome {
        let mut outcome = EmitOutcome::default();
        let Some(hooks) = self.hooks.get(&event.as_u32()) else {
            return outcome;
        };

        let ctx = EventContext {
            event: event.as_u32(),
            payload_ptr: payload.as_ptr(),
            payload_len: payload.len(),
        };

        for hook in hooks {
            outcome.handlers_run += 1;
            let result = (hook.callback)(&ctx);
            if result.cancelled != 0 {
                outcome.cancelled = true;
                tracing::info!(
                    event = event.name(),
                    plugin = %hook.plugin,
                    "event hook cancelled remaining handlers"
                );
                break;
            }
        }

        outcome
    }

    pub fn mutate_config(&self, input: &[u8]) -> Vec<u8> {
        let mut current = input.to_vec();

        for hook in &self.config_hooks {
            let mut output = vec![0_u8; current.len().max(4096).saturating_mul(2)];
            let mut output_len = 0_usize;
            let ctx = featherfly_plugin_sdk::ConfigMutateContext {
                yaml_in_ptr: current.as_ptr(),
                yaml_in_len: current.len(),
                yaml_out_ptr: output.as_mut_ptr(),
                yaml_out_capacity: output.len(),
                yaml_out_len: &mut output_len,
            };

            let status = (hook.callback)(&ctx);
            if status == CONFIG_MUTATE_MODIFIED && output_len > 0 {
                current = output[..output_len].to_vec();
            }
        }

        current
    }

    pub fn run_request_hooks(
        &self,
        phase: RequestHookPhase,
        route: &str,
        method: &str,
        headers_json: &[u8],
        body: &[u8],
    ) -> RequestHookOutcome {
        let method_code = method_to_code(method);

        for hook in &self.request_hooks {
            if hook.phase != phase.as_u32() || !route_matches(&hook.route_pattern, route) {
                continue;
            }

            let mut status_code = 200_u16;
            let mut response = vec![0_u8; body.len().max(4096).saturating_mul(2)];
            let mut response_len = 0_usize;

            let ctx = featherfly_plugin_sdk::RequestHookContext {
                phase: phase.as_u32(),
                method: method_code,
                path_ptr: route.as_ptr(),
                path_len: route.len(),
                headers_json_ptr: headers_json.as_ptr(),
                headers_json_len: headers_json.len(),
                body_in_ptr: body.as_ptr(),
                body_in_len: body.len(),
                status_code: &mut status_code,
                response_body_ptr: response.as_mut_ptr(),
                response_body_capacity: response.len(),
                response_body_len: &mut response_len,
            };

            let status = (hook.callback)(&ctx);
            if status == REQUEST_RESPOND {
                return RequestHookOutcome {
                    respond: true,
                    status: status_code,
                    body: response[..response_len].to_vec(),
                };
            }
        }

        RequestHookOutcome {
            respond: false,
            status: 200,
            body: Vec::new(),
        }
    }

    pub fn mutate_json(
        &self,
        target: featherfly_plugin_sdk::JsonMutateTarget,
        route: &str,
        method: &str,
        input: &[u8],
    ) -> Vec<u8> {
        let mut current = input.to_vec();
        let method_code = method_to_code(method);

        for hook in &self.json_hooks {
            if hook.target != target.as_u32() || !route_matches(&hook.route_pattern, route) {
                continue;
            }

            let mut output = vec![0_u8; current.len().max(4096).saturating_mul(2)];
            let mut output_len = 0_usize;
            let ctx = featherfly_plugin_sdk::JsonMutateContext {
                target: target.as_u32(),
                route_ptr: route.as_ptr(),
                route_len: route.len(),
                method: method_code,
                json_in_ptr: current.as_ptr(),
                json_in_len: current.len(),
                json_out_ptr: output.as_mut_ptr(),
                json_out_capacity: output.len(),
                json_out_len: &mut output_len,
            };

            let status = (hook.callback)(&ctx);
            if status == featherfly_plugin_sdk::JSON_MUTATE_MODIFIED && output_len > 0 {
                current = output[..output_len].to_vec();
            }
        }

        current
    }

    #[must_use]
    pub fn hook_count(&self) -> usize {
        self.hooks.values().map(Vec::len).sum::<usize>()
            + self.json_hooks.len()
            + self.config_hooks.len()
            + self.request_hooks.len()
            + self.routes.len()
    }
}

pub type SharedEventBus = std::sync::Arc<Mutex<EventBus>>;

pub fn route_matches(pattern: &str, route: &str) -> bool {
    pattern == "*" || route.starts_with(pattern)
}

pub fn method_to_code(method: &str) -> u32 {
    match method {
        "GET" => featherfly_plugin_sdk::HTTP_METHOD_GET,
        "POST" => featherfly_plugin_sdk::HTTP_METHOD_POST,
        "PUT" => featherfly_plugin_sdk::HTTP_METHOD_PUT,
        "PATCH" => featherfly_plugin_sdk::HTTP_METHOD_PATCH,
        "DELETE" => featherfly_plugin_sdk::HTTP_METHOD_DELETE,
        _ => 0,
    }
}

pub fn code_to_method(code: u32) -> &'static str {
    match code {
        x if x == featherfly_plugin_sdk::HTTP_METHOD_GET => "GET",
        x if x == featherfly_plugin_sdk::HTTP_METHOD_POST => "POST",
        x if x == featherfly_plugin_sdk::HTTP_METHOD_PUT => "PUT",
        x if x == featherfly_plugin_sdk::HTTP_METHOD_PATCH => "PATCH",
        x if x == featherfly_plugin_sdk::HTTP_METHOD_DELETE => "DELETE",
        _ => "GET",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use featherfly_plugin_sdk::HookResult;

    extern "C" fn cancel_hook(_ctx: *const EventContext) -> HookResult {
        HookResult::cancel()
    }

    extern "C" fn continue_hook(_ctx: *const EventContext) -> HookResult {
        HookResult::r#continue()
    }

    #[test]
    fn cancelled_event_stops_remaining_hooks() {
        let mut bus = EventBus::new();
        bus.register("first", PluginEvent::DaemonStarted.as_u32(), cancel_hook);
        bus.register("second", PluginEvent::DaemonStarted.as_u32(), continue_hook);

        let outcome = bus.emit(PluginEvent::DaemonStarted, b"");
        assert!(outcome.cancelled);
        assert_eq!(outcome.handlers_run, 1);
    }
}
