use crate::routes::State;

pub fn router(state: &State) -> axum::Router<State> {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(crate::controllers::system::overview::get),
        )
        .with_state(state.clone())
}
