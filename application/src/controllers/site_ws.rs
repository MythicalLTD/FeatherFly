use axum::{
    extract::{Path, WebSocketUpgrade},
    response::Response,
};

use crate::{routes::GetState, site_websocket::handle_site_ws};

#[utoipa::path(
    get,
    path = "/api/sites/{id}/ws",
    operation_id = "get_sites_ws",
    params(("id" = String, Path, description = "Site id")),
    responses(
        (status = 101, description = "WebSocket upgraded; client sends auth event with JWT"),
        (status = NOT_FOUND, description = "Site not found"),
    ),
)]
pub async fn get(ws: WebSocketUpgrade, state: GetState, Path(id): Path<String>) -> Response {
    handle_site_ws(ws, state, Path(id)).await
}
