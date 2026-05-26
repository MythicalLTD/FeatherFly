//! Wings-compatible site WebSocket auth and messaging.

mod handler;

pub use handler::handle_site_ws;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::remote::jwt::BasePayload;

pub const PERMISSION_WS_CONNECT: &str = "websocket.connect";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SiteWebsocketJwtPayload {
    #[serde(flatten)]
    pub base: BasePayload,
    pub site_id: String,
    #[serde(default)]
    pub user_uuid: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SiteWebsocketEvent {
    #[serde(rename = "auth")]
    Authentication,
    #[serde(rename = "auth success")]
    AuthenticationSuccess,
    #[serde(rename = "token expiring")]
    TokenExpiring,
    #[serde(rename = "token expired")]
    TokenExpired,
    #[serde(rename = "jwt error")]
    JwtError,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SiteWebsocketMessage {
    pub event: SiteWebsocketEvent,
    #[serde(default)]
    pub args: Vec<String>,
}

impl SiteWebsocketMessage {
    pub fn new(event: SiteWebsocketEvent, args: Vec<String>) -> Self {
        Self { event, args }
    }
}

pub fn has_permission(permissions: &[String], required: &str) -> bool {
    permissions
        .iter()
        .any(|perm| perm == "*" || perm == required)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_grants_permission() {
        assert!(has_permission(&["*".into()], PERMISSION_WS_CONNECT));
    }

    #[test]
    fn explicit_permission_required() {
        assert!(!has_permission(
            &["file.read".into()],
            PERMISSION_WS_CONNECT
        ));
        assert!(has_permission(
            &["websocket.connect".into()],
            PERMISSION_WS_CONNECT
        ));
    }
}
