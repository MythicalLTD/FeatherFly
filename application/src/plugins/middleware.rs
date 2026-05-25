use crate::routes::State as AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, header},
    middleware::Next,
};
use featherfly_plugin_sdk::PluginEvent;

use crate::utils::plugin_events::{self, PluginJsonMutatedPayload};

pub async fn response_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response<Body> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let response = next.run(request).await;

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !content_type.contains("application/json") {
        return response;
    }

    let (parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, 1024 * 1024).await else {
        return Response::from_parts(parts, Body::empty());
    };

    if bytes.is_empty() {
        return Response::from_parts(parts, Body::empty());
    }

    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    let body_json = serde_json::to_vec(&value).unwrap_or_else(|_| bytes.to_vec());
    let mutated_body = state
        .plugins
        .mutate_response_body(&path, &method, &body_json);
    if mutated_body != body_json {
        plugin_events::emit_state_event(
            &state,
            PluginEvent::PluginJsonResponseMutated,
            &PluginJsonMutatedPayload {
                method: &method,
                path: &path,
                input_len: body_json.len(),
                output_len: mutated_body.len(),
            },
        );
    }

    if let Ok(next_value) = serde_json::from_slice::<serde_json::Value>(&mutated_body) {
        value = next_value;
    }

    let had_actions = value.get("actions").is_some();
    let actions_input = serde_json::to_vec(value.get("actions").unwrap_or(&serde_json::json!([])))
        .unwrap_or_else(|_| b"[]".to_vec());
    let mutated_actions = state
        .plugins
        .mutate_response_actions(&path, &method, &actions_input);
    if mutated_actions != actions_input {
        plugin_events::emit_state_event(
            &state,
            PluginEvent::PluginJsonActionsMutated,
            &PluginJsonMutatedPayload {
                method: &method,
                path: &path,
                input_len: actions_input.len(),
                output_len: mutated_actions.len(),
            },
        );
    }

    if (had_actions || mutated_actions.as_slice() != b"[]")
        && let Ok(actions) = serde_json::from_slice::<serde_json::Value>(&mutated_actions)
        && let serde_json::Value::Object(ref mut map) = value
    {
        map.insert("actions".to_string(), actions);
    }

    let final_bytes = serde_json::to_vec(&value).unwrap_or(mutated_body);
    Response::from_parts(parts, Body::from(final_bytes))
}
