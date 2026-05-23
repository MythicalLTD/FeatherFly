use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiAction {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
}

impl ApiAction {
    pub fn get(id: impl Into<String>, label: impl Into<String>, step: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            step: Some(step.into()),
        }
    }
}

pub fn system_actions() -> Vec<ApiAction> {
    vec![
        ApiAction::get(
            "system_overview",
            "System overview",
            "GET /api/system/overview",
        ),
        ApiAction::get(
            "check_update",
            "Check for updates",
            "GET /api/system/update",
        ),
        ApiAction::get("list_plugins", "List plugins", "GET /api/system/plugins"),
        ApiAction::get("diagnostics", "Run diagnostics", "GET /health"),
    ]
}

pub fn plugins_actions() -> Vec<ApiAction> {
    vec![
        ApiAction::get("reload_plugins", "Reload plugins", "POST /api/system/plugins/reload"),
        ApiAction::get("plugin_docs", "Plugin developer docs", "GET /docs/plugins"),
    ]
}
