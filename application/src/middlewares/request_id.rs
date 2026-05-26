use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(String);

impl RequestId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub async fn middleware(mut req: Request, next: Next) -> Response {
    let request_id = request_id_from_headers(req.headers()).unwrap_or_else(generate_request_id);
    req.extensions_mut().insert(request_id.clone());

    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(request_id.as_str()) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}

#[must_use]
pub fn current_request_id(req: &Request) -> &str {
    req.extensions()
        .get::<RequestId>()
        .map(RequestId::as_str)
        .unwrap_or("")
}

fn request_id_from_headers(headers: &HeaderMap) -> Option<RequestId> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(sanitize_request_id)
}

fn sanitize_request_id(value: &str) -> Option<RequestId> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return None;
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        return None;
    }
    Some(RequestId(value.to_string()))
}

fn generate_request_id() -> RequestId {
    RequestId(uuid::Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_incoming_request_id() {
        assert_eq!(
            sanitize_request_id("req-123_abc.def:456").unwrap().as_str(),
            "req-123_abc.def:456"
        );
    }

    #[test]
    fn rejects_empty_long_or_unsafe_request_ids() {
        assert!(sanitize_request_id("").is_none());
        assert!(sanitize_request_id(&"a".repeat(129)).is_none());
        assert!(sanitize_request_id("bad id").is_none());
        assert!(sanitize_request_id("bad\nid").is_none());
    }

    #[test]
    fn generated_request_id_is_header_safe() {
        let request_id = generate_request_id();
        assert!(HeaderValue::from_str(request_id.as_str()).is_ok());
        assert!(sanitize_request_id(request_id.as_str()).is_some());
    }
}
