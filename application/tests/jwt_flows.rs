//! JWT auth integration tests (Wings-compatible flows).

use featherfly::remote::jwt::BasePayload;
use featherfly::site_websocket::{PERMISSION_WS_CONNECT, SiteWebsocketJwtPayload};

fn jwt_client() -> std::sync::Arc<featherfly::remote::jwt::JwtClient> {
    std::sync::Arc::new(featherfly::remote::jwt::JwtClient::new("test-secret", 2))
}

fn base_claims(now: i64) -> BasePayload {
    BasePayload {
        issuer: "panel".into(),
        subject: None,
        audience: vec![],
        expiration_time: Some(now + 300),
        not_before: None,
        issued_at: Some(now),
        jwt_id: uuid::Uuid::new_v4().to_string(),
    }
}

#[test]
fn websocket_jwt_roundtrip_has_connect_permission() {
    let client = jwt_client();
    let now = chrono::Utc::now().timestamp();
    let token = client
        .create(&SiteWebsocketJwtPayload {
            base: base_claims(now),
            site_id: "demo".into(),
            user_uuid: Some("user-1".into()),
            permissions: vec![PERMISSION_WS_CONNECT.into()],
        })
        .unwrap();

    let decoded: SiteWebsocketJwtPayload = client.verify(&token).unwrap();
    assert_eq!(decoded.site_id, "demo");
    assert!(featherfly::site_websocket::has_permission(
        &decoded.permissions,
        PERMISSION_WS_CONNECT
    ));
}

#[test]
fn upload_jwt_unique_id_respects_max_uses() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = jwt_client();
        assert!(client.limited_jwt_id("upload-1").await);
        assert!(client.limited_jwt_id("upload-1").await);
        assert!(!client.limited_jwt_id("upload-1").await);
    });
}
