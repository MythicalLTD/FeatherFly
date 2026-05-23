use crate::routes::State as AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode},
    middleware::Next,
};
use featherfly_plugin_sdk::RequestHookPhase;

pub async fn intercept_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response<Body> {
    run_request_hooks(state, request, next, RequestHookPhase::Intercept).await
}

pub async fn inject_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response<Body> {
    run_request_hooks(state, request, next, RequestHookPhase::Middleware).await
}

async fn run_request_hooks(
    state: AppState,
    request: Request,
    next: Next,
    phase: RequestHookPhase,
) -> Response<Body> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let (parts, body) = request.into_parts();

    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let headers_json = {
        let mut map = serde_json::Map::new();
        for (name, value) in parts.headers.iter() {
            let key = name.as_str().to_ascii_lowercase();
            let value = if key == "authorization" {
                serde_json::Value::String("<redacted>".into())
            } else if let Ok(text) = value.to_str() {
                serde_json::Value::String(text.into())
            } else {
                continue;
            };
            map.insert(key, value);
        }
        serde_json::to_vec(&map).unwrap_or_else(|_| b"{}".to_vec())
    };

    let outcome =
        state
            .plugins
            .run_request_hooks(phase, &path, &method, &headers_json, &body_bytes);

    if outcome.respond {
        let mut response = Response::new(Body::from(outcome.body));
        *response.status_mut() = StatusCode::from_u16(outcome.status).unwrap_or(StatusCode::OK);
        return response;
    }

    let request = Request::from_parts(parts, Body::from(body_bytes));
    next.run(request).await
}

use axum::response::IntoResponse;
