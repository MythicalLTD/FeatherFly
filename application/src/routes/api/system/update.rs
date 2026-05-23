use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::GetState,
        update::UpdateStatus,
    };

    #[utoipa::path(
        get,
        path = "/",
        operation_id = "get_system_update",
        security(("bearer_auth" = [])),
        responses((status = OK, body = inline(UpdateStatus))),
    )]
    pub async fn route(state: GetState) -> ApiResponseResult {
        let channel = state.config.load().updates.channel;
        let status = crate::update::check_update(channel).await.map_err(|err| {
            tracing::error!("failed to check for updates: {err:#}");
            ApiResponse::error("failed to check for updates")
                .with_status(axum::http::StatusCode::BAD_GATEWAY)
        })?;

        ApiResponse::new_serialized(status).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
