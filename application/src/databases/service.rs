use std::collections::HashMap;

use anyhow::Context;
use bollard::container::{Config, CreateContainerOptions};
use bollard::models::{Mount, MountTypeEnum};
use featherfly_plugin_sdk::PluginEvent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    cache::DatabaseRecord,
    routes::State,
    utils::plugin_events::{self, DatabaseCreatedPayload, DatabaseRemovedPayload},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseEngine {
    Mysql,
    Postgres,
    Redis,
    Mongodb,
}

impl DatabaseEngine {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mysql => "mysql",
            Self::Postgres => "postgres",
            Self::Redis => "redis",
            Self::Mongodb => "mongodb",
        }
    }

    fn image(self) -> &'static str {
        match self {
            Self::Mysql => "mysql:8",
            Self::Postgres => "postgres:16-alpine",
            Self::Redis => "redis:7-alpine",
            Self::Mongodb => "mongo:7",
        }
    }

    pub fn default_port(self) -> i64 {
        match self {
            Self::Mysql => 3306,
            Self::Postgres => 5432,
            Self::Redis => 6379,
            Self::Mongodb => 27017,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateDatabaseRequest {
    pub id: String,
    pub engine: DatabaseEngine,
    #[serde(default)]
    pub username: Option<String>,
    pub password: String,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub memory_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DatabaseSummary {
    pub id: String,
    pub site_id: String,
    pub engine: String,
    pub host: String,
    pub port: i64,
    pub username: Option<String>,
    pub database: Option<String>,
    pub container_id: Option<String>,
}

pub struct DatabaseService;

impl DatabaseService {
    pub async fn create(
        state: &State,
        site_id: &str,
        req: CreateDatabaseRequest,
    ) -> Result<DatabaseSummary, anyhow::Error> {
        let inner = state.config.load();
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let site = cache.get_site(site_id).await?.context("site not found")?;

        if inner.hosting.shared_services {
            let (host, db_name, port, container_id) =
                crate::hosting::shared_db::provision_shared_database(
                    state,
                    site_id,
                    req.engine,
                    &req.id,
                    req.username.as_deref(),
                    &req.password,
                    req.database.as_deref(),
                )
                .await?;

            let user = crate::hosting::shared_db::shared_db_user(
                site_id,
                req.username.as_deref(),
                &req.id,
            );

            let record = DatabaseRecord {
                id: req.id.clone(),
                site_id: site_id.to_string(),
                engine: req.engine.as_str().into(),
                container_id: Some(container_id),
                volume: None,
                username: Some(user),
                database_name: Some(db_name),
                host,
                port,
                created_at: chrono::Utc::now().timestamp(),
            };
            cache.save_database(&record).await?;

            plugin_events::emit_state_event(
                state,
                PluginEvent::DatabaseCreated,
                &DatabaseCreatedPayload {
                    site_id,
                    database_id: &req.id,
                    engine: &record.engine,
                    host: &record.host,
                    port,
                },
            );

            return Ok(record_to_summary(&record));
        }

        let docker = state.docker.as_ref().context("docker unavailable")?;
        let network = site.network.as_ref().context("site has no network")?;

        let volume_name = format!("db-vol-{}-{}", site_id, req.id);
        docker.create_volume(&volume_name).await?;

        let container_name = format!("db-{}-{}", site_id, req.id);
        let host = container_name.clone();
        let port = req.engine.default_port();

        let (env, db_name) = build_db_env(req.engine, &req, site_id);
        let memory_mb = req.memory_mb.or(site.memory_mb.map(|m| m as u64));

        let host_config = crate::docker::secure_host_config(
            network,
            inner.docker.container_pid_limit,
            memory_mb,
            site.cpu_quota,
            HashMap::new(),
        );
        let mut host_config = host_config;
        host_config.mounts = Some(vec![Mount {
            typ: Some(MountTypeEnum::VOLUME),
            source: Some(volume_name.clone()),
            target: Some(data_mount(req.engine).to_string()),
            ..Default::default()
        }]);

        let mut labels = HashMap::new();
        labels.insert("featherfly.site.id".into(), site_id.to_string());
        labels.insert("featherfly.database.id".into(), req.id.clone());
        labels.insert(
            "featherfly.database.engine".into(),
            req.engine.as_str().into(),
        );

        let config = Config {
            image: Some(req.engine.image().into()),
            env: Some(env),
            labels: Some(labels),
            host_config: Some(host_config),
            ..Default::default()
        };

        let response = docker
            .client()
            .create_container(
                Some(CreateContainerOptions {
                    name: container_name.clone(),
                    platform: None,
                }),
                config,
            )
            .await
            .with_context(|| format!("failed to create database container {container_name}"))?;

        docker.start_container(&response.id).await?;

        let record = DatabaseRecord {
            id: req.id.clone(),
            site_id: site_id.to_string(),
            engine: req.engine.as_str().into(),
            container_id: Some(response.id.clone()),
            volume: Some(volume_name),
            username: req.username.clone(),
            database_name: db_name,
            host,
            port,
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.save_database(&record).await?;

        plugin_events::emit_state_event(
            state,
            PluginEvent::DatabaseCreated,
            &DatabaseCreatedPayload {
                site_id,
                database_id: &req.id,
                engine: &record.engine,
                host: &record.host,
                port,
            },
        );

        Ok(record_to_summary(&record))
    }

    pub async fn list(state: &State, site_id: &str) -> Result<Vec<DatabaseSummary>, anyhow::Error> {
        let cache = state.cache.as_ref().context("cache unavailable")?;
        Ok(cache
            .list_databases(site_id)
            .await?
            .iter()
            .map(record_to_summary)
            .collect())
    }

    pub async fn remove(
        state: &State,
        site_id: &str,
        db_id: &str,
        force: bool,
    ) -> Result<(), anyhow::Error> {
        let inner = state.config.load();
        let docker = state.docker.as_ref().context("docker unavailable")?;
        let cache = state.cache.as_ref().context("cache unavailable")?;
        let record = cache
            .get_database(site_id, db_id)
            .await?
            .context("database not found")?;

        if inner.hosting.shared_services {
            crate::hosting::shared_db::deprovision_shared_database(state, &record).await?;
        } else {
            if let Some(ref cid) = record.container_id {
                let _ = docker.stop_container(cid).await;
                docker.remove_container(cid, force).await?;
            }
            if let Some(ref vol) = record.volume {
                let _ = docker.remove_volume(vol, force).await;
            }
        }

        cache.delete_database(site_id, db_id).await?;
        plugin_events::emit_state_event(
            state,
            PluginEvent::DatabaseRemoved,
            &DatabaseRemovedPayload {
                site_id,
                database_id: db_id,
                engine: &record.engine,
            },
        );
        Ok(())
    }
}

fn data_mount(engine: DatabaseEngine) -> &'static str {
    match engine {
        DatabaseEngine::Mysql => "/var/lib/mysql",
        DatabaseEngine::Postgres => "/var/lib/postgresql/data",
        DatabaseEngine::Redis => "/data",
        DatabaseEngine::Mongodb => "/data/db",
    }
}

fn build_db_env(
    engine: DatabaseEngine,
    req: &CreateDatabaseRequest,
    site_id: &str,
) -> (Vec<String>, Option<String>) {
    match engine {
        DatabaseEngine::Mysql => {
            let db = req
                .database
                .clone()
                .unwrap_or_else(|| format!("site_{site_id}"));
            let user = req.username.clone().unwrap_or_else(|| "app".into());
            (
                vec![
                    "MYSQL_ROOT_PASSWORD=".to_string() + &req.password,
                    format!("MYSQL_DATABASE={db}"),
                    format!("MYSQL_USER={user}"),
                    format!("MYSQL_PASSWORD={}", req.password),
                ],
                Some(db),
            )
        }
        DatabaseEngine::Postgres => {
            let db = req
                .database
                .clone()
                .unwrap_or_else(|| format!("site_{site_id}"));
            let user = req.username.clone().unwrap_or_else(|| "app".into());
            (
                vec![
                    format!("POSTGRES_PASSWORD={}", req.password),
                    format!("POSTGRES_DB={db}"),
                    format!("POSTGRES_USER={user}"),
                ],
                Some(db),
            )
        }
        DatabaseEngine::Redis => (vec![format!("REDIS_PASSWORD={}", req.password)], None),
        DatabaseEngine::Mongodb => {
            let db = req.database.clone().unwrap_or_else(|| "app".into());
            (
                vec![
                    format!(
                        "MONGO_INITDB_ROOT_USERNAME={}",
                        req.username.clone().unwrap_or_else(|| "root".into())
                    ),
                    format!("MONGO_INITDB_ROOT_PASSWORD={}", req.password),
                    format!("MONGO_INITDB_DATABASE={db}"),
                ],
                Some(db),
            )
        }
    }
}

fn record_to_summary(record: &DatabaseRecord) -> DatabaseSummary {
    DatabaseSummary {
        id: record.id.clone(),
        site_id: record.site_id.clone(),
        engine: record.engine.clone(),
        host: record.host.clone(),
        port: record.port,
        username: record.username.clone(),
        database: record.database_name.clone(),
        container_id: record.container_id.clone(),
    }
}
