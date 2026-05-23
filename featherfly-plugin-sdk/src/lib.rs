pub mod metadata;

pub const FEATHERFLY_PLUGIN_API_VERSION: u32 = 3;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginEvent {
    DaemonStarting = 1,
    DaemonStarted = 2,
    DaemonStopping = 3,
    ConfigLoaded = 4,
    PluginLoaded = 5,
}

impl PluginEvent {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DaemonStarting => "daemon.starting",
            Self::DaemonStarted => "daemon.started",
            Self::DaemonStopping => "daemon.stopping",
            Self::ConfigLoaded => "config.loaded",
            Self::PluginLoaded => "plugin.loaded",
        }
    }

    #[must_use]
    pub fn all() -> &'static [metadata::EventDoc] {
        metadata::EVENT_DOCS
    }

    #[must_use]
    pub fn all_names() -> Vec<&'static str> {
        metadata::EVENT_DOCS.iter().map(|doc| doc.name).collect()
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonMutateTarget {
    ResponseBody = 1,
    ResponseActions = 2,
}

impl JsonMutateTarget {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ResponseBody => "json.response",
            Self::ResponseActions => "json.actions",
        }
    }

    #[must_use]
    pub fn all() -> &'static [metadata::JsonHookDoc] {
        metadata::JSON_HOOK_DOCS
    }

    #[must_use]
    pub fn all_names() -> Vec<&'static str> {
        metadata::JSON_HOOK_DOCS
            .iter()
            .map(|doc| doc.name)
            .collect()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EventContext {
    pub event: u32,
    pub payload_ptr: *const u8,
    pub payload_len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JsonMutateContext {
    pub target: u32,
    pub route_ptr: *const u8,
    pub route_len: usize,
    pub method: u32,
    pub json_in_ptr: *const u8,
    pub json_in_len: usize,
    pub json_out_ptr: *mut u8,
    pub json_out_capacity: usize,
    pub json_out_len: *mut usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HookResult {
    pub cancelled: u8,
}

impl HookResult {
    #[must_use]
    pub const fn r#continue() -> Self {
        Self { cancelled: 0 }
    }

    #[must_use]
    pub const fn cancel() -> Self {
        Self { cancelled: 1 }
    }
}

pub type HookCallback = extern "C" fn(*const EventContext) -> HookResult;
pub type JsonMutateCallback = extern "C" fn(*const JsonMutateContext) -> i32;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HostApi {
    pub api_version: u32,
    pub plugin_name_ptr: *const u8,
    pub plugin_name_len: usize,
    pub hook_state: *mut core::ffi::c_void,
    pub register_hook: Option<
        extern "C" fn(event: u32, callback: HookCallback, user_data: *mut core::ffi::c_void) -> i32,
    >,
    pub register_json_hook: Option<
        extern "C" fn(
            target: u32,
            route_pattern_ptr: *const u8,
            route_pattern_len: usize,
            callback: JsonMutateCallback,
            user_data: *mut core::ffi::c_void,
        ) -> i32,
    >,
    pub log_info: Option<extern "C" fn(message_ptr: *const u8, message_len: usize)>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginDescriptor {
    pub api_version: u32,
    pub name_ptr: *const u8,
    pub name_len: usize,
    pub version_ptr: *const u8,
    pub version_len: usize,
}

pub type PluginInitFn = extern "C" fn(*const HostApi) -> i32;
pub type PluginShutdownFn = extern "C" fn();

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginVTable {
    pub init: Option<PluginInitFn>,
    pub shutdown: Option<PluginShutdownFn>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginEntry {
    pub descriptor: PluginDescriptor,
    pub vtable: PluginVTable,
}

impl PluginEntry {
    #[must_use]
    pub const fn new(
        name: &'static str,
        version: &'static str,
        init: Option<PluginInitFn>,
        shutdown: Option<PluginShutdownFn>,
    ) -> Self {
        Self {
            descriptor: PluginDescriptor {
                api_version: FEATHERFLY_PLUGIN_API_VERSION,
                name_ptr: name.as_ptr(),
                name_len: name.len(),
                version_ptr: version.as_ptr(),
                version_len: version.len(),
            },
            vtable: PluginVTable { init, shutdown },
        }
    }
}

pub const JSON_MUTATE_UNCHANGED: i32 = 1;
pub const JSON_MUTATE_MODIFIED: i32 = 0;

pub const HTTP_METHOD_GET: u32 = 1;
pub const HTTP_METHOD_POST: u32 = 2;
pub const HTTP_METHOD_PUT: u32 = 3;
pub const HTTP_METHOD_PATCH: u32 = 4;
pub const HTTP_METHOD_DELETE: u32 = 5;

pub unsafe fn register_hook(
    host: *const HostApi,
    event: PluginEvent,
    callback: HookCallback,
) -> i32 {
    if host.is_null() {
        return -1;
    }

    let host = unsafe { &*host };
    let Some(register_hook) = host.register_hook else {
        return -2;
    };

    register_hook(event.as_u32(), callback, host.hook_state)
}

pub unsafe fn register_json_hook(
    host: *const HostApi,
    target: JsonMutateTarget,
    route_pattern: &str,
    callback: JsonMutateCallback,
) -> i32 {
    if host.is_null() {
        return -1;
    }

    let host = unsafe { &*host };
    let Some(register_json_hook) = host.register_json_hook else {
        return -2;
    };

    register_json_hook(
        target.as_u32(),
        route_pattern.as_ptr(),
        route_pattern.len(),
        callback,
        host.hook_state,
    )
}

pub unsafe fn log_info(host: *const HostApi, message: &str) {
    if host.is_null() {
        return;
    }

    let host = unsafe { &*host };
    if let Some(log_info) = host.log_info {
        log_info(message.as_ptr(), message.len());
    }
}

pub fn write_json_output(ctx: &JsonMutateContext, json: &[u8]) -> i32 {
    if ctx.json_out_ptr.is_null() || ctx.json_out_len.is_null() {
        return -1;
    }

    if json.len() > ctx.json_out_capacity {
        return -2;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(json.as_ptr(), ctx.json_out_ptr, json.len());
        *ctx.json_out_len = json.len();
    }

    JSON_MUTATE_MODIFIED
}

#[allow(dead_code)]
pub extern "C" fn noop_shutdown() {}

#[macro_export]
macro_rules! declare_plugin {
    (
        name: $name:expr,
        version: $version:expr,
        init: $init:ident $(,)?
    ) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn featherfly_plugin_entry() -> $crate::PluginEntry {
            $crate::PluginEntry::new($name, $version, Some($init), Some($crate::noop_shutdown))
        }
    };
    (
        name: $name:expr,
        version: $version:expr,
        init: $init:ident,
        shutdown: $shutdown:ident $(,)?
    ) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn featherfly_plugin_entry() -> $crate::PluginEntry {
            $crate::PluginEntry::new($name, $version, Some($init), Some($shutdown))
        }
    };
}

#[macro_export]
macro_rules! hook {
    ($host:expr, $event:expr, $callback:ident) => {
        let status = unsafe { $crate::register_hook($host, $event, $callback) };
        if status != 0 {
            return status;
        }
    };
}

#[macro_export]
macro_rules! hook_json {
    ($host:expr, $target:expr, $route:expr, $callback:ident) => {
        let status = unsafe { $crate::register_json_hook($host, $target, $route, $callback) };
        if status != 0 {
            return status;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_entry_carries_api_version() {
        extern "C" fn init(_host: *const HostApi) -> i32 {
            0
        }

        let entry = PluginEntry::new("demo", "0.1.0", Some(init), None);
        assert_eq!(entry.descriptor.api_version, FEATHERFLY_PLUGIN_API_VERSION);
    }
}
