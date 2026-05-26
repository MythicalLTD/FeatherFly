use uuid::Uuid;

const TOKEN_ID_LEN: usize = 16;
const TOKEN_LEN: usize = 64;
const TOKEN_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

pub fn ensure_node_identity(uuid: &mut String, token_id: &mut String, token: &mut String) -> bool {
    let mut changed = false;

    if uuid.trim().is_empty() {
        *uuid = Uuid::new_v4().to_string();
        changed = true;
    }

    if token_id.trim().is_empty() {
        *token_id = random_alphanumeric(TOKEN_ID_LEN);
        changed = true;
    }

    if token.trim().is_empty() {
        *token = random_alphanumeric(TOKEN_LEN);
        changed = true;
    }

    changed
}

pub fn generate_token_id() -> String {
    random_alphanumeric(TOKEN_ID_LEN)
}

pub fn generate_token_secret() -> String {
    random_alphanumeric(TOKEN_LEN)
}

fn random_alphanumeric(len: usize) -> String {
    let mut out = String::with_capacity(len);
    let mut buf = [0u8; 1];

    while out.len() < len {
        getrandom::fill(&mut buf).expect("failed to read random bytes");
        let idx = (buf[0] as usize) % TOKEN_ALPHABET.len();
        out.push(TOKEN_ALPHABET[idx] as char);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_missing_identity_fields() {
        let mut uuid = String::new();
        let mut token_id = String::new();
        let mut token = String::new();

        assert!(ensure_node_identity(&mut uuid, &mut token_id, &mut token));
        assert_eq!(uuid.len(), 36);
        assert_eq!(token_id.len(), TOKEN_ID_LEN);
        assert_eq!(token.len(), TOKEN_LEN);
    }

    #[test]
    fn preserves_existing_identity_fields() {
        let mut uuid = "existing-uuid".into();
        let mut token_id = "existing-token-id".into();
        let mut token = "existing-token".into();

        assert!(!ensure_node_identity(&mut uuid, &mut token_id, &mut token));
    }
}
