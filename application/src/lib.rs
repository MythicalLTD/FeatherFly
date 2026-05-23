pub mod actions;
pub mod api_spec;
pub mod auth;
pub mod commands;
pub mod config;
pub mod daemon_control;
pub mod logging;
pub mod plugins;
pub mod probe_guard;
pub mod response;
pub mod routes;
pub mod update;

#[cfg(not(unix))]
compile_error!("FeatherFly only supports Unix-like systems");

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_COMMIT: &str = env!("CARGO_GIT_COMMIT");
pub const GIT_BRANCH: &str = env!("CARGO_GIT_BRANCH");
pub const TARGET: &str = env!("CARGO_TARGET");

pub const DEFAULT_CONFIG_PATH: &str = "/etc/featherfly/config.yml";
pub const GITHUB_REPO: &str = "MythicalLTD/featherfly";
pub const GITHUB_REPOSITORY: &str = "https://github.com/MythicalLTD/featherfly";
pub const PROJECT_WEBSITE: &str = "https://featherpanel.com";
pub const PROJECT_LICENSE: &str = "MIT";
pub const DOCS_WEBSITE: &str = "https://mythicalltd.github.io/featherfly";

pub fn full_version() -> String {
    if GIT_BRANCH == "unknown" {
        VERSION.to_string()
    } else {
        format!("{VERSION}:{GIT_COMMIT}@{GIT_BRANCH}")
    }
}

pub mod daemon;
