use constant_time_eq::constant_time_eq;

pub fn validate_bearer_header(header: &str, token_id: &str, token: &str) -> bool {
    let (typ, presented) = match header.split_once(' ') {
        Some((t, tok)) => (t, tok),
        None => return false,
    };

    if typ != "Bearer" {
        return false;
    }

    // Plain daemon token (legacy) or Wings-style token_id.token from the panel.
    constant_time_eq(presented.as_bytes(), token.as_bytes())
        || constant_time_eq(
            presented.as_bytes(),
            format!("{token_id}.{token}").as_bytes(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_token() {
        assert!(validate_bearer_header(
            "Bearer secret-token",
            "id123",
            "secret-token",
        ));
    }

    #[test]
    fn accepts_wings_token_format() {
        assert!(validate_bearer_header(
            "Bearer id123.secret-token",
            "id123",
            "secret-token",
        ));
    }

    #[test]
    fn rejects_missing_bearer_prefix() {
        assert!(!validate_bearer_header(
            "secret-token",
            "id",
            "secret-token"
        ));
    }

    #[test]
    fn rejects_wrong_token() {
        assert!(!validate_bearer_header(
            "Bearer wrong",
            "id",
            "secret-token"
        ));
    }

    #[test]
    fn rejects_empty_header() {
        assert!(!validate_bearer_header("", "id", "secret-token"));
    }
}
