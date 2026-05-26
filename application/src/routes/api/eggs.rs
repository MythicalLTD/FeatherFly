use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::eggs::list))
        .routes(routes!(crate::controllers::eggs::get))
        .with_state(state.clone())
}
