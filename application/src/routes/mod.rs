use arc_swap::ArcSwapOption;
use serde::Serialize;
use std::{sync::Arc, time::Instant};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

pub mod api;

#[derive(Debug, ToSchema, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AppContainerType {
    Official,
    Unknown,
    None,
}

pub struct AppState {
    #[allow(dead_code)]
    pub start_time: Instant,
    pub container_type: AppContainerType,
    pub version: String,
    pub config: Arc<crate::config::Config>,
    pub plugins: crate::plugins::PluginRegistry,
    pub probe_guard: crate::middlewares::probe::ProbeGuard,
    pub docker: Option<Arc<crate::docker::DockerManager>>,
    pub panel: ArcSwapOption<crate::websocket::PanelHandle>,
    pub cache: Option<Arc<crate::cache::Cache>>,
}

pub type State = Arc<AppState>;
pub type GetState = axum::extract::State<State>;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(crate::controllers::health::get))
        .nest("/api", api::router(state))
        .with_state(state.clone())
}
