use crate::plugins::events::{RegisteredRoute, code_to_method};
use crate::routes::State as AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::any,
};
use featherfly_plugin_sdk::ROUTE_OK;
use utoipa_axum::router::OpenApiRouter;

pub fn router(state: &AppState) -> OpenApiRouter<AppState> {
    let mut plugin_router = OpenApiRouter::new();

    for path in unique_paths(state) {
        plugin_router = plugin_router.route(&path, any(dispatch_plugin_route));
    }

    plugin_router.with_state(state.clone())
}

fn unique_paths(state: &AppState) -> Vec<String> {
    let mut paths: Vec<String> = state
        .plugins
        .routes()
        .into_iter()
        .map(|route| route.path)
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

async fn dispatch_plugin_route(State(state): State<AppState>, req: Request) -> Response<Body> {
    let path = req.uri().path();
    let method = req.method().as_str();

    let Some(route) = state
        .plugins
        .routes()
        .into_iter()
        .find(|route| route.path == path && code_to_method(route.method) == method)
    else {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    };

    invoke_route(&route, req).await
}

async fn invoke_route(route: &RegisteredRoute, req: Request) -> Response<Body> {
    let body = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mut out = vec![0_u8; body.len().max(4096).saturating_mul(2)];
    let mut out_len = 0_usize;
    let mut status = 200_u16;

    let ctx = featherfly_plugin_sdk::RouteHandlerContext {
        method: route.method,
        path_ptr: route.path.as_ptr(),
        path_len: route.path.len(),
        body_in_ptr: body.as_ptr(),
        body_in_len: body.len(),
        body_out_ptr: out.as_mut_ptr(),
        body_out_capacity: out.len(),
        body_out_len: &mut out_len,
        status_code: &mut status,
    };

    let result = (route.callback)(&ctx);
    if result != ROUTE_OK {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut response = Response::new(Body::from(out[..out_len].to_vec()));
    *response.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
    response
}
