use constant_time_eq::constant_time_eq;

use crate::config::InnerConfig;

/// Panel-to-daemon API auth: single node bearer secret (Wings-compatible inbound).
pub fn validate_node_bearer(header: &str, inner: &InnerConfig) -> bool {
    let Some((typ, presented)) = header.split_once(' ') else {
        return false;
    };

    if typ != "Bearer" {
        return false;
    }

    constant_time_eq(presented.as_bytes(), inner.token.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> InnerConfig {
        InnerConfig {
            token_id: "node-id".into(),
            token: "node-secret".into(),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_node_bearer_token() {
        let inner = test_config();
        assert!(validate_node_bearer("Bearer node-secret", &inner));
    }

    #[test]
    fn rejects_wings_outbound_format_on_inbound_api() {
        let inner = test_config();
        assert!(!validate_node_bearer("Bearer node-id.node-secret", &inner));
    }

    #[test]
    fn rejects_wrong_token() {
        let inner = test_config();
        assert!(!validate_node_bearer("Bearer wrong", &inner));
    }
}
