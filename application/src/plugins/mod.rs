pub mod events;
pub mod middleware;
pub mod request_middleware;
pub mod routes;
pub mod schema;

use anyhow::Context;
use events::{EmitOutcome, EventBus, RegisteredRoute, RequestHookOutcome, SharedEventBus};
use featherfly_plugin_sdk::{
    ConfigMutateCallback, FEATHERFLY_PLUGIN_API_VERSION, HostApi, PluginEntry, PluginEvent,
    RequestHookCallback, RequestHookPhase,
};
use libloading::Library;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use schema::{PluginHookSchema, build_hook_schema};

pub const PLUGIN_ENTRY_SYMBOL: &[u8] = b"featherfly_plugin_entry";

#[derive(Debug, Serialize, Clone)]
pub struct PluginSummary {
    pub name: String,
    pub version: String,
    pub path: String,
    pub api_version: u32,
    pub hooks: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct PluginLoadError {
    pub path: String,
    pub error: String,
}

struct HookRegistration {
    plugin_name: String,
    bus: SharedEventBus,
}

pub struct LoadedPlugin {
    summary: PluginSummary,
    shutdown: Option<extern "C" fn()>,
    _hook_state: Box<HookRegistration>,
    _library: Library,
}

pub struct PluginRegistry {
    plugins: Vec<LoadedPlugin>,
    event_bus: SharedEventBus,
    load_errors: Vec<PluginLoadError>,
}

impl PluginRegistry {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            plugins: Vec::new(),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
            load_errors: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_load_error(path: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            plugins: Vec::new(),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
            load_errors: vec![PluginLoadError {
                path: path.into(),
                error: error.into(),
            }],
        }
    }

    #[must_use]
    pub fn summaries(&self) -> Vec<PluginSummary> {
        self.plugins
            .iter()
            .map(|plugin| plugin.summary.clone())
            .collect()
    }

    #[must_use]
    pub fn load_errors(&self) -> Vec<PluginLoadError> {
        self.load_errors.clone()
    }

    #[must_use]
    pub fn hook_count(&self) -> usize {
        self.event_bus
            .lock()
            .map(|bus| bus.hook_count())
            .unwrap_or(0)
    }

    #[must_use]
    pub fn routes(&self) -> Vec<RegisteredRoute> {
        self.event_bus
            .lock()
            .map(|bus| bus.routes().to_vec())
            .unwrap_or_default()
    }

    pub fn has_event_hooks(&self, event: PluginEvent) -> bool {
        self.event_bus
            .lock()
            .map(|bus| bus.has_event_hooks(event))
            .unwrap_or(false)
    }

    pub fn emit(&self, event: PluginEvent, payload: &[u8]) -> EmitOutcome {
        self.event_bus
            .lock()
            .map(|bus| bus.emit(event, payload))
            .unwrap_or_default()
    }

    pub fn mutate_config(&self, input: &[u8]) -> Vec<u8> {
        self.event_bus
            .lock()
            .map(|bus| bus.mutate_config(input))
            .unwrap_or_else(|_| input.to_vec())
    }

    pub fn run_request_hooks(
        &self,
        phase: RequestHookPhase,
        route: &str,
        method: &str,
        headers_json: &[u8],
        body: &[u8],
    ) -> RequestHookOutcome {
        self.event_bus
            .lock()
            .map(|bus| bus.run_request_hooks(phase, route, method, headers_json, body))
            .unwrap_or_else(|_| events::RequestHookOutcome {
                respond: false,
                status: 200,
                body: Vec::new(),
            })
    }

    pub fn config_hook_count(&self) -> usize {
        self.event_bus
            .lock()
            .map(|bus| bus.config_hook_count())
            .unwrap_or(0)
    }

    pub fn log_startup_summary(&self) {
        let summaries = self.summaries();
        if summaries.is_empty() {
            tracing::debug!("no plugins loaded");
            return;
        }

        for plugin in &summaries {
            tracing::debug!(
                name = %plugin.name,
                version = %plugin.version,
                hooks = plugin.hooks,
                path = %plugin.path,
                "plugin ready"
            );
        }

        for route in self.routes() {
            tracing::debug!(
                plugin = %route.plugin,
                method = %events::code_to_method(route.method),
                path = %route.path,
                "plugin route registered"
            );
        }
    }

    pub fn shutdown_all(&self) {
        self.emit(PluginEvent::DaemonStopping, b"");
        for plugin in &self.plugins {
            if let Some(shutdown) = plugin.shutdown {
                shutdown();
            }
        }
    }

    pub fn mutate_response_body(&self, route: &str, method: &str, input: &[u8]) -> Vec<u8> {
        self.event_bus
            .lock()
            .map(|bus| {
                bus.mutate_json(
                    featherfly_plugin_sdk::JsonMutateTarget::ResponseBody,
                    route,
                    method,
                    input,
                )
            })
            .unwrap_or_else(|_| input.to_vec())
    }

    pub fn mutate_response_actions(&self, route: &str, method: &str, input: &[u8]) -> Vec<u8> {
        self.event_bus
            .lock()
            .map(|bus| {
                bus.mutate_json(
                    featherfly_plugin_sdk::JsonMutateTarget::ResponseActions,
                    route,
                    method,
                    input,
                )
            })
            .unwrap_or_else(|_| input.to_vec())
    }
}

pub fn load_directory(directory: &Path, enabled: bool) -> Result<PluginRegistry, anyhow::Error> {
    let event_bus = Arc::new(Mutex::new(EventBus::new()));

    if !enabled {
        tracing::debug!("plugin loading disabled in configuration");
        return Ok(PluginRegistry {
            plugins: Vec::new(),
            event_bus,
            load_errors: Vec::new(),
        });
    }

    if !directory.exists() {
        tracing::debug!(
            path = %directory.display(),
            "plugins directory does not exist yet"
        );
        return Ok(PluginRegistry {
            plugins: Vec::new(),
            event_bus,
            load_errors: Vec::new(),
        });
    }

    tracing::debug!(path = %directory.display(), "scanning plugins directory");

    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
        .with_context(|| format!("failed to read plugins directory {}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_plugin_file(path))
        .collect();
    paths.sort();

    if paths.is_empty() {
        tracing::debug!(
            path = %directory.display(),
            "no .so plugin libraries found"
        );
        return Ok(PluginRegistry {
            plugins: Vec::new(),
            event_bus,
            load_errors: Vec::new(),
        });
    }

    tracing::debug!(
        path = %directory.display(),
        count = paths.len(),
        "found plugin libraries"
    );

    let mut plugins = Vec::new();
    let mut load_errors = Vec::new();
    for path in &paths {
        tracing::debug!(path = %path.display(), "loading plugin");
        match load_plugin(path, Arc::clone(&event_bus)) {
            Ok(plugin) => {
                tracing::debug!(
                    name = %plugin.summary.name,
                    version = %plugin.summary.version,
                    hooks = plugin.summary.hooks,
                    api_version = plugin.summary.api_version,
                    "loaded plugin"
                );
                let payload = plugin.summary.name.as_bytes();
                event_bus
                    .lock()
                    .expect("plugin event bus poisoned")
                    .emit(PluginEvent::PluginLoaded, payload);
                plugins.push(plugin);
            }
            Err(err) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %err,
                    "failed to load plugin"
                );
                load_errors.push(PluginLoadError {
                    path: path.display().to_string(),
                    error: err.to_string(),
                });
            }
        }
    }

    let hook_count = event_bus
        .lock()
        .expect("plugin event bus poisoned")
        .hook_count();
    tracing::debug!(
        count = plugins.len(),
        hooks = hook_count,
        routes = event_bus.lock().map(|b| b.routes().len()).unwrap_or(0),
        "plugin load complete"
    );

    Ok(PluginRegistry {
        plugins,
        event_bus,
        load_errors,
    })
}

fn is_plugin_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "so")
}

fn load_plugin(path: &Path, event_bus: SharedEventBus) -> Result<LoadedPlugin, anyhow::Error> {
    let library = unsafe { Library::new(path) }
        .with_context(|| format!("failed to load shared library {}", path.display()))?;

    let entry = unsafe {
        let entry_fn = library.get::<extern "C" fn() -> PluginEntry>(PLUGIN_ENTRY_SYMBOL)?;
        entry_fn()
    };

    if entry.descriptor.api_version != FEATHERFLY_PLUGIN_API_VERSION {
        anyhow::bail!(
            "plugin API version mismatch: plugin={} daemon={FEATHERFLY_PLUGIN_API_VERSION}",
            entry.descriptor.api_version
        );
    }

    let name = read_bytes(entry.descriptor.name_ptr, entry.descriptor.name_len)
        .context("plugin name is invalid")?;
    let version = read_bytes(entry.descriptor.version_ptr, entry.descriptor.version_len)
        .context("plugin version is invalid")?;

    let hooks_before = event_bus
        .lock()
        .expect("plugin event bus poisoned")
        .hook_count();
    let hook_state = Box::new(HookRegistration {
        plugin_name: name.clone(),
        bus: Arc::clone(&event_bus),
    });
    let host_api = build_host_api(&hook_state);

    if let Some(init) = entry.vtable.init {
        let status = init(&host_api);
        if status != 0 {
            anyhow::bail!("plugin {name} init returned status {status}");
        }
    }

    let hooks = event_bus
        .lock()
        .expect("plugin event bus poisoned")
        .hook_count()
        .saturating_sub(hooks_before);

    Ok(LoadedPlugin {
        summary: PluginSummary {
            name,
            version,
            path: path.display().to_string(),
            api_version: entry.descriptor.api_version,
            hooks,
        },
        shutdown: entry.vtable.shutdown,
        _hook_state: hook_state,
        _library: library,
    })
}

fn build_host_api(hook_state: &HookRegistration) -> HostApi {
    HostApi {
        api_version: FEATHERFLY_PLUGIN_API_VERSION,
        plugin_name_ptr: hook_state.plugin_name.as_ptr(),
        plugin_name_len: hook_state.plugin_name.len(),
        hook_state: (hook_state as *const HookRegistration).cast_mut().cast(),
        register_hook: Some(register_hook_trampoline),
        register_json_hook: Some(register_json_hook_trampoline),
        register_config_hook: Some(register_config_hook_trampoline),
        register_request_hook: Some(register_request_hook_trampoline),
        register_route: Some(register_route_trampoline),
        log_info: Some(log_info_trampoline),
    }
}

extern "C" fn register_hook_trampoline(
    event: u32,
    callback: featherfly_plugin_sdk::HookCallback,
    user_data: *mut core::ffi::c_void,
) -> i32 {
    if user_data.is_null() {
        return -1;
    }

    let registration = unsafe { &*(user_data.cast::<HookRegistration>()) };
    let Ok(mut bus) = registration.bus.lock() else {
        return -2;
    };

    bus.register(registration.plugin_name.clone(), event, callback);
    0
}

extern "C" fn register_json_hook_trampoline(
    target: u32,
    route_pattern_ptr: *const u8,
    route_pattern_len: usize,
    callback: featherfly_plugin_sdk::JsonMutateCallback,
    user_data: *mut core::ffi::c_void,
) -> i32 {
    if user_data.is_null() {
        return -1;
    }

    let registration = unsafe { &*(user_data.cast::<HookRegistration>()) };
    let Ok(route_pattern) = read_bytes(route_pattern_ptr, route_pattern_len) else {
        return -2;
    };
    let Ok(mut bus) = registration.bus.lock() else {
        return -3;
    };

    bus.register_json_hook(
        registration.plugin_name.clone(),
        target,
        route_pattern,
        callback,
    );
    0
}

extern "C" fn register_config_hook_trampoline(
    callback: ConfigMutateCallback,
    user_data: *mut core::ffi::c_void,
) -> i32 {
    if user_data.is_null() {
        return -1;
    }

    let registration = unsafe { &*(user_data.cast::<HookRegistration>()) };
    let Ok(mut bus) = registration.bus.lock() else {
        return -2;
    };

    bus.register_config_hook(registration.plugin_name.clone(), callback);
    0
}

extern "C" fn register_request_hook_trampoline(
    phase: u32,
    route_pattern_ptr: *const u8,
    route_pattern_len: usize,
    callback: RequestHookCallback,
    user_data: *mut core::ffi::c_void,
) -> i32 {
    if user_data.is_null() {
        return -1;
    }

    let registration = unsafe { &*(user_data.cast::<HookRegistration>()) };
    let Ok(route_pattern) = read_bytes(route_pattern_ptr, route_pattern_len) else {
        return -2;
    };
    let Ok(mut bus) = registration.bus.lock() else {
        return -3;
    };

    bus.register_request_hook(
        registration.plugin_name.clone(),
        phase,
        route_pattern,
        callback,
    );
    0
}

extern "C" fn register_route_trampoline(
    method: u32,
    path_ptr: *const u8,
    path_len: usize,
    callback: featherfly_plugin_sdk::RouteHandlerCallback,
    user_data: *mut core::ffi::c_void,
) -> i32 {
    if user_data.is_null() {
        return -1;
    }

    let registration = unsafe { &*(user_data.cast::<HookRegistration>()) };
    let Ok(path) = read_bytes(path_ptr, path_len) else {
        return -2;
    };
    let Ok(mut bus) = registration.bus.lock() else {
        return -3;
    };

    match bus.register_route(registration.plugin_name.clone(), method, path, callback) {
        Ok(()) => 0,
        Err(err) => {
            tracing::warn!(plugin = %registration.plugin_name, error = %err, "failed to register plugin route");
            -4
        }
    }
}

extern "C" fn log_info_trampoline(message_ptr: *const u8, message_len: usize) {
    if message_ptr.is_null() || message_len == 0 {
        return;
    }

    let message = unsafe { std::slice::from_raw_parts(message_ptr, message_len) };
    tracing::debug!(target: "plugin", "{}", String::from_utf8_lossy(message));
}

fn read_bytes(ptr: *const u8, len: usize) -> Result<String, anyhow::Error> {
    if ptr.is_null() || len == 0 {
        anyhow::bail!("string pointer or length is empty");
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

pub fn find_plugin_artifact(plugin_dir: &Path) -> Option<PathBuf> {
    let release_dir = plugin_dir.join("target/release");
    if !release_dir.exists() {
        return None;
    }

    std::fs::read_dir(release_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_file() && is_plugin_file(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_so_extension_only() {
        assert!(is_plugin_file(Path::new("libhello.so")));
        assert!(!is_plugin_file(Path::new("hello.txt")));
    }
}
