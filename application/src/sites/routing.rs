use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::cache::SiteRecord;

use super::state::site_effective_domains;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct SiteRedirect {
    pub from_host: String,
    pub to_url: String,
    #[serde(default = "default_true")]
    pub permanent: bool,
}

fn default_true() -> bool {
    true
}

#[must_use]
pub fn build_routing_hosts(
    domain: &str,
    aliases: &[String],
    preview_domain: Option<&str>,
    wildcard_domain: bool,
) -> Vec<String> {
    let preview = preview_domain.filter(|d| !d.is_empty()).map(String::from);
    let stub = SiteRecord {
        id: String::new(),
        name: String::new(),
        domain: domain.into(),
        egg_id: String::new(),
        container_id: None,
        network: None,
        volume: None,
        git_repo: None,
        memory_mb: None,
        cpu_quota: None,
        disk_quota_mb: None,
        bandwidth_mbps: None,
        status: crate::sites::SITE_STATUS_ACTIVE.into(),
        domains: aliases.to_vec(),
        document_root: None,
        site_type: String::new(),
        variables: Default::default(),
        redirects: Vec::new(),
        preview_domain: preview,
        wildcard_domain,
        created_at: 0,
    };
    site_routing_hosts(&stub)
}

#[must_use]
pub fn site_routing_hosts(site: &SiteRecord) -> Vec<String> {
    let mut hosts = site_effective_domains(&site.domain, &site.domains);
    if let Some(preview) = site.preview_domain.as_ref().filter(|d| !d.is_empty())
        && !hosts.iter().any(|h| h == preview)
    {
        hosts.push(preview.clone());
    }
    if site.wildcard_domain {
        let wildcard = format!("*.{}", site.domain.trim().trim_start_matches("www."));
        if !hosts.iter().any(|h| h == &wildcard) {
            hosts.push(wildcard);
        }
    }
    hosts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::SITE_STATUS_ACTIVE;

    fn sample_site() -> SiteRecord {
        SiteRecord {
            id: "s1".into(),
            name: "Demo".into(),
            domain: "example.com".into(),
            egg_id: "web".into(),
            container_id: None,
            network: None,
            volume: None,
            git_repo: None,
            memory_mb: None,
            cpu_quota: None,
            disk_quota_mb: None,
            bandwidth_mbps: None,
            status: SITE_STATUS_ACTIVE.into(),
            domains: vec!["alias.example.com".into()],
            document_root: None,
            site_type: "web".into(),
            variables: Default::default(),
            redirects: Vec::new(),
            preview_domain: Some("preview.example.com".into()),
            wildcard_domain: true,
            created_at: 1,
        }
    }

    #[test]
    fn routing_hosts_include_aliases_preview_and_wildcard() {
        let hosts = site_routing_hosts(&sample_site());
        assert!(hosts.contains(&"example.com".into()));
        assert!(hosts.contains(&"alias.example.com".into()));
        assert!(hosts.contains(&"preview.example.com".into()));
        assert!(hosts.contains(&"*.example.com".into()));
    }
}
