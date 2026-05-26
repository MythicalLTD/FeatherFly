use crate::{
    cache::AuditEventRecord,
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use axum::extract::Query;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListAuditQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Serialize, ToSchema)]
struct ListAuditResponse {
    events: Vec<AuditEventRecord>,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_system_audit",
    security(("bearer_auth" = [])),
    params(ListAuditQuery),
    responses(
        (status = OK, body = inline(ListAuditResponse)),
        (status = SERVICE_UNAVAILABLE, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn list(state: GetState, Query(query): Query<ListAuditQuery>) -> ApiResponseResult {
    let Some(cache) = &state.cache else {
        return ApiResponse::error("cache unavailable")
            .with_status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            .ok();
    };

    match cache.list_audit_events(query.limit).await {
        Ok(events) => ApiResponse::new_serialized(ListAuditResponse { events }).ok(),
        Err(err) => ApiResponse::error(&format!("failed to list audit events: {err:#}"))
            .with_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            .ok(),
    }
}
