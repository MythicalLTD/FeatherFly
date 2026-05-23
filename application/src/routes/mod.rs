use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

pub mod api;

mod health;

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
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct ApiError<'a> {
    pub error: &'a str,
}

impl<'a> ApiError<'a> {
    #[inline]
    pub fn new(error: &'a str) -> Self {
        Self { error }
    }
}

pub type State = Arc<AppState>;
pub type GetState = axum::extract::State<State>;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(health::route))
        .nest("/api", api::router(state))
        .with_state(state.clone())
}
