use crate::routes::ApiError;
use axum::response::IntoResponse;

pub type ApiResponseResult = Result<ApiResponse, ApiResponse>;

pub struct ApiResponse {
    pub body: axum::body::Body,
    pub status: axum::http::StatusCode,
    pub headers: axum::http::HeaderMap,
}

impl ApiResponse {
    #[inline]
    #[allow(dead_code)]
    pub fn new(body: axum::body::Body) -> Self {
        Self {
            body,
            status: axum::http::StatusCode::OK,
            headers: axum::http::HeaderMap::new(),
        }
    }

    pub fn new_serialized(body: impl serde::Serialize) -> Self {
        let bytes = serde_json::to_vec(&body).unwrap_or_else(|err| {
            tracing::error!("failed to serialize response body to JSON: {:?}", err);
            b"{}".to_vec()
        });

        Self {
            body: axum::body::Body::from(bytes),
            status: axum::http::StatusCode::OK,
            headers: axum::http::HeaderMap::from_iter([(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            )]),
        }
    }

    #[inline]
    pub fn error(err: &str) -> Self {
        Self::new_serialized(ApiError::new(err)).with_status(axum::http::StatusCode::BAD_REQUEST)
    }

    #[inline]
    pub fn with_status(mut self, status: axum::http::StatusCode) -> Self {
        self.status = status;
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

impl IntoResponse for ApiResponse {
    fn into_response(self) -> axum::response::Response {
        let mut response = axum::response::Response::new(self.body);
        *response.status_mut() = self.status;
        response.headers_mut().extend(self.headers);
        response
    }
}
