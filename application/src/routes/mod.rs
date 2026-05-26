use serde::Serialize;
use std::{sync::Arc, time::Instant};

pub mod api;

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AppContainerType {
    Official,
    Unknown,
    None,
}

pub struct AppState {
    pub start_time: Instant,
    pub container_type: AppContainerType,
    pub version: String,
    pub config: Arc<crate::config::Config>,
    pub plugins: crate::plugins::PluginRegistry,
    pub probe_guard: crate::middlewares::probe::ProbeGuard,
}

pub type State = Arc<AppState>;
pub type GetState = axum::extract::State<State>;

pub fn router(state: &State) -> axum::Router<State> {
    axum::Router::new()
        .route(
            "/health",
            axum::routing::get(crate::controllers::health::get),
        )
        .route(
            "/live",
            axum::routing::get(crate::controllers::health::live),
        )
        .route(
            "/ready",
            axum::routing::get(crate::controllers::health::ready),
        )
        .nest("/api", api::router(state))
        .with_state(state.clone())
}
