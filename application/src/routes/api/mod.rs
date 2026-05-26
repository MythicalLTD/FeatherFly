use super::State;

mod system;

pub fn router(state: &State) -> axum::Router<State> {
    axum::Router::new()
        .nest(
            "/system",
            system::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .with_state(state.clone())
}
