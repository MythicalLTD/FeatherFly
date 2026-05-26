use crate::routes::State;

mod cloudpanel;
mod overview;
mod plugins;

pub fn router(state: &State) -> axum::Router<State> {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(crate::controllers::system::root::get),
        )
        .nest("/cloudpanel", cloudpanel::router(state))
        .nest("/overview", overview::router(state))
        .nest("/plugins", plugins::router(state))
        .with_state(state.clone())
}
