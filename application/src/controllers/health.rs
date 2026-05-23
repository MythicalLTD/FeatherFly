use axum::{Json, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: String,
}

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "get_health",
    responses((status = OK, body = inline(HealthResponse))),
)]
pub async fn get() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        service: "featherfly",
        version: crate::full_version(),
    })
}
