use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::system::plugins::get))
        .routes(routes!(crate::controllers::system::plugins::schema))
        .routes(routes!(crate::controllers::system::plugins::reload))
        .with_state(state.clone())
}
