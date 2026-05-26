use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use featherfly_plugin_sdk::PluginEvent;

use crate::{
    routes::GetState,
    utils::{
        plugin_events::{self, RequestPayload},
        response::ApiResponse,
    },
};

pub async fn middleware(
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    state: GetState,
    req: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let request_id = crate::middlewares::request_id::current_request_id(&req).to_string();
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let client_ip = peer.ip().to_string();

    let key = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let invalid = || {
        tracing::warn!(
            request_id = %request_id,
            client_ip = %client_ip,
            method = %method,
            path = %path,
            "rejected request with invalid bearer token"
        );
        plugin_events::emit_state_event(
            &state,
            PluginEvent::AuthFailed,
            &RequestPayload {
                request_id: &request_id,
                client_ip: &client_ip,
                method: &method,
                path: &path,
                query: "",
            },
        );
        ApiResponse::error("invalid authorization token")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response()
    };

    if key.split_once(' ').is_none_or(|(typ, _)| typ != "Bearer") {
        return Ok(invalid());
    }

    if !crate::utils::auth::validate_node_bearer(key, &state.config.load()) {
        return Ok(invalid());
    }

    Ok(next.run(req).await)
}
