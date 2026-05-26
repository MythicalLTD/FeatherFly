use crate::{
    eggs::{EggDefinition, EggSummary, list_eggs, load_egg},
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use axum::http::StatusCode;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_eggs",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(Vec<EggSummary>))),
)]
pub async fn list(state: GetState) -> ApiResponseResult {
    let dir = state.config.load().hosting.eggs_directory.clone();
    match list_eggs(&dir) {
        Ok(eggs) => ApiResponse::new_serialized(eggs).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list eggs: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "get_eggs_by_id",
    security(("bearer_auth" = [])),
    params(("id" = String, Path)),
    responses((status = OK, body = inline(EggDefinition))),
)]
pub async fn get(
    state: GetState,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResponseResult {
    let dir = state.config.load().hosting.eggs_directory.clone();
    match load_egg(&dir, &id) {
        Ok(egg) => ApiResponse::new_serialized(egg).ok(),
        Err(err) => ApiResponse::error(&format!("failed to load egg: {err:#}"))
            .with_status(StatusCode::NOT_FOUND)
            .ok(),
    }
}
