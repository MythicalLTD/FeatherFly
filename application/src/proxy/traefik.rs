use std::collections::HashMap;

use crate::config::ProxyConfig;

/// Primary site routing spec (main domain + optional aliases).
#[derive(Debug, Clone)]
pub struct SiteProxySpec {
    pub site_id: String,
    pub domains: Vec<String>,
    pub port: u16,
    pub path_prefix: Option<String>,
}

/// Addon service routing (webmail, admin UIs, etc.).
#[derive(Debug, Clone)]
pub struct ServiceRouteSpec {
    pub site_id: String,
    pub role: String,
    pub hosts: Vec<String>,
    pub port: u16,
    pub path_prefix: Option<String>,
}

pub fn build_site_labels(cfg: &ProxyConfig, spec: &SiteProxySpec) -> HashMap<String, String> {
    let hosts = expand_hosts(&spec.domains, cfg.include_www_alias);
    build_route_labels(
        cfg,
        &format!("site-{}", spec.site_id),
        &hosts,
        spec.port,
        spec.path_prefix.as_deref(),
    )
}

pub fn build_subdomain_labels(
    cfg: &ProxyConfig,
    site_id: &str,
    role: &str,
    host: &str,
    port: u16,
) -> HashMap<String, String> {
    build_route_labels(
        cfg,
        &format!("site-{site_id}-{role}"),
        &[host.to_string()],
        port,
        None,
    )
}

pub fn build_service_labels(cfg: &ProxyConfig, spec: &ServiceRouteSpec) -> HashMap<String, String> {
    let hosts = expand_hosts(&spec.hosts, cfg.include_www_alias);
    build_route_labels(
        cfg,
        &format!("site-{}-{}", spec.site_id, spec.role),
        &hosts,
        spec.port,
        spec.path_prefix.as_deref(),
    )
}

fn build_route_labels(
    cfg: &ProxyConfig,
    router_key: &str,
    hosts: &[String],
    port: u16,
    path_prefix: Option<&str>,
) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    if !cfg.enabled || cfg.provider != "traefik" || hosts.is_empty() {
        return labels;
    }

    let service = router_key.to_string();
    let host_rule = build_host_rule(hosts, path_prefix);
    if host_rule.is_empty() {
        return labels;
    }

    labels.insert("traefik.enable".into(), "true".into());
    labels.insert("traefik.docker.network".into(), cfg.network.clone());

    labels.insert(
        format!("traefik.http.services.{service}.loadbalancer.server.port"),
        port.to_string(),
    );

    if let Some(path) = cfg.health_check_path.as_ref().filter(|p| !p.is_empty()) {
        labels.insert(
            format!("traefik.http.services.{service}.loadbalancer.healthcheck.path"),
            path.clone(),
        );
        labels.insert(
            format!("traefik.http.services.{service}.loadbalancer.healthcheck.interval"),
            cfg.health_check_interval.clone(),
        );
        labels.insert(
            format!("traefik.http.services.{service}.loadbalancer.healthcheck.timeout"),
            "5s".into(),
        );
    }

    apply_middleware_definitions(&mut labels, cfg, router_key, cfg.routes_use_tls());

    if cfg.routes_use_tls() {
        labels.insert(
            format!("traefik.http.routers.{router_key}.rule"),
            host_rule.clone(),
        );
        labels.insert(
            format!("traefik.http.routers.{router_key}.entrypoints"),
            cfg.entrypoint_https.clone(),
        );
        labels.insert(
            format!("traefik.http.routers.{router_key}.tls.certresolver"),
            cfg.cert_resolver.clone(),
        );
        labels.insert(
            format!("traefik.http.routers.{router_key}.service"),
            service.clone(),
        );
        labels.insert(
            format!("traefik.http.routers.{router_key}.priority"),
            "100".into(),
        );
        if let Some(chain) = middleware_chain(cfg, router_key) {
            labels.insert(
                format!("traefik.http.routers.{router_key}.middlewares"),
                chain,
            );
        }

        if cfg.routes_redirect_http() {
            let insecure = format!("{router_key}-http");
            labels.insert(
                format!("traefik.http.middlewares.{router_key}-redir.redirectscheme.scheme"),
                "https".into(),
            );
            labels.insert(
                format!("traefik.http.middlewares.{router_key}-redir.redirectscheme.permanent"),
                "true".into(),
            );
            labels.insert(format!("traefik.http.routers.{insecure}.rule"), host_rule);
            labels.insert(
                format!("traefik.http.routers.{insecure}.entrypoints"),
                cfg.entrypoint_http.clone(),
            );
            labels.insert(
                format!("traefik.http.routers.{insecure}.middlewares"),
                format!("{router_key}-redir"),
            );
            labels.insert(format!("traefik.http.routers.{insecure}.service"), service);
            labels.insert(
                format!("traefik.http.routers.{insecure}.priority"),
                "90".into(),
            );
        }
    } else {
        labels.insert(format!("traefik.http.routers.{router_key}.rule"), host_rule);
        labels.insert(
            format!("traefik.http.routers.{router_key}.entrypoints"),
            cfg.entrypoint_http.clone(),
        );
        labels.insert(
            format!("traefik.http.routers.{router_key}.service"),
            service,
        );
        if let Some(chain) = middleware_chain(cfg, router_key) {
            labels.insert(
                format!("traefik.http.routers.{router_key}.middlewares"),
                chain,
            );
        }
    }

    labels
}

fn apply_middleware_definitions(
    labels: &mut HashMap<String, String>,
    cfg: &ProxyConfig,
    router_key: &str,
    tls_route: bool,
) {
    if cfg.compress_responses {
        labels.insert(
            format!("traefik.http.middlewares.{router_key}-compress.compress"),
            "true".into(),
        );
    }
    if cfg.security_headers {
        let prefix = format!("traefik.http.middlewares.{router_key}-sec.headers");
        if tls_route {
            labels.insert(format!("{prefix}.stsSeconds"), "31536000".into());
            labels.insert(format!("{prefix}.sslRedirect"), "true".into());
            labels.insert(format!("{prefix}.forceSTSHeader"), "true".into());
        }
        labels.insert(format!("{prefix}.browserXssFilter"), "true".into());
        labels.insert(format!("{prefix}.contentTypeNosniff"), "true".into());
        labels.insert(format!("{prefix}.frameDeny"), "true".into());
        labels.insert(
            format!("{prefix}.referrerPolicy"),
            "strict-origin-when-cross-origin".into(),
        );
    }
}

fn middleware_chain(cfg: &ProxyConfig, router_key: &str) -> Option<String> {
    let mut parts = Vec::new();
    if cfg.compress_responses {
        parts.push(format!("{router_key}-compress"));
    }
    if cfg.security_headers {
        parts.push(format!("{router_key}-sec"));
    }
    parts.push("featherfly-errors".into());
    Some(parts.join(","))
}

fn build_host_rule(hosts: &[String], path_prefix: Option<&str>) -> String {
    let host_expr = hosts
        .iter()
        .map(|d| host_matcher(d))
        .collect::<Vec<_>>()
        .join(" || ");

    if host_expr.is_empty() {
        return String::new();
    }

    match path_prefix.filter(|p| !p.is_empty()) {
        Some(prefix) => {
            let path = prefix.trim_start_matches('/');
            format!("({host_expr}) && PathPrefix(`/{path}`)")
        }
        None => host_expr,
    }
}

fn host_matcher(domain: &str) -> String {
    let domain = domain.trim().trim_end_matches('.');
    if let Some(base) = domain.strip_prefix("*.") {
        format!("HostRegexp(`{{subdomain:[a-zA-Z0-9-]+}}.{base}`)")
    } else {
        format!("Host(`{domain}`)")
    }
}

#[derive(Debug, Clone)]
pub struct RedirectSpec {
    pub from_host: String,
    pub to_url: String,
    pub permanent: bool,
}

pub fn build_redirect_labels(
    cfg: &ProxyConfig,
    site_id: &str,
    service_key: &str,
    redirects: &[RedirectSpec],
) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    if !cfg.enabled || cfg.provider != "traefik" || redirects.is_empty() {
        return labels;
    }

    for (idx, redirect) in redirects.iter().enumerate() {
        let key = format!("site-{site_id}-redir-{idx}");
        let from = redirect.from_host.trim();
        let to = redirect.to_url.trim();
        if from.is_empty() || to.is_empty() {
            continue;
        }
        labels.insert(
            format!("traefik.http.middlewares.{key}.redirectregex.regex"),
            format!("^https?://{from}/(.*)"),
        );
        labels.insert(
            format!("traefik.http.middlewares.{key}.redirectregex.replacement"),
            format!("{to}/$1"),
        );
        labels.insert(
            format!("traefik.http.middlewares.{key}.redirectregex.permanent"),
            redirect.permanent.to_string(),
        );
        labels.insert(
            format!("traefik.http.routers.{key}.rule"),
            format!("Host(`{from}`)"),
        );
        labels.insert(
            format!("traefik.http.routers.{key}.entrypoints"),
            if cfg.routes_use_tls() {
                cfg.entrypoint_https.clone()
            } else {
                cfg.entrypoint_http.clone()
            },
        );
        labels.insert(
            format!("traefik.http.routers.{key}.service"),
            service_key.into(),
        );
        labels.insert(
            format!("traefik.http.routers.{key}.middlewares"),
            key.clone(),
        );
        labels.insert(format!("traefik.http.routers.{key}.priority"), "110".into());
    }

    labels
}

/// Add `www.` variants for apex domains (skip when already www or multi-level).
pub fn expand_hosts(domains: &[String], include_www: bool) -> Vec<String> {
    let mut out = Vec::new();
    for domain in domains {
        let d = domain.trim().trim_end_matches('.');
        if d.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == d) {
            out.push(d.to_string());
        }
        if include_www && should_add_www(d) {
            let www = format!("www.{d}");
            if !out.iter().any(|existing| existing == &www) {
                out.push(www);
            }
        }
    }
    out
}

fn should_add_www(domain: &str) -> bool {
    !domain.starts_with("www.") && !domain.starts_with('*') && domain.matches('.').count() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_proxy_config() -> ProxyConfig {
        ProxyConfig {
            enabled: true,
            provider: "traefik".into(),
            cert_resolver: "letsencrypt".into(),
            entrypoint_http: "web".into(),
            entrypoint_https: "websecure".into(),
            network: "featherfly_nw".into(),
            tls_enabled: true,
            redirect_http_to_https: true,
            compress_responses: true,
            security_headers: true,
            health_check_path: Some("/".into()),
            health_check_interval: "30s".into(),
            include_www_alias: true,
            auto_provision: true,
            traefik_image: "traefik:v3.3".into(),
            acme_email: "admin@example.com".into(),
            dashboard_enabled: false,
            publish_http_port: 80,
            publish_https_port: 443,
        }
    }

    fn test_proxy_config_http_only() -> ProxyConfig {
        ProxyConfig {
            acme_email: String::new(),
            auto_provision: true,
            ..test_proxy_config()
        }
    }

    #[test]
    fn expands_www_for_apex_domain() {
        let hosts = expand_hosts(&["example.com".into()], true);
        assert!(hosts.contains(&"example.com".into()));
        assert!(hosts.contains(&"www.example.com".into()));
    }

    #[test]
    fn skips_www_for_subdomains() {
        let hosts = expand_hosts(&["webmail.example.com".into()], true);
        assert_eq!(hosts, vec!["webmail.example.com".to_string()]);
    }

    #[test]
    fn builds_https_and_redirect_routers() {
        let cfg = test_proxy_config();
        let labels = build_site_labels(
            &cfg,
            &SiteProxySpec {
                site_id: "demo".into(),
                domains: vec!["example.com".into()],
                port: 8080,
                path_prefix: None,
            },
        );

        assert_eq!(labels.get("traefik.enable"), Some(&"true".into()));
        assert!(
            labels
                .get("traefik.http.routers.site-demo.rule")
                .is_some_and(|r| r.contains("example.com"))
        );
        assert_eq!(
            labels.get("traefik.http.routers.site-demo.entrypoints"),
            Some(&"websecure".into())
        );
        assert!(labels.contains_key("traefik.http.routers.site-demo-http.rule"));
        assert!(labels.contains_key("traefik.http.middlewares.site-demo-compress.compress"));
        assert!(
            labels.contains_key("traefik.http.services.site-demo.loadbalancer.healthcheck.path")
        );
    }

    #[test]
    fn builds_http_router_without_acme_email() {
        let cfg = test_proxy_config_http_only();
        let labels = build_site_labels(
            &cfg,
            &SiteProxySpec {
                site_id: "demo".into(),
                domains: vec!["example.com".into()],
                port: 8080,
                path_prefix: None,
            },
        );

        assert_eq!(
            labels.get("traefik.http.routers.site-demo.entrypoints"),
            Some(&"web".into())
        );
        assert!(!labels.contains_key("traefik.http.routers.site-demo.tls.certresolver"));
        assert!(!labels.contains_key("traefik.http.routers.site-demo-http.rule"));
    }
}
