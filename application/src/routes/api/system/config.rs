use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::system::config::get))
        .routes(routes!(crate::controllers::system::config::put))
        .with_state(state.clone())
}
