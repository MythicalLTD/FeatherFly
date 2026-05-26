use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtValidateError {
    Expired,
    NotYetValid,
    InvalidIssuedAt,
    Denied,
}

impl std::fmt::Display for JwtValidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expired => write!(f, "token is expired"),
            Self::NotYetValid => write!(f, "token is not yet valid"),
            Self::InvalidIssuedAt => write!(f, "token has invalid issued at time"),
            Self::Denied => write!(f, "token has been denied"),
        }
    }
}

impl std::error::Error for JwtValidateError {}

/// Standard JWT claims (Wings-compatible). Panel signs with the node `token` secret (HS256).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BasePayload {
    #[serde(rename = "iss")]
    pub issuer: String,
    #[serde(rename = "sub")]
    pub subject: Option<String>,
    #[serde(rename = "aud")]
    pub audience: Vec<String>,
    #[serde(rename = "exp")]
    pub expiration_time: Option<i64>,
    #[serde(rename = "nbf")]
    pub not_before: Option<i64>,
    #[serde(rename = "iat")]
    pub issued_at: Option<i64>,
    #[serde(rename = "jti")]
    pub jwt_id: String,
}

impl BasePayload {
    pub async fn validate(&self, client: &JwtClient) -> Result<(), JwtValidateError> {
        let now = chrono::Utc::now().timestamp();

        let Some(exp) = self.expiration_time else {
            return Err(JwtValidateError::Expired);
        };
        if exp < now {
            return Err(JwtValidateError::Expired);
        }

        if let Some(nbf) = self.not_before
            && nbf > now
        {
            return Err(JwtValidateError::NotYetValid);
        }

        let Some(iat) = self.issued_at else {
            return Err(JwtValidateError::InvalidIssuedAt);
        };
        if iat - 5 > now || iat < client.boot_time.timestamp() {
            return Err(JwtValidateError::InvalidIssuedAt);
        }

        if let Some(expired_until) = client.denied_jtokens.read().await.get(&self.jwt_id)
            && iat < expired_until.timestamp()
        {
            return Err(JwtValidateError::Denied);
        }

        Ok(())
    }
}

type CountingMap = HashMap<String, (usize, chrono::DateTime<chrono::Utc>)>;

pub struct JwtClient {
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
    pub validation: Validation,
    pub boot_time: chrono::DateTime<chrono::Utc>,
    pub max_jwt_uses: usize,
    pub denied_jtokens: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
    pub seen_jtoken_ids: Arc<RwLock<CountingMap>>,
}

impl JwtClient {
    pub fn new(token: &str, max_jwt_uses: usize) -> Self {
        let denied_jtokens = Arc::new(RwLock::new(HashMap::new()));
        let seen_jtoken_ids = Arc::new(RwLock::new(HashMap::new()));

        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn({
                let denied_jtokens = Arc::clone(&denied_jtokens);
                let seen_jtoken_ids = Arc::clone(&seen_jtoken_ids);
                async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        let mut denied = denied_jtokens.write().await;
                        denied.retain(|_, expiration| {
                            *expiration > chrono::Utc::now() - chrono::Duration::hours(1)
                        });
                        drop(denied);
                        let mut seen = seen_jtoken_ids.write().await;
                        seen.retain(|_, (_, expiration)| {
                            *expiration > chrono::Utc::now() - chrono::Duration::hours(1)
                        });
                    }
                }
            });
        }

        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        validation.validate_aud = false;
        validation.required_spec_claims.clear();

        Self {
            decoding_key: DecodingKey::from_secret(token.as_bytes()),
            encoding_key: EncodingKey::from_secret(token.as_bytes()),
            validation,
            boot_time: chrono::Utc::now(),
            max_jwt_uses,
            denied_jtokens,
            seen_jtoken_ids,
        }
    }

    pub fn verify<T: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<T, jsonwebtoken::errors::Error> {
        Ok(jsonwebtoken::decode::<T>(token, &self.decoding_key, &self.validation)?.claims)
    }

    pub fn create<T: Serialize>(&self, payload: &T) -> Result<String, jsonwebtoken::errors::Error> {
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), payload, &self.encoding_key)
    }

    pub async fn limited_jwt_id(&self, id: &str) -> bool {
        let seen = self.seen_jtoken_ids.read().await;
        if let Some((count, _)) = seen.get(id) {
            if *count >= self.max_jwt_uses {
                return false;
            }
            drop(seen);
            if self.max_jwt_uses != 0 {
                let mut seen = self.seen_jtoken_ids.write().await;
                if let Some((count, _)) = seen.get_mut(id) {
                    *count += 1;
                }
            }
        } else {
            drop(seen);
            self.seen_jtoken_ids
                .write()
                .await
                .insert(id.to_string(), (1, chrono::Utc::now()));
        }
        true
    }

    pub async fn deny(&self, id: impl Into<String>) {
        self.denied_jtokens
            .write()
            .await
            .insert(id.into(), chrono::Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_base(exp_offset: i64) -> BasePayload {
        let now = chrono::Utc::now().timestamp();
        BasePayload {
            issuer: "panel".into(),
            subject: None,
            audience: vec![],
            expiration_time: Some(now + exp_offset),
            not_before: None,
            issued_at: Some(now),
            jwt_id: "jti-1".into(),
        }
    }

    #[tokio::test]
    async fn validate_rejects_expired_token() {
        let client = JwtClient::new("secret", 5);
        let base = sample_base(-10);
        assert_eq!(
            base.validate(&client).await.unwrap_err(),
            JwtValidateError::Expired
        );
    }

    #[tokio::test]
    async fn limited_jwt_id_enforces_max_uses() {
        let client = JwtClient::new("secret", 2);
        assert!(client.limited_jwt_id("uid-1").await);
        assert!(client.limited_jwt_id("uid-1").await);
        assert!(!client.limited_jwt_id("uid-1").await);
    }
}
