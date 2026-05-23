use super::{GetState, State};
use crate::response::ApiResponse;
use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use utoipa_axum::router::OpenApiRouter;

mod system;

pub async fn auth(state: GetState, req: Request, next: Next) -> Result<Response<Body>, StatusCode> {
    let key = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let Some((typ, _token)) = key.split_once(' ') else {
        return Ok(ApiResponse::error("invalid authorization header")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    };

    if typ != "Bearer" {
        return Ok(ApiResponse::error("invalid authorization header")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    }

    if !crate::auth::validate_bearer_header(key, state.config.load().token.as_str()) {
        return Ok(ApiResponse::error("invalid authorization token")
            .with_status(StatusCode::UNAUTHORIZED)
            .with_header("WWW-Authenticate", "Bearer")
            .into_response());
    }

    Ok(next.run(req).await)
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest(
            "/system",
            system::router(state)
                .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth)),
        )
        .with_state(state.clone())
}
