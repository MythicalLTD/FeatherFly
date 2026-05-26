use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod audit;
mod config;
mod features;
mod overview;
mod plugins;
mod restart;
mod update;
mod upgrade;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::system::root::get))
        .nest("/audit", audit::router(state))
        .nest("/config", config::router(state))
        .nest("/features", features::router(state))
        .nest("/overview", overview::router(state))
        .nest("/plugins", plugins::router(state))
        .nest("/restart", restart::router(state))
        .nest("/update", update::router(state))
        .nest("/upgrade", upgrade::router(state))
        .with_state(state.clone())
}
