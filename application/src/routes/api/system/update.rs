use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::system::update::get))
        .routes(routes!(crate::controllers::system::update::apply))
        .with_state(state.clone())
}
