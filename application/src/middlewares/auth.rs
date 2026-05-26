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
    utils::{audit::AuditEvent, plugin_events, response::ApiResponse},
};

pub async fn middleware(
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    state: GetState,
    req: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let client_ip = plugin_events::ip_string(peer.ip());
    let request_id = crate::middlewares::request_id::current_request_id(&req).to_string();
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
                "request_id": request_id,
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
        record_auth_failed(
            &state,
            &request_id,
            &client_ip,
            method,
            path,
            401,
            "invalid authorization header",
        )
        .await;
        return Ok(ApiResponse::error("invalid authorization header")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    };

    if typ != "Bearer" {
        emit_auth_failed("invalid authorization header");
        record_auth_failed(
            &state,
            &request_id,
            &client_ip,
            method,
            path,
            401,
            "invalid authorization header",
        )
        .await;
        return Ok(ApiResponse::error("invalid authorization header")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    }

    if !crate::utils::auth::validate_node_bearer(key, &state.config.load()) {
        emit_auth_failed("invalid authorization token");
        record_auth_failed(
            &state,
            &request_id,
            &client_ip,
            method,
            path,
            401,
            "invalid authorization token",
        )
        .await;
        return Ok(ApiResponse::error("invalid authorization token")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    }

    Ok(next.run(req).await)
}

async fn record_auth_failed(
    state: &GetState,
    request_id: &str,
    client_ip: &str,
    method: &str,
    path: &str,
    status: i64,
    message: &str,
) {
    crate::utils::audit::record(
        state,
        AuditEvent::security("auth.failed")
            .with_request_context(request_id, client_ip, method, path, status)
            .with_message(message),
    )
    .await;
}
