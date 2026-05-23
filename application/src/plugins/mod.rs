mod events;
pub mod middleware;

use anyhow::Context;
use events::{EmitOutcome, EventBus, SharedEventBus};
use featherfly_plugin_sdk::{
    HostApi, PluginEntry, PluginEvent, FEATHERFLY_PLUGIN_API_VERSION,
};
use libloading::Library;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use utoipa::ToSchema;

pub const PLUGIN_ENTRY_SYMBOL: &[u8] = b"featherfly_plugin_entry";

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct PluginSummary {
    pub name: String,
    pub version: String,
    pub path: String,
    pub api_version: u32,
    pub hooks: usize,
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
}

impl PluginRegistry {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            plugins: Vec::new(),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
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
    pub fn hook_count(&self) -> usize {
        self.event_bus.lock().map(|bus| bus.hook_count()).unwrap_or(0)
    }

    pub fn emit(&self, event: PluginEvent, payload: &[u8]) -> EmitOutcome {
        self.event_bus
            .lock()
            .map(|bus| bus.emit(event, payload))
            .unwrap_or_default()
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

pub fn load_directory(
    directory: &Path,
    enabled: bool,
    config_payload: &[u8],
) -> Result<PluginRegistry, anyhow::Error> {
    let event_bus = Arc::new(Mutex::new(EventBus::new()));

    if !enabled {
        tracing::info!("plugin loading disabled in config");
        return Ok(PluginRegistry {
            plugins: Vec::new(),
            event_bus,
        });
    }

    {
        let bus = event_bus.lock().expect("plugin event bus poisoned");
        bus.emit(PluginEvent::ConfigLoaded, config_payload);
    }

    if !directory.exists() {
        tracing::info!(
            path = %directory.display(),
            "plugins directory does not exist yet; skipping plugin load"
        );
        return Ok(PluginRegistry {
            plugins: Vec::new(),
            event_bus,
        });
    }

    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
        .with_context(|| format!("failed to read plugins directory {}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_plugin_file(path))
        .collect();
    paths.sort();

    let mut plugins = Vec::new();
    for path in paths {
        match load_plugin(&path, Arc::clone(&event_bus)) {
            Ok(plugin) => {
                tracing::info!(
                    name = %plugin.summary.name,
                    version = %plugin.summary.version,
                    hooks = plugin.summary.hooks,
                    path = %plugin.summary.path,
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
            }
        }
    }

    let hook_count = event_bus
        .lock()
        .expect("plugin event bus poisoned")
        .hook_count();
    tracing::info!(count = plugins.len(), hooks = hook_count, "plugin load complete");

    Ok(PluginRegistry {
        plugins,
        event_bus,
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

extern "C" fn log_info_trampoline(message_ptr: *const u8, message_len: usize) {
    if message_ptr.is_null() || message_len == 0 {
        return;
    }

    let message = unsafe { std::slice::from_raw_parts(message_ptr, message_len) };
    tracing::info!(target: "plugin", "{}", String::from_utf8_lossy(message));
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
