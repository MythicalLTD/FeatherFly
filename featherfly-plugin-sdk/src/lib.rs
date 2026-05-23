pub mod metadata;

pub const FEATHERFLY_PLUGIN_API_VERSION: u32 = 5;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginEvent {
    DaemonStarting = 1,
    DaemonStarted = 2,
    DaemonStopping = 3,
    ConfigLoaded = 4,
    PluginLoaded = 5,
    RequestReceived = 6,
    RequestCompleted = 7,
    RequestNotFound = 8,
    RequestBlocked = 9,
    ProbeClientBlocked = 10,
    AuthFailed = 11,
    ConfigApplied = 12,
    ConfigExported = 13,
    RestartScheduled = 14,
    PluginReloadRequested = 15,
    UpdateChecked = 16,
    UpdateApplied = 17,
    UpdateApplyFailed = 18,
    UpgradeStarted = 19,
    UpgradeCompleted = 20,
    UpgradeFailed = 21,
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
            Self::RequestReceived => "request.received",
            Self::RequestCompleted => "request.completed",
            Self::RequestNotFound => "request.not_found",
            Self::RequestBlocked => "request.blocked",
            Self::ProbeClientBlocked => "probe.client_blocked",
            Self::AuthFailed => "auth.failed",
            Self::ConfigApplied => "config.applied",
            Self::ConfigExported => "config.exported",
            Self::RestartScheduled => "restart.scheduled",
            Self::PluginReloadRequested => "plugins.reload_requested",
            Self::UpdateChecked => "update.checked",
            Self::UpdateApplied => "update.applied",
            Self::UpdateApplyFailed => "update.apply_failed",
            Self::UpgradeStarted => "upgrade.started",
            Self::UpgradeCompleted => "upgrade.completed",
            Self::UpgradeFailed => "upgrade.failed",
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

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestHookPhase {
    Intercept = 1,
    Middleware = 2,
}

impl RequestHookPhase {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Intercept => "request.intercept",
            Self::Middleware => "middleware.inject",
        }
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
pub struct ConfigMutateContext {
    pub yaml_in_ptr: *const u8,
    pub yaml_in_len: usize,
    pub yaml_out_ptr: *mut u8,
    pub yaml_out_capacity: usize,
    pub yaml_out_len: *mut usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RequestHookContext {
    pub phase: u32,
    pub method: u32,
    pub path_ptr: *const u8,
    pub path_len: usize,
    pub headers_json_ptr: *const u8,
    pub headers_json_len: usize,
    pub body_in_ptr: *const u8,
    pub body_in_len: usize,
    pub status_code: *mut u16,
    pub response_body_ptr: *mut u8,
    pub response_body_capacity: usize,
    pub response_body_len: *mut usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RouteHandlerContext {
    pub method: u32,
    pub path_ptr: *const u8,
    pub path_len: usize,
    pub body_in_ptr: *const u8,
    pub body_in_len: usize,
    pub body_out_ptr: *mut u8,
    pub body_out_capacity: usize,
    pub body_out_len: *mut usize,
    pub status_code: *mut u16,
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
pub type ConfigMutateCallback = extern "C" fn(*const ConfigMutateContext) -> i32;
pub type RequestHookCallback = extern "C" fn(*const RequestHookContext) -> i32;
pub type RouteHandlerCallback = extern "C" fn(*const RouteHandlerContext) -> i32;

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
    pub register_config_hook: Option<
        extern "C" fn(callback: ConfigMutateCallback, user_data: *mut core::ffi::c_void) -> i32,
    >,
    pub register_request_hook: Option<
        extern "C" fn(
            phase: u32,
            route_pattern_ptr: *const u8,
            route_pattern_len: usize,
            callback: RequestHookCallback,
            user_data: *mut core::ffi::c_void,
        ) -> i32,
    >,
    pub register_route: Option<
        extern "C" fn(
            method: u32,
            path_ptr: *const u8,
            path_len: usize,
            callback: RouteHandlerCallback,
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

pub const CONFIG_MUTATE_UNCHANGED: i32 = 1;
pub const CONFIG_MUTATE_MODIFIED: i32 = 0;

pub const REQUEST_CONTINUE: i32 = 0;
pub const REQUEST_RESPOND: i32 = 1;

pub const ROUTE_OK: i32 = 0;
pub const ROUTE_ERROR: i32 = -1;

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

pub unsafe fn register_config_hook(host: *const HostApi, callback: ConfigMutateCallback) -> i32 {
    if host.is_null() {
        return -1;
    }

    let host = unsafe { &*host };
    let Some(register_config_hook) = host.register_config_hook else {
        return -2;
    };

    register_config_hook(callback, host.hook_state)
}

pub unsafe fn register_request_hook(
    host: *const HostApi,
    phase: RequestHookPhase,
    route_pattern: &str,
    callback: RequestHookCallback,
) -> i32 {
    if host.is_null() {
        return -1;
    }

    let host = unsafe { &*host };
    let Some(register_request_hook) = host.register_request_hook else {
        return -2;
    };

    register_request_hook(
        phase.as_u32(),
        route_pattern.as_ptr(),
        route_pattern.len(),
        callback,
        host.hook_state,
    )
}

pub unsafe fn register_route(
    host: *const HostApi,
    method: u32,
    path: &str,
    callback: RouteHandlerCallback,
) -> i32 {
    if host.is_null() {
        return -1;
    }

    let host = unsafe { &*host };
    let Some(register_route) = host.register_route else {
        return -2;
    };

    register_route(method, path.as_ptr(), path.len(), callback, host.hook_state)
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
    write_bytes(
        ctx.json_out_ptr,
        ctx.json_out_capacity,
        ctx.json_out_len,
        json,
    )
    .map(|()| JSON_MUTATE_MODIFIED)
    .unwrap_or(-2)
}

pub fn write_yaml_output(ctx: &ConfigMutateContext, yaml: &[u8]) -> i32 {
    write_bytes(
        ctx.yaml_out_ptr,
        ctx.yaml_out_capacity,
        ctx.yaml_out_len,
        yaml,
    )
    .map(|()| CONFIG_MUTATE_MODIFIED)
    .unwrap_or(-2)
}

pub fn write_request_response(ctx: &RequestHookContext, status: u16, body: &[u8]) -> i32 {
    if ctx.status_code.is_null() {
        return -1;
    }

    unsafe {
        *ctx.status_code = status;
    }

    write_bytes(
        ctx.response_body_ptr,
        ctx.response_body_capacity,
        ctx.response_body_len,
        body,
    )
    .map(|()| REQUEST_RESPOND)
    .unwrap_or(-2)
}

pub fn write_route_response(ctx: &RouteHandlerContext, status: u16, body: &[u8]) -> i32 {
    if ctx.status_code.is_null() {
        return -1;
    }

    unsafe {
        *ctx.status_code = status;
    }

    write_bytes(
        ctx.body_out_ptr,
        ctx.body_out_capacity,
        ctx.body_out_len,
        body,
    )
    .map(|()| ROUTE_OK)
    .unwrap_or(-2)
}

fn write_bytes(
    out_ptr: *mut u8,
    capacity: usize,
    out_len_ptr: *mut usize,
    data: &[u8],
) -> Result<(), ()> {
    if out_ptr.is_null() || out_len_ptr.is_null() {
        return Err(());
    }

    if data.len() > capacity {
        return Err(());
    }

    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), out_ptr, data.len());
        *out_len_ptr = data.len();
    }

    Ok(())
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

#[macro_export]
macro_rules! hook_config {
    ($host:expr, $callback:ident) => {
        let status = unsafe { $crate::register_config_hook($host, $callback) };
        if status != 0 {
            return status;
        }
    };
}

#[macro_export]
macro_rules! hook_request {
    ($host:expr, $phase:expr, $route:expr, $callback:ident) => {
        let status = unsafe { $crate::register_request_hook($host, $phase, $route, $callback) };
        if status != 0 {
            return status;
        }
    };
}

#[macro_export]
macro_rules! route {
    ($host:expr, $method:expr, $path:expr, $callback:ident) => {
        let status = unsafe { $crate::register_route($host, $method, $path, $callback) };
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
