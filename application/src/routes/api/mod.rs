use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod containers;
mod docker;
mod eggs;
mod metrics;
mod proxy;
mod sites;
mod stats;
mod system;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::site_ws::get))
        .nest(
            "/system",
            system::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/docker",
            docker::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/containers",
            containers::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/stats",
            stats::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/eggs",
            eggs::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/sites",
            sites::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/metrics",
            metrics::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .nest(
            "/proxy",
            proxy::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .with_state(state.clone())
}
