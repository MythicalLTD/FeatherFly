use std::str::FromStr;

use anyhow::Context;
use chrono::Utc;
use cron::Schedule;
use featherfly_plugin_sdk::PluginEvent;

use crate::{
    cache::TaskRecord,
    routes::State,
    sites::{DeployRequest, SiteService},
    utils::plugin_events::{self, TaskExecutedPayload, TaskFailedPayload},
};

pub fn spawn_scheduler(state: crate::routes::State) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(err) = tick(&state).await {
                tracing::debug!(%err, "scheduler tick error");
            }
        }
    });
}

async fn tick(state: &State) -> Result<(), anyhow::Error> {
    let Some(cache) = state.cache.as_ref() else {
        return Ok(());
    };
    let tasks = cache.list_enabled_tasks().await?;
    let now = Utc::now().timestamp();

    for task in tasks {
        if !should_run(&task, now) {
            continue;
        }
        if let Err(err) = execute_task(state, &task).await {
            plugin_events::emit_state_event(
                state,
                PluginEvent::TaskFailed,
                &TaskFailedPayload {
                    site_id: &task.site_id,
                    task_id: &task.id,
                    error: err.to_string(),
                },
            );
        } else {
            cache.mark_task_run(&task.site_id, &task.id).await?;
        }
    }
    Ok(())
}

fn should_run(task: &TaskRecord, now: i64) -> bool {
    match task.kind.as_str() {
        "once" => task
            .run_at
            .is_some_and(|t| t <= now && task.last_run_at.is_none()),
        "cron" => {
            let Some(schedule) = task.schedule.as_ref() else {
                return false;
            };
            let Ok(schedule) = Schedule::from_str(schedule) else {
                return false;
            };
            let last = task
                .last_run_at
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
            schedule
                .after(&last)
                .next()
                .is_some_and(|next| next.timestamp() <= now)
        }
        _ => false,
    }
}

pub async fn execute_task(state: &State, task: &TaskRecord) -> Result<(), anyhow::Error> {
    match task.action.as_str() {
        "deploy" => {
            SiteService::deploy(
                state,
                &task.site_id,
                DeployRequest {
                    git_ref: None,
                    rebuild: false,
                    zero_downtime: false,
                },
            )
            .await?;
        }
        "backup" => {
            crate::sites::SiteService::create_backup(state, &task.site_id).await?;
        }
        "restart" => {
            let docker = state.docker.as_ref().context("docker unavailable")?;
            let cache = state.cache.as_ref().context("cache unavailable")?;
            let site = cache
                .get_site(&task.site_id)
                .await?
                .context("site not found")?;
            if let Some(cid) = site.container_id {
                docker.restart_container(&cid).await?;
            }
        }
        "shell" => {
            if let Some(cmd) = &task.command {
                run_shell(state, &task.site_id, cmd).await?;
            }
        }
        other => anyhow::bail!("unknown task action: {other}"),
    }

    plugin_events::emit_state_event(
        state,
        PluginEvent::TaskExecuted,
        &TaskExecutedPayload {
            site_id: &task.site_id,
            task_id: &task.id,
            action: &task.action,
        },
    );
    Ok(())
}

async fn run_shell(state: &State, site_id: &str, cmd: &str) -> Result<(), anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let cache = state.cache.as_ref().context("cache unavailable")?;
    let site = cache.get_site(site_id).await?.context("site not found")?;
    let cid = site.container_id.context("site has no container")?;
    let exec = docker
        .create_exec(
            &cid,
            crate::docker::ExecCreateRequest {
                cmd: vec!["/bin/sh".into(), "-c".into(), cmd.into()],
                tty: false,
            },
        )
        .await?;
    let _ = docker.attach_exec(&exec.exec_id, false).await?;
    Ok(())
}
