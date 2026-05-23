use featherfly_plugin_sdk::{EventContext, HookCallback, JsonMutateCallback, PluginEvent};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct EventBus {
    hooks: HashMap<u32, Vec<RegisteredHook>>,
    json_hooks: Vec<RegisteredJsonHook>,
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

#[derive(Debug, Clone, Copy, Default)]
pub struct EmitOutcome {
    pub cancelled: bool,
    pub handlers_run: usize,
}

impl EventBus {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        plugin_name: impl Into<String>,
        event: u32,
        callback: HookCallback,
    ) {
        self.hooks
            .entry(event)
            .or_default()
            .push(RegisteredHook {
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
        self.hooks.values().map(Vec::len).sum::<usize>() + self.json_hooks.len()
    }
}

pub type SharedEventBus = std::sync::Arc<Mutex<EventBus>>;

fn route_matches(pattern: &str, route: &str) -> bool {
    pattern == "*" || route.starts_with(pattern)
}

fn method_to_code(method: &str) -> u32 {
    match method {
        "GET" => featherfly_plugin_sdk::HTTP_METHOD_GET,
        "POST" => featherfly_plugin_sdk::HTTP_METHOD_POST,
        "PUT" => featherfly_plugin_sdk::HTTP_METHOD_PUT,
        "PATCH" => featherfly_plugin_sdk::HTTP_METHOD_PATCH,
        "DELETE" => featherfly_plugin_sdk::HTTP_METHOD_DELETE,
        _ => 0,
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
