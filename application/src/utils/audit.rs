use serde::Serialize;

use crate::{cache::AuditEventRecord, routes::State};

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub request_id: Option<String>,
    pub actor: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub client_ip: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<i64>,
    pub success: bool,
    pub message: Option<String>,
    pub metadata_json: Option<String>,
}

impl AuditEvent {
    #[must_use]
    pub fn action(action: impl Into<String>) -> Self {
        Self {
            request_id: None,
            actor: None,
            action: action.into(),
            resource_type: None,
            resource_id: None,
            client_ip: None,
            method: None,
            path: None,
            status: None,
            success: true,
            message: None,
            metadata_json: None,
        }
    }

    #[must_use]
    pub fn security(action: impl Into<String>) -> Self {
        Self {
            request_id: None,
            actor: None,
            action: action.into(),
            resource_type: None,
            resource_id: None,
            client_ip: None,
            method: None,
            path: None,
            status: None,
            success: false,
            message: None,
            metadata_json: None,
        }
    }

    #[must_use]
    pub fn with_request_context(
        mut self,
        request_id: &str,
        client_ip: &str,
        method: &str,
        path: &str,
        status: i64,
    ) -> Self {
        self.request_id = non_empty(request_id);
        self.client_ip = non_empty(client_ip);
        self.method = non_empty(method);
        self.path = non_empty(path);
        self.status = Some(status);
        self
    }

    #[must_use]
    pub fn with_resource(
        mut self,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        self.resource_type = Some(resource_type.into());
        self.resource_id = Some(resource_id.into());
        self
    }

    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata_json = serde_json::to_string(&metadata).ok();
        self
    }

    fn into_record(self) -> AuditEventRecord {
        AuditEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().timestamp(),
            request_id: self.request_id,
            actor: self.actor,
            action: self.action,
            resource_type: self.resource_type,
            resource_id: self.resource_id,
            client_ip: self.client_ip,
            method: self.method,
            path: self.path,
            status: self.status,
            success: i64::from(self.success),
            message: self.message,
            metadata_json: self.metadata_json,
        }
    }
}

pub async fn record(state: &State, event: AuditEvent) {
    let Some(cache) = &state.cache else {
        return;
    };
    if let Err(err) = cache.save_audit_event(&event.into_record()).await {
        tracing::debug!(%err, "failed to persist audit event");
    }
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_context_omits_empty_request_id() {
        let record = AuditEvent::security("auth.failed")
            .with_request_context("", "127.0.0.1", "GET", "/api/system", 401)
            .into_record();

        assert!(record.request_id.is_none());
        assert_eq!(record.client_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(record.status, Some(401));
        assert_eq!(record.success, 0);
    }

    #[test]
    fn metadata_serializes_to_json() {
        let record = AuditEvent::security("probe.blocked")
            .with_metadata(serde_json::json!({ "hits": 3 }))
            .into_record();

        assert_eq!(record.metadata_json.as_deref(), Some(r#"{"hits":3}"#));
    }

    #[test]
    fn action_events_default_to_success() {
        let record = AuditEvent::action("site.created")
            .with_resource("site", "demo")
            .into_record();

        assert_eq!(record.success, 1);
        assert_eq!(record.resource_type.as_deref(), Some("site"));
        assert_eq!(record.resource_id.as_deref(), Some("demo"));
    }
}
