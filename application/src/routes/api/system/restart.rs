use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::system::restart::post))
        .with_state(state.clone())
}
