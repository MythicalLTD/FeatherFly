use constant_time_eq::constant_time_eq;

pub fn validate_bearer_header(header: &str, expected_token: &str) -> bool {
    let (typ, token) = match header.split_once(' ') {
        Some((t, tok)) => (t, tok),
        None => return false,
    };

    if typ != "Bearer" {
        return false;
    }

    constant_time_eq(token.as_bytes(), expected_token.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_bearer_token() {
        assert!(validate_bearer_header(
            "Bearer secret-token",
            "secret-token",
        ));
    }

    #[test]
    fn rejects_missing_bearer_prefix() {
        assert!(!validate_bearer_header("secret-token", "secret-token"));
    }

    #[test]
    fn rejects_wrong_token() {
        assert!(!validate_bearer_header("Bearer wrong", "secret-token"));
    }

    #[test]
    fn rejects_empty_header() {
        assert!(!validate_bearer_header("", "secret-token"));
    }
}
