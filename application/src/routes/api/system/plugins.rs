use crate::routes::State;

pub fn router(state: &State) -> axum::Router<State> {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(crate::controllers::system::plugins::get),
        )
        .route(
            "/schema",
            axum::routing::get(crate::controllers::system::plugins::schema),
        )
        .route(
            "/reload",
            axum::routing::post(crate::controllers::system::plugins::reload),
        )
        .with_state(state.clone())
}
