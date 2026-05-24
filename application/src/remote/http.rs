use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::config::InnerConfig;

use super::backoff::ExponentialBackoff;

#[derive(Debug, Clone)]
pub struct PanelClient {
    http: Client,
    base_url: String,
    token_id: String,
    token: String,
    retry_limit: u32,
    extra_query: HashMap<String, String>,
    custom_headers: HashMap<String, String>,
}

#[derive(Debug)]
pub enum PanelRequestError {
    ClientError { status: StatusCode, body: String },
    Exhausted(anyhow::Error),
}

impl std::fmt::Display for PanelRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientError { status, body } => {
                write!(f, "panel returned HTTP {status}: {body}")
            }
            Self::Exhausted(err) => write!(f, "panel request failed after retries: {err:#}"),
        }
    }
}

impl std::error::Error for PanelRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Exhausted(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl PanelClient {
    #[must_use]
    pub fn from_config(inner: &InnerConfig) -> Option<Self> {
        let remote = inner.remote.trim();
        if remote.is_empty() {
            return None;
        }

        let timeout = Duration::from_secs(inner.remote_query.timeout.max(1) as u64);
        let http = Client::builder().timeout(timeout).build().ok()?;

        Some(Self {
            http,
            base_url: panel_api_base(remote),
            token_id: inner.token_id.clone(),
            token: inner.token.clone(),
            retry_limit: inner.remote_query.retry_limit,
            extra_query: inner.remote_query.query.clone(),
            custom_headers: inner.remote_query.custom_headers.clone(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn custom_headers(&self) -> &HashMap<String, String> {
        &self.custom_headers
    }

    pub fn retry_limit(&self) -> u32 {
        self.retry_limit
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &HashMap<String, String>,
    ) -> Result<T, PanelRequestError> {
        let response = self
            .request(Method::GET, path, None, query)
            .await
            .map_err(PanelRequestError::Exhausted)?;
        let body = response.text().await.map_err(|err| {
            PanelRequestError::Exhausted(
                anyhow::Error::from(err).context("failed to read panel response body"),
            )
        })?;
        serde_json::from_str(&body).map_err(|err| {
            PanelRequestError::Exhausted(
                anyhow::Error::from(err).context(format!("invalid JSON from panel: {body}")),
            )
        })
    }

    pub async fn post_json<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<Response, PanelRequestError> {
        let bytes =
            serde_json::to_vec(body).map_err(|err| PanelRequestError::Exhausted(err.into()))?;
        self.request(Method::POST, path, Some(bytes), &HashMap::new())
            .await
            .map_err(PanelRequestError::Exhausted)
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
        query: &HashMap<String, String>,
    ) -> Result<Response, anyhow::Error> {
        let url = format!("{}{}", self.base_url, path);
        let mut backoff = ExponentialBackoff::wings(self.retry_limit);
        let mut last_err = anyhow::anyhow!("panel request failed");

        loop {
            let builder = self.build_request(method.clone(), &url, body.as_deref(), query)?;
            match builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }
                    let body_text = response.text().await.unwrap_or_default();
                    if status.is_client_error() {
                        return Err(anyhow::anyhow!("panel client error {status}: {body_text}"));
                    }
                    last_err = anyhow::anyhow!("panel server error {status}: {body_text}");
                }
                Err(err) => {
                    if err.is_timeout() || err.is_connect() || err.is_request() {
                        last_err = err.into();
                    } else {
                        return Err(err.into());
                    }
                }
            }

            let Some(delay) = backoff.next_delay() else {
                break;
            };
            tracing::debug!(
                method = %method,
                path = %path,
                delay_ms = delay.as_millis(),
                "retrying panel HTTP request"
            );
            tokio::time::sleep(delay).await;
        }

        Err(last_err)
    }

    fn build_request(
        &self,
        method: Method,
        url: &str,
        body: Option<&[u8]>,
        query: &HashMap<String, String>,
    ) -> Result<RequestBuilder, anyhow::Error> {
        let mut builder = self.http.request(method, url);
        builder = builder
            .header(
                "Authorization",
                format!("Bearer {}.{}", self.token_id, self.token),
            )
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                format!("FeatherFly/{} (id:{})", crate::VERSION, self.token_id),
            );

        for (key, value) in &self.custom_headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        let mut merged = self.extra_query.clone();
        merged.extend(query.iter().map(|(k, v)| (k.clone(), v.clone())));
        if !merged.is_empty() {
            builder = builder.query(&merged);
        }

        if let Some(bytes) = body {
            builder = builder.body(bytes.to_vec());
        }

        Ok(builder)
    }
}

#[must_use]
pub fn panel_api_base(remote: &str) -> String {
    format!("{}/api/remote", remote.trim().trim_end_matches('/'))
}

#[must_use]
pub fn panel_bearer_token(token_id: &str, token: &str) -> String {
    format!("{token_id}.{token}")
}

pub fn panel_ws_url(remote: &str) -> Result<String, anyhow::Error> {
    let remote = remote.trim().trim_end_matches('/');
    let ws_base = if let Some(rest) = remote.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = remote.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if remote.starts_with("wss://") || remote.starts_with("ws://") {
        remote.to_string()
    } else {
        anyhow::bail!("remote must start with http://, https://, ws://, or wss://");
    };
    Ok(format!("{ws_base}/ws"))
}

pub fn apply_custom_headers(
    headers: &mut http::HeaderMap,
    custom: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    use http::header::{HeaderName, HeaderValue};

    for (key, value) in custom {
        let name = HeaderName::from_bytes(key.as_bytes())
            .with_context(|| format!("invalid custom header name: {key}"))?;
        let val = HeaderValue::from_str(value)
            .with_context(|| format!("invalid custom header value for {key}"))?;
        headers.insert(name, val);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_api_base_trims_trailing_slash() {
        assert_eq!(
            panel_api_base("https://panel.example.com/"),
            "https://panel.example.com/api/remote"
        );
    }

    #[test]
    fn bearer_uses_wings_format() {
        assert_eq!(panel_bearer_token("abc123", "secret"), "abc123.secret");
    }
}
