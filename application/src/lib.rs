pub mod api_spec;
pub mod backups;
pub mod cache;
pub mod commands;
pub mod config;
pub mod controllers;
pub mod daemon;
pub mod daemon_control;
pub mod databases;
pub mod deployments;
pub mod dns;
pub mod docker;
pub mod eggs;
pub mod feature_status;
pub mod files;
pub mod ftp;
pub mod hosting;
pub mod mail;
pub mod middlewares;
pub mod plugins;
pub mod proxy;
pub mod remote;
pub mod routes;
pub mod scheduler;
pub mod site_websocket;
pub mod sites;
pub mod utils;
pub mod websocket;

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
