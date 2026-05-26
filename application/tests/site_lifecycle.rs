use featherfly::sites::{
    SITE_STATUS_ACTIVE, SITE_STATUS_SUSPENDED, SiteType, site_effective_domains, site_is_active,
};

#[test]
fn suspended_site_is_not_active() {
    assert!(site_is_active(SITE_STATUS_ACTIVE));
    assert!(!site_is_active(SITE_STATUS_SUSPENDED));
}

#[test]
fn site_type_parse_accepts_node_alias() {
    assert_eq!(SiteType::parse("node"), SiteType::Node);
}

#[test]
fn effective_domains_keeps_primary_first() {
    let domains = site_effective_domains("primary.test", &["alias.test".into()]);
    assert_eq!(domains[0], "primary.test");
    assert!(domains.contains(&"alias.test".into()));
}
