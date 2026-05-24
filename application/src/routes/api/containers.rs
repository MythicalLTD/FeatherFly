use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(crate::controllers::containers::list))
        .routes(routes!(crate::controllers::containers::create))
        .routes(routes!(crate::controllers::containers::inspect))
        .routes(routes!(crate::controllers::containers::restart))
        .routes(routes!(crate::controllers::containers::pull))
        .routes(routes!(crate::controllers::containers::logs))
        .routes(routes!(crate::controllers::containers::exec_create))
        .routes(routes!(crate::controllers::containers::container_stats))
        .routes(routes!(crate::controllers::containers::start))
        .routes(routes!(crate::controllers::containers::stop))
        .routes(routes!(crate::controllers::containers::remove))
        .route(
            "/{id}/exec/ws",
            axum::routing::get(crate::controllers::containers::exec_ws),
        )
        .with_state(state.clone())
}
