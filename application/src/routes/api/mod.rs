use super::State;
use utoipa_axum::router::OpenApiRouter;

mod system;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest(
            "/system",
            system::router(state).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::auth::middleware,
            )),
        )
        .with_state(state.clone())
}
