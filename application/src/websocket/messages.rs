use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PanelResponse {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMessage {
    Event {
        event: String,
        payload: serde_json::Value,
    },
    Response(PanelResponse),
}

impl OutboundMessage {
    pub fn from_lifecycle(event: &str, payload: &impl Serialize) -> Option<Self> {
        let payload = serde_json::to_value(payload).ok()?;
        Some(Self::Event {
            event: event.to_string(),
            payload,
        })
    }
}
