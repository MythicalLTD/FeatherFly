use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const SITE_STATUS_ACTIVE: &str = "active";
pub const SITE_STATUS_SUSPENDED: &str = "suspended";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SiteType {
    Static,
    Php,
    Node,
    Custom,
}

impl SiteType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Php => "php",
            Self::Node => "node",
            Self::Custom => "custom",
        }
    }

    #[must_use]
    pub fn from_template(template: &str) -> Self {
        match template {
            "static" => Self::Static,
            "php" => Self::Php,
            "nodejs" => Self::Node,
            _ => Self::Custom,
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "static" => Self::Static,
            "php" => Self::Php,
            "node" | "nodejs" => Self::Node,
            _ => Self::Custom,
        }
    }
}

#[must_use]
pub fn site_effective_domains(primary: &str, aliases: &[String]) -> Vec<String> {
    let mut domains = Vec::with_capacity(1 + aliases.len());
    domains.push(primary.to_string());
    for alias in aliases {
        if !alias.is_empty() && !domains.iter().any(|d| d == alias) {
            domains.push(alias.clone());
        }
    }
    domains
}

#[must_use]
pub fn site_is_active(status: &str) -> bool {
    status != SITE_STATUS_SUSPENDED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_domains_deduplicates_primary() {
        let domains = site_effective_domains(
            "example.com",
            &["www.example.com".into(), "example.com".into()],
        );
        assert_eq!(domains, vec!["example.com", "www.example.com"]);
    }

    #[test]
    fn site_type_from_template_maps_builtin() {
        assert_eq!(SiteType::from_template("static"), SiteType::Static);
        assert_eq!(SiteType::from_template("php"), SiteType::Php);
        assert_eq!(SiteType::from_template("nodejs"), SiteType::Node);
        assert_eq!(SiteType::from_template("custom-app"), SiteType::Custom);
    }
}
