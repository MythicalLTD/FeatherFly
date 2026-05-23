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
    utils::{plugin_events, response::ApiResponse},
};

pub async fn middleware(
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    state: GetState,
    req: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let client_ip = plugin_events::ip_string(peer.ip());
    let method = req.method().as_str();
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");

    let emit_auth_failed = |reason: &str| {
        plugin_events::emit_json(
            &state.plugins,
            PluginEvent::AuthFailed,
            &serde_json::json!({
                "client_ip": client_ip,
                "method": method,
                "path": path,
                "query": query,
                "reason": reason,
            }),
        );
    };

    let key = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let Some((typ, _token)) = key.split_once(' ') else {
        emit_auth_failed("invalid authorization header");
        return Ok(ApiResponse::error("invalid authorization header")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    };

    if typ != "Bearer" {
        emit_auth_failed("invalid authorization header");
        return Ok(ApiResponse::error("invalid authorization header")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    }

    if !crate::utils::auth::validate_bearer_header(key, state.config.load().token.as_str()) {
        emit_auth_failed("invalid authorization token");
        return Ok(ApiResponse::error("invalid authorization token")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    }

    Ok(next.run(req).await)
}
