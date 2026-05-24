use super::State;
use utoipa_axum::router::OpenApiRouter;

mod containers;
mod docker;
mod metrics;
mod proxy;
mod sites;
mod stats;
mod system;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/system", system::router(state))
        .nest("/docker", docker::router(state))
        .nest("/containers", containers::router(state))
        .nest("/stats", stats::router(state))
        .nest("/sites", sites::router(state))
        .nest("/metrics", metrics::router(state))
        .nest("/proxy", proxy::router(state))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middlewares::auth::middleware,
        ))
        .with_state(state.clone())
}
