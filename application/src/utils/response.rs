use axum::{http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
    pub status: u16,
}

impl ApiError {
    #[inline]
    pub fn new(error: impl Into<String>, status: StatusCode) -> Self {
        Self {
            error: error.into(),
            code: status_code(status),
            status: status.as_u16(),
        }
    }
}

pub type ApiResponseResult = Result<ApiResponse, ApiResponse>;

pub struct ApiResponse {
    pub body: axum::body::Body,
    pub status: axum::http::StatusCode,
    pub headers: Box<axum::http::HeaderMap>,
    error: Option<Box<ApiError>>,
}

impl ApiResponse {
    #[inline]
    #[allow(dead_code)]
    pub fn new(body: axum::body::Body) -> Self {
        Self {
            body,
            status: axum::http::StatusCode::OK,
            headers: Box::new(axum::http::HeaderMap::new()),
            error: None,
        }
    }

    pub fn new_serialized(body: impl serde::Serialize) -> Self {
        Self {
            body: axum::body::Body::from(serialize_json_body(body)),
            status: axum::http::StatusCode::OK,
            headers: Box::new(json_headers()),
            error: None,
        }
    }

    #[inline]
    pub fn error(err: &str) -> Self {
        let status = axum::http::StatusCode::BAD_REQUEST;
        let error = ApiError::new(err, status);
        Self {
            body: axum::body::Body::from(serialize_json_body(&error)),
            status,
            headers: Box::new(json_headers()),
            error: Some(Box::new(error)),
        }
    }

    #[inline]
    pub fn with_status(mut self, status: axum::http::StatusCode) -> Self {
        self.status = status;
        if let Some(error) = &mut self.error {
            error.code = status_code(status);
            error.status = status.as_u16();
            self.body = axum::body::Body::from(serialize_json_body(error.as_ref()));
            self.headers.extend(json_headers());
        }
        self
    }

    #[inline]
    pub fn with_header(mut self, key: &'static str, value: impl AsRef<str>) -> Self {
        if let Ok(header_value) = axum::http::HeaderValue::from_str(value.as_ref()) {
            self.headers.insert(key, header_value);
        }

        self
    }

    #[inline]
    pub fn ok(self) -> ApiResponseResult {
        Ok(self)
    }
}

fn json_headers() -> axum::http::HeaderMap {
    axum::http::HeaderMap::from_iter([(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    )])
}

fn serialize_json_body(body: impl serde::Serialize) -> Vec<u8> {
    serde_json::to_vec(&body).unwrap_or_else(|err| {
        tracing::error!("failed to serialize response body to JSON: {:?}", err);
        b"{}".to_vec()
    })
}

fn status_code(status: StatusCode) -> String {
    status
        .canonical_reason()
        .unwrap_or("error")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> axum::response::Response {
        let mut response = axum::response::Response::new(self.body);
        *response.status_mut() = self.status;
        response.headers_mut().extend(*self.headers);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn error_response_includes_code_and_status() {
        let response = ApiResponse::error("not allowed").with_status(StatusCode::FORBIDDEN);

        assert_eq!(response.status, StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.body, 1024).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            value,
            json!({
                "error": "not allowed",
                "code": "forbidden",
                "status": 403
            })
        );
    }

    #[tokio::test]
    async fn error_response_status_defaults_to_bad_request() {
        let response = ApiResponse::error("invalid input");

        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.body, 1024).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(value["code"], "bad_request");
        assert_eq!(value["status"], 400);
        assert_eq!(value["error"], "invalid input");
    }
}
