use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::download::file))
        .routes(routes!(crate::controllers::download::backup))
        .with_state(state.clone())
}
