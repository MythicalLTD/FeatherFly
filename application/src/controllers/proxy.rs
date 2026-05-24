use crate::{
    proxy::traefik_runtime_status,
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProxyStatusResponse {
    pub enabled: bool,
    pub provider: String,
    pub auto_provision: bool,
    pub tls_enabled: bool,
    pub acme_ready: bool,
    pub redirect_http_to_https: bool,
    pub compress_responses: bool,
    pub security_headers: bool,
    pub include_www_alias: bool,
    pub network: String,
    pub cert_resolver: String,
    pub traefik: TraefikRuntimeStatus,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TraefikRuntimeStatus {
    pub running: bool,
    pub container_id: Option<String>,
    pub image: Option<String>,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_proxy_status",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(ProxyStatusResponse))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let inner = state.config.load();
    let runtime = traefik_runtime_status(state.docker.as_deref(), &inner).await;

    ApiResponse::new_serialized(ProxyStatusResponse {
        enabled: inner.proxy.enabled,
        provider: inner.proxy.provider.clone(),
        auto_provision: inner.proxy.auto_provision,
        tls_enabled: inner.proxy.tls_enabled,
        acme_ready: inner.proxy.acme_ready(),
        redirect_http_to_https: inner.proxy.routes_redirect_http(),
        compress_responses: inner.proxy.compress_responses,
        security_headers: inner.proxy.security_headers,
        include_www_alias: inner.proxy.include_www_alias,
        network: inner.proxy.network.clone(),
        cert_resolver: inner.proxy.cert_resolver.clone(),
        traefik: TraefikRuntimeStatus {
            running: runtime.running,
            container_id: runtime.container_id,
            image: runtime.image,
        },
    })
    .ok()
}
