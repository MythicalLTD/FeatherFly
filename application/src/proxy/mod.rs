mod bootstrap;
mod traefik;

pub use bootstrap::{
    TRAEFIK_CONTAINER_NAME, TraefikEnsureResult, TraefikStatus, ensure_traefik,
    traefik_result_label, traefik_runtime_status,
};
pub use traefik::{
    RedirectSpec, ServiceRouteSpec, SiteProxySpec, build_redirect_labels, build_service_labels,
    build_site_labels, build_subdomain_labels, expand_hosts,
};
