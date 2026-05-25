use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    cache::TaskRecord,
    routes::State,
    utils::plugin_events::{self, TaskFailedPayload, TaskScheduledPayload},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Cron,
    Once,
    OnDemand,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskAction {
    Deploy,
    Backup,
    Restart,
    Shell,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateTaskRequest {
    pub id: String,
    pub name: String,
    pub kind: TaskKind,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub run_at: Option<i64>,
    pub action: TaskAction,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TaskSummary {
    pub id: String,
    pub site_id: String,
    pub name: String,
    pub kind: String,
    pub schedule: Option<String>,
    pub action: String,
    pub enabled: bool,
}

pub struct SchedulerService;

impl SchedulerService {
    pub async fn create(
        state: &State,
        site_id: &str,
        req: CreateTaskRequest,
    ) -> Result<TaskSummary, anyhow::Error> {
        let cache = state
            .cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
        cache
            .get_site(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("site not found"))?;

        let record = TaskRecord {
            id: req.id.clone(),
            site_id: site_id.to_string(),
            name: req.name.clone(),
            kind: kind_str(req.kind).into(),
            schedule: req.schedule.clone(),
            run_at: req.run_at,
            action: action_str(req.action).into(),
            command: req.command.clone(),
            enabled: i64::from(req.enabled),
            last_run_at: None,
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.save_task(&record).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::TaskScheduled,
            &TaskScheduledPayload {
                site_id,
                task_id: &req.id,
                kind: &record.kind,
                action: &record.action,
            },
        );

        Ok(to_summary(&record))
    }

    pub async fn list(state: &State, site_id: &str) -> Result<Vec<TaskSummary>, anyhow::Error> {
        let cache = state
            .cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
        Ok(cache
            .list_tasks(site_id)
            .await?
            .iter()
            .map(to_summary)
            .collect())
    }

    pub async fn run_now(state: &State, site_id: &str, task_id: &str) -> Result<(), anyhow::Error> {
        let cache = state
            .cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
        let task = cache
            .get_task(site_id, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("task not found"))?;
        if let Err(err) = crate::scheduler::runner::execute_task(state, &task).await {
            plugin_events::emit_state_event(
                state,
                PluginEvent::TaskFailed,
                &TaskFailedPayload {
                    site_id: &task.site_id,
                    task_id: &task.id,
                    error: err.to_string(),
                },
            );
            return Err(err);
        }
        cache.mark_task_run(site_id, task_id).await?;
        Ok(())
    }

    pub async fn delete(state: &State, site_id: &str, task_id: &str) -> Result<(), anyhow::Error> {
        let cache = state
            .cache
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
        cache.delete_task(site_id, task_id).await?;
        Ok(())
    }
}

fn kind_str(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::Cron => "cron",
        TaskKind::Once => "once",
        TaskKind::OnDemand => "on_demand",
    }
}

fn action_str(action: TaskAction) -> &'static str {
    match action {
        TaskAction::Deploy => "deploy",
        TaskAction::Backup => "backup",
        TaskAction::Restart => "restart",
        TaskAction::Shell => "shell",
    }
}

fn to_summary(record: &TaskRecord) -> TaskSummary {
    TaskSummary {
        id: record.id.clone(),
        site_id: record.site_id.clone(),
        name: record.name.clone(),
        kind: record.kind.clone(),
        schedule: record.schedule.clone(),
        action: record.action.clone(),
        enabled: record.enabled != 0,
    }
}
