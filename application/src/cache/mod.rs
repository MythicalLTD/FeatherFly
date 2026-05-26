use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqliteConnectOptions};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteRecord {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub egg_id: String,
    pub container_id: Option<String>,
    pub network: Option<String>,
    pub volume: Option<String>,
    pub git_repo: Option<String>,
    pub memory_mb: Option<i64>,
    pub cpu_quota: Option<i64>,
    pub disk_quota_mb: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
    #[serde(default = "site_status_active_default")]
    pub status: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub document_root: Option<String>,
    #[serde(default = "site_type_custom_default")]
    pub site_type: String,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub redirects: Vec<crate::sites::SiteRedirect>,
    #[serde(default)]
    pub preview_domain: Option<String>,
    #[serde(default)]
    pub wildcard_domain: bool,
    pub created_at: i64,
}

fn site_status_active_default() -> String {
    crate::sites::SITE_STATUS_ACTIVE.into()
}

fn site_type_custom_default() -> String {
    crate::sites::SiteType::Custom.as_str().into()
}

#[derive(Debug, FromRow)]
struct SiteRow {
    id: String,
    name: String,
    domain: String,
    egg_id: Option<String>,
    template: Option<String>,
    container_id: Option<String>,
    network: Option<String>,
    volume: Option<String>,
    git_repo: Option<String>,
    memory_mb: Option<i64>,
    cpu_quota: Option<i64>,
    disk_quota_mb: Option<i64>,
    bandwidth_mbps: Option<i64>,
    status: Option<String>,
    domains: Option<String>,
    document_root: Option<String>,
    site_type: Option<String>,
    variables: Option<String>,
    redirects: Option<String>,
    preview_domain: Option<String>,
    wildcard_domain: Option<i64>,
    created_at: i64,
}

impl From<SiteRow> for SiteRecord {
    fn from(row: SiteRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            domain: row.domain,
            egg_id: row
                .egg_id
                .or(row.template)
                .unwrap_or_else(|| "unknown".into()),
            container_id: row.container_id,
            network: row.network,
            volume: row.volume,
            git_repo: row.git_repo,
            memory_mb: row.memory_mb,
            cpu_quota: row.cpu_quota,
            disk_quota_mb: row.disk_quota_mb,
            bandwidth_mbps: row.bandwidth_mbps,
            status: row
                .status
                .unwrap_or_else(|| crate::sites::SITE_STATUS_ACTIVE.into()),
            domains: row
                .domains
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            document_root: row.document_root,
            site_type: row
                .site_type
                .unwrap_or_else(|| crate::sites::SiteType::Custom.as_str().into()),
            variables: row
                .variables
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            redirects: row
                .redirects
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            preview_domain: row.preview_domain,
            wildcard_domain: row.wildcard_domain.is_some_and(|v| v != 0),
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DatabaseRecord {
    pub id: String,
    pub site_id: String,
    pub engine: String,
    pub container_id: Option<String>,
    pub volume: Option<String>,
    pub username: Option<String>,
    pub database_name: Option<String>,
    pub host: String,
    pub port: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaskRecord {
    pub id: String,
    pub site_id: String,
    pub name: String,
    pub kind: String,
    pub schedule: Option<String>,
    pub run_at: Option<i64>,
    pub action: String,
    pub command: Option<String>,
    pub enabled: i64,
    pub last_run_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DnsRecordRow {
    pub id: String,
    pub site_id: String,
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BackupRecord {
    pub id: String,
    pub site_id: String,
    pub path: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MailAccountRecord {
    pub id: String,
    pub site_id: String,
    pub address: String,
    pub password: String,
    pub quota_mb: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MailMetaRecord {
    pub site_id: String,
    pub mailserver_container_id: Option<String>,
    pub mailserver_volume: Option<String>,
    pub roundcube_container_id: Option<String>,
    pub mail_domain: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FtpAccountRecord {
    pub id: String,
    pub site_id: String,
    pub username: String,
    pub password: String,
    pub protocol: String,
    pub home_subpath: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FtpMetaRecord {
    pub site_id: String,
    pub sftp_container_id: Option<String>,
    pub ftp_container_id: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AuditEventRecord {
    pub id: String,
    pub created_at: i64,
    pub request_id: Option<String>,
    pub actor: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub client_ip: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<i64>,
    pub success: i64,
    pub message: Option<String>,
    pub metadata_json: Option<String>,
}

pub struct Cache {
    pool: SqlitePool,
}

impl Cache {
    pub async fn open(data_dir: &str) -> Result<Self, anyhow::Error> {
        let cache_dir = Path::new(data_dir).join("cache");
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("failed to create cache directory {}", cache_dir.display()))?;

        let db_path = cache_dir.join("featherfly.db");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .with_context(|| format!("failed to open sqlite cache at {}", db_path.display()))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS container_state (
                id TEXT PRIMARY KEY,
                last_seen INTEGER NOT NULL,
                panel_correlation_id TEXT
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate container_state table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS deployments (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                panel_correlation_id TEXT
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate deployments table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS panel_messages (
                correlation_id TEXT PRIMARY KEY,
                handled_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate panel_messages table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sites (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                domain TEXT NOT NULL,
                template TEXT NOT NULL,
                container_id TEXT,
                network TEXT,
                volume TEXT,
                git_repo TEXT,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate sites table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_backups (
                id TEXT PRIMARY KEY,
                site_id TEXT NOT NULL,
                path TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_backups table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_databases (
                id TEXT PRIMARY KEY,
                site_id TEXT NOT NULL,
                engine TEXT NOT NULL,
                container_id TEXT,
                volume TEXT,
                username TEXT,
                database_name TEXT,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_databases table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_tasks (
                id TEXT PRIMARY KEY,
                site_id TEXT NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                schedule TEXT,
                run_at INTEGER,
                action TEXT NOT NULL,
                command TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run_at INTEGER,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_tasks table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_dns (
                id TEXT PRIMARY KEY,
                site_id TEXT NOT NULL,
                name TEXT NOT NULL,
                record_type TEXT NOT NULL,
                value TEXT NOT NULL,
                ttl INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_dns table")?;

        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN memory_mb INTEGER")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN cpu_quota INTEGER")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN disk_quota_mb INTEGER")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN bandwidth_mbps INTEGER")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN status TEXT NOT NULL DEFAULT 'active'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN domains TEXT NOT NULL DEFAULT '[]'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN document_root TEXT")
            .execute(&pool)
            .await;
        let _ =
            sqlx::query("ALTER TABLE sites ADD COLUMN site_type TEXT NOT NULL DEFAULT 'custom'")
                .execute(&pool)
                .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN egg_id TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("UPDATE sites SET egg_id = template WHERE egg_id IS NULL")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN variables TEXT NOT NULL DEFAULT '{}'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN redirects TEXT NOT NULL DEFAULT '[]'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE sites ADD COLUMN preview_domain TEXT")
            .execute(&pool)
            .await;
        let _ =
            sqlx::query("ALTER TABLE sites ADD COLUMN wildcard_domain INTEGER NOT NULL DEFAULT 0")
                .execute(&pool)
                .await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_mail_accounts (
                id TEXT PRIMARY KEY,
                site_id TEXT NOT NULL,
                address TEXT NOT NULL,
                password TEXT NOT NULL,
                quota_mb INTEGER,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_mail_accounts table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_mail_meta (
                site_id TEXT PRIMARY KEY,
                mailserver_container_id TEXT,
                mailserver_volume TEXT,
                roundcube_container_id TEXT,
                mail_domain TEXT,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_mail_meta table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_ftp_accounts (
                id TEXT PRIMARY KEY,
                site_id TEXT NOT NULL,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                protocol TEXT NOT NULL,
                home_subpath TEXT NOT NULL DEFAULT 'files',
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_ftp_accounts table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS site_ftp_meta (
                site_id TEXT PRIMARY KEY,
                sftp_container_id TEXT,
                ftp_container_id TEXT,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate site_ftp_meta table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                request_id TEXT,
                actor TEXT,
                action TEXT NOT NULL,
                resource_type TEXT,
                resource_id TEXT,
                client_ip TEXT,
                method TEXT,
                path TEXT,
                status INTEGER,
                success INTEGER NOT NULL,
                message TEXT,
                metadata_json TEXT
            )",
        )
        .execute(&pool)
        .await
        .context("failed to migrate audit_events table")?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), anyhow::Error> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .context("sqlite cache ping failed")?;
        Ok(())
    }

    pub async fn save_site(&self, site: &SiteRecord) -> Result<(), anyhow::Error> {
        let domains = serde_json::to_string(&site.domains).context("failed to encode domains")?;
        let variables =
            serde_json::to_string(&site.variables).context("failed to encode variables")?;
        let redirects =
            serde_json::to_string(&site.redirects).context("failed to encode redirects")?;
        let wildcard = i64::from(site.wildcard_domain);
        sqlx::query(
            "INSERT INTO sites (id, name, domain, template, egg_id, container_id, network, volume, git_repo, memory_mb, cpu_quota, disk_quota_mb, bandwidth_mbps, status, domains, document_root, site_type, variables, redirects, preview_domain, wildcard_domain, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, domain=excluded.domain, template=excluded.template,
               egg_id=excluded.egg_id,
               container_id=excluded.container_id, network=excluded.network,
               volume=excluded.volume, git_repo=excluded.git_repo,
               memory_mb=excluded.memory_mb, cpu_quota=excluded.cpu_quota,
               disk_quota_mb=excluded.disk_quota_mb, bandwidth_mbps=excluded.bandwidth_mbps,
               status=excluded.status, domains=excluded.domains,
               document_root=excluded.document_root, site_type=excluded.site_type,
               variables=excluded.variables, redirects=excluded.redirects,
               preview_domain=excluded.preview_domain, wildcard_domain=excluded.wildcard_domain",
        )
        .bind(&site.id)
        .bind(&site.name)
        .bind(&site.domain)
        .bind(&site.egg_id)
        .bind(&site.egg_id)
        .bind(&site.container_id)
        .bind(&site.network)
        .bind(&site.volume)
        .bind(&site.git_repo)
        .bind(site.memory_mb)
        .bind(site.cpu_quota)
        .bind(site.disk_quota_mb)
        .bind(site.bandwidth_mbps)
        .bind(&site.status)
        .bind(&domains)
        .bind(&site.document_root)
        .bind(&site.site_type)
        .bind(&variables)
        .bind(&redirects)
        .bind(&site.preview_domain)
        .bind(wildcard)
        .bind(site.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save site")?;
        Ok(())
    }

    pub async fn get_site(&self, id: &str) -> Result<Option<SiteRecord>, anyhow::Error> {
        sqlx::query_as::<_, SiteRow>(
            "SELECT id, name, domain, egg_id, template, container_id, network, volume, git_repo, memory_mb, cpu_quota, disk_quota_mb, bandwidth_mbps, status, domains, document_root, site_type, variables, redirects, preview_domain, wildcard_domain, created_at
             FROM sites WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get site")
        .map(|row| row.map(SiteRecord::from))
    }

    pub async fn list_sites(&self) -> Result<Vec<SiteRecord>, anyhow::Error> {
        sqlx::query_as::<_, SiteRow>(
            "SELECT id, name, domain, egg_id, template, container_id, network, volume, git_repo, memory_mb, cpu_quota, disk_quota_mb, bandwidth_mbps, status, domains, document_root, site_type, variables, redirects, preview_domain, wildcard_domain, created_at
             FROM sites ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list sites")
        .map(|rows| rows.into_iter().map(SiteRecord::from).collect())
    }

    pub async fn delete_site(&self, id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM sites WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("failed to delete site")?;
        Ok(())
    }

    pub async fn save_backup(&self, backup: &BackupRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_backups (id, site_id, path, created_at) VALUES (?1,?2,?3,?4)",
        )
        .bind(&backup.id)
        .bind(&backup.site_id)
        .bind(&backup.path)
        .bind(backup.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save backup")?;
        Ok(())
    }

    pub async fn list_backups(&self, site_id: &str) -> Result<Vec<BackupRecord>, anyhow::Error> {
        sqlx::query_as::<_, BackupRecord>(
            "SELECT id, site_id, path, created_at FROM site_backups WHERE site_id = ?1 ORDER BY created_at DESC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list backups")
    }

    pub async fn get_backup(&self, id: &str) -> Result<Option<BackupRecord>, anyhow::Error> {
        sqlx::query_as::<_, BackupRecord>(
            "SELECT id, site_id, path, created_at FROM site_backups WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get backup")
    }

    pub async fn record_container_seen(
        &self,
        id: &str,
        correlation_id: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO container_state (id, last_seen, panel_correlation_id)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
               last_seen = excluded.last_seen,
               panel_correlation_id = COALESCE(excluded.panel_correlation_id, container_state.panel_correlation_id)",
        )
        .bind(id)
        .bind(now)
        .bind(correlation_id)
        .execute(&self.pool)
        .await
        .context("failed to upsert container_state")?;
        Ok(())
    }

    pub async fn panel_message_seen(&self, correlation_id: &str) -> Result<bool, anyhow::Error> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT handled_at FROM panel_messages WHERE correlation_id = ?1")
                .bind(correlation_id)
                .fetch_optional(&self.pool)
                .await
                .context("failed to query panel_messages")?;
        Ok(row.is_some())
    }

    pub async fn mark_panel_message_handled(
        &self,
        correlation_id: &str,
    ) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT OR IGNORE INTO panel_messages (correlation_id, handled_at) VALUES (?1, ?2)",
        )
        .bind(correlation_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to insert panel_messages")?;
        Ok(())
    }

    pub async fn save_database(&self, db: &DatabaseRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_databases (id, site_id, engine, container_id, volume, username, database_name, host, port, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        )
        .bind(&db.id)
        .bind(&db.site_id)
        .bind(&db.engine)
        .bind(&db.container_id)
        .bind(&db.volume)
        .bind(&db.username)
        .bind(&db.database_name)
        .bind(&db.host)
        .bind(db.port)
        .bind(db.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save database")?;
        Ok(())
    }

    pub async fn list_databases(
        &self,
        site_id: &str,
    ) -> Result<Vec<DatabaseRecord>, anyhow::Error> {
        sqlx::query_as::<_, DatabaseRecord>(
            "SELECT id, site_id, engine, container_id, volume, username, database_name, host, port, created_at
             FROM site_databases WHERE site_id = ?1 ORDER BY created_at DESC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list databases")
    }

    pub async fn list_all_databases(&self) -> Result<Vec<DatabaseRecord>, anyhow::Error> {
        sqlx::query_as::<_, DatabaseRecord>(
            "SELECT id, site_id, engine, container_id, volume, username, database_name, host, port, created_at
             FROM site_databases ORDER BY site_id ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list all databases")
    }

    pub async fn get_database(
        &self,
        site_id: &str,
        id: &str,
    ) -> Result<Option<DatabaseRecord>, anyhow::Error> {
        sqlx::query_as::<_, DatabaseRecord>(
            "SELECT id, site_id, engine, container_id, volume, username, database_name, host, port, created_at
             FROM site_databases WHERE site_id = ?1 AND id = ?2",
        )
        .bind(site_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get database")
    }

    pub async fn delete_database(&self, site_id: &str, id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_databases WHERE site_id = ?1 AND id = ?2")
            .bind(site_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("failed to delete database")?;
        Ok(())
    }

    pub async fn delete_databases_for_site(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_databases WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete site databases")?;
        Ok(())
    }

    pub async fn save_task(&self, task: &TaskRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_tasks (id, site_id, name, kind, schedule, run_at, action, command, enabled, last_run_at, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, kind=excluded.kind, schedule=excluded.schedule,
               run_at=excluded.run_at, action=excluded.action, command=excluded.command,
               enabled=excluded.enabled",
        )
        .bind(&task.id)
        .bind(&task.site_id)
        .bind(&task.name)
        .bind(&task.kind)
        .bind(&task.schedule)
        .bind(task.run_at)
        .bind(&task.action)
        .bind(&task.command)
        .bind(task.enabled)
        .bind(task.last_run_at)
        .bind(task.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save task")?;
        Ok(())
    }

    pub async fn list_tasks(&self, site_id: &str) -> Result<Vec<TaskRecord>, anyhow::Error> {
        sqlx::query_as::<_, TaskRecord>(
            "SELECT id, site_id, name, kind, schedule, run_at, action, command, enabled, last_run_at, created_at
             FROM site_tasks WHERE site_id = ?1 ORDER BY created_at DESC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list tasks")
    }

    pub async fn list_enabled_tasks(&self) -> Result<Vec<TaskRecord>, anyhow::Error> {
        sqlx::query_as::<_, TaskRecord>(
            "SELECT id, site_id, name, kind, schedule, run_at, action, command, enabled, last_run_at, created_at
             FROM site_tasks WHERE enabled = 1",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list enabled tasks")
    }

    pub async fn get_task(
        &self,
        site_id: &str,
        id: &str,
    ) -> Result<Option<TaskRecord>, anyhow::Error> {
        sqlx::query_as::<_, TaskRecord>(
            "SELECT id, site_id, name, kind, schedule, run_at, action, command, enabled, last_run_at, created_at
             FROM site_tasks WHERE site_id = ?1 AND id = ?2",
        )
        .bind(site_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get task")
    }

    pub async fn delete_task(&self, site_id: &str, id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_tasks WHERE site_id = ?1 AND id = ?2")
            .bind(site_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("failed to delete task")?;
        Ok(())
    }

    pub async fn delete_tasks_for_site(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_tasks WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete site tasks")?;
        Ok(())
    }

    pub async fn mark_task_run(&self, site_id: &str, id: &str) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE site_tasks SET last_run_at = ?1 WHERE site_id = ?2 AND id = ?3")
            .bind(now)
            .bind(site_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("failed to mark task run")?;
        Ok(())
    }

    pub async fn replace_dns(
        &self,
        site_id: &str,
        rows: &[DnsRecordRow],
    ) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_dns WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to clear dns records")?;
        for row in rows {
            sqlx::query(
                "INSERT INTO site_dns (id, site_id, name, record_type, value, ttl, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
            )
            .bind(&row.id)
            .bind(&row.site_id)
            .bind(&row.name)
            .bind(&row.record_type)
            .bind(&row.value)
            .bind(row.ttl)
            .bind(row.created_at)
            .execute(&self.pool)
            .await
            .context("failed to insert dns record")?;
        }
        Ok(())
    }

    pub async fn list_dns(&self, site_id: &str) -> Result<Vec<DnsRecordRow>, anyhow::Error> {
        sqlx::query_as::<_, DnsRecordRow>(
            "SELECT id, site_id, name, record_type, value, ttl, created_at
             FROM site_dns WHERE site_id = ?1 ORDER BY name ASC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list dns records")
    }

    pub async fn delete_dns_for_site(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_dns WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete site dns")?;
        Ok(())
    }

    pub async fn save_mail_account(&self, row: &MailAccountRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_mail_accounts (id, site_id, address, password, quota_mb, created_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET
               address=excluded.address, password=excluded.password, quota_mb=excluded.quota_mb",
        )
        .bind(&row.id)
        .bind(&row.site_id)
        .bind(&row.address)
        .bind(&row.password)
        .bind(row.quota_mb)
        .bind(row.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save mail account")?;
        Ok(())
    }

    pub async fn get_mail_account(
        &self,
        site_id: &str,
        id: &str,
    ) -> Result<Option<MailAccountRecord>, anyhow::Error> {
        sqlx::query_as::<_, MailAccountRecord>(
            "SELECT id, site_id, address, password, quota_mb, created_at
             FROM site_mail_accounts WHERE site_id = ?1 AND id = ?2",
        )
        .bind(site_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get mail account")
    }

    pub async fn list_mail_accounts(
        &self,
        site_id: &str,
    ) -> Result<Vec<MailAccountRecord>, anyhow::Error> {
        sqlx::query_as::<_, MailAccountRecord>(
            "SELECT id, site_id, address, password, quota_mb, created_at
             FROM site_mail_accounts WHERE site_id = ?1 ORDER BY address ASC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list mail accounts")
    }

    pub async fn list_all_mail_accounts(&self) -> Result<Vec<MailAccountRecord>, anyhow::Error> {
        sqlx::query_as::<_, MailAccountRecord>(
            "SELECT id, site_id, address, password, quota_mb, created_at
             FROM site_mail_accounts ORDER BY site_id ASC, address ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list all mail accounts")
    }

    pub async fn delete_mail_account(&self, site_id: &str, id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_mail_accounts WHERE site_id = ?1 AND id = ?2")
            .bind(site_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("failed to delete mail account")?;
        Ok(())
    }

    pub async fn delete_mail_accounts_for_site(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_mail_accounts WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete site mail accounts")?;
        Ok(())
    }

    pub async fn save_mail_meta(&self, row: &MailMetaRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_mail_meta (site_id, mailserver_container_id, mailserver_volume, roundcube_container_id, mail_domain, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(site_id) DO UPDATE SET
               mailserver_container_id=excluded.mailserver_container_id,
               mailserver_volume=excluded.mailserver_volume,
               roundcube_container_id=excluded.roundcube_container_id,
               mail_domain=excluded.mail_domain,
               updated_at=excluded.updated_at",
        )
        .bind(&row.site_id)
        .bind(&row.mailserver_container_id)
        .bind(&row.mailserver_volume)
        .bind(&row.roundcube_container_id)
        .bind(&row.mail_domain)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await
        .context("failed to save mail meta")?;
        Ok(())
    }

    pub async fn get_mail_meta(
        &self,
        site_id: &str,
    ) -> Result<Option<MailMetaRecord>, anyhow::Error> {
        sqlx::query_as::<_, MailMetaRecord>(
            "SELECT site_id, mailserver_container_id, mailserver_volume, roundcube_container_id, mail_domain, updated_at
             FROM site_mail_meta WHERE site_id = ?1",
        )
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get mail meta")
    }

    pub async fn delete_mail_meta(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_mail_meta WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete mail meta")?;
        Ok(())
    }

    pub async fn save_ftp_account(&self, row: &FtpAccountRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_ftp_accounts (id, site_id, username, password, protocol, home_subpath, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET
               username=excluded.username, password=excluded.password,
               protocol=excluded.protocol, home_subpath=excluded.home_subpath",
        )
        .bind(&row.id)
        .bind(&row.site_id)
        .bind(&row.username)
        .bind(&row.password)
        .bind(&row.protocol)
        .bind(&row.home_subpath)
        .bind(row.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save ftp account")?;
        Ok(())
    }

    pub async fn get_ftp_account(
        &self,
        site_id: &str,
        id: &str,
    ) -> Result<Option<FtpAccountRecord>, anyhow::Error> {
        sqlx::query_as::<_, FtpAccountRecord>(
            "SELECT id, site_id, username, password, protocol, home_subpath, created_at
             FROM site_ftp_accounts WHERE site_id = ?1 AND id = ?2",
        )
        .bind(site_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get ftp account")
    }

    pub async fn list_ftp_accounts(
        &self,
        site_id: &str,
    ) -> Result<Vec<FtpAccountRecord>, anyhow::Error> {
        sqlx::query_as::<_, FtpAccountRecord>(
            "SELECT id, site_id, username, password, protocol, home_subpath, created_at
             FROM site_ftp_accounts WHERE site_id = ?1 ORDER BY username ASC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list ftp accounts")
    }

    pub async fn list_all_ftp_accounts(&self) -> Result<Vec<FtpAccountRecord>, anyhow::Error> {
        sqlx::query_as::<_, FtpAccountRecord>(
            "SELECT id, site_id, username, password, protocol, home_subpath, created_at
             FROM site_ftp_accounts ORDER BY site_id ASC, username ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list all ftp accounts")
    }

    pub async fn delete_ftp_account(&self, site_id: &str, id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_ftp_accounts WHERE site_id = ?1 AND id = ?2")
            .bind(site_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("failed to delete ftp account")?;
        Ok(())
    }

    pub async fn delete_ftp_accounts_for_site(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_ftp_accounts WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete site ftp accounts")?;
        Ok(())
    }

    pub async fn save_ftp_meta(&self, row: &FtpMetaRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO site_ftp_meta (site_id, sftp_container_id, ftp_container_id, updated_at)
             VALUES (?1,?2,?3,?4)
             ON CONFLICT(site_id) DO UPDATE SET
               sftp_container_id=excluded.sftp_container_id,
               ftp_container_id=excluded.ftp_container_id,
               updated_at=excluded.updated_at",
        )
        .bind(&row.site_id)
        .bind(&row.sftp_container_id)
        .bind(&row.ftp_container_id)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await
        .context("failed to save ftp meta")?;
        Ok(())
    }

    pub async fn get_ftp_meta(
        &self,
        site_id: &str,
    ) -> Result<Option<FtpMetaRecord>, anyhow::Error> {
        sqlx::query_as::<_, FtpMetaRecord>(
            "SELECT site_id, sftp_container_id, ftp_container_id, updated_at
             FROM site_ftp_meta WHERE site_id = ?1",
        )
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get ftp meta")
    }

    pub async fn delete_ftp_meta(&self, site_id: &str) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM site_ftp_meta WHERE site_id = ?1")
            .bind(site_id)
            .execute(&self.pool)
            .await
            .context("failed to delete ftp meta")?;
        Ok(())
    }

    pub async fn save_audit_event(&self, event: &AuditEventRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO audit_events (
                id, created_at, request_id, actor, action, resource_type, resource_id,
                client_ip, method, path, status, success, message, metadata_json
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        )
        .bind(&event.id)
        .bind(event.created_at)
        .bind(&event.request_id)
        .bind(&event.actor)
        .bind(&event.action)
        .bind(&event.resource_type)
        .bind(&event.resource_id)
        .bind(&event.client_ip)
        .bind(&event.method)
        .bind(&event.path)
        .bind(event.status)
        .bind(event.success)
        .bind(&event.message)
        .bind(&event.metadata_json)
        .execute(&self.pool)
        .await
        .context("failed to save audit event")?;
        Ok(())
    }

    pub async fn list_audit_events(
        &self,
        limit: i64,
    ) -> Result<Vec<AuditEventRecord>, anyhow::Error> {
        let limit = limit.clamp(1, 500);
        sqlx::query_as::<_, AuditEventRecord>(
            "SELECT id, created_at, request_id, actor, action, resource_type, resource_id,
                    client_ip, method, path, status, success, message, metadata_json
             FROM audit_events
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("failed to list audit events")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_events_persist_and_list_newest_first() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let cache = Cache::open(dir.path().to_str().unwrap()).await.unwrap();

            cache
                .save_audit_event(&AuditEventRecord {
                    id: "old".into(),
                    created_at: 1,
                    request_id: Some("req-old".into()),
                    actor: None,
                    action: "site.created".into(),
                    resource_type: Some("site".into()),
                    resource_id: Some("demo-old".into()),
                    client_ip: Some("127.0.0.1".into()),
                    method: Some("POST".into()),
                    path: Some("/api/sites".into()),
                    status: Some(201),
                    success: 1,
                    message: None,
                    metadata_json: Some(r#"{"domain":"old.example"}"#.into()),
                })
                .await
                .unwrap();

            cache
                .save_audit_event(&AuditEventRecord {
                    id: "new".into(),
                    created_at: 2,
                    request_id: Some("req-new".into()),
                    actor: None,
                    action: "site.destroyed".into(),
                    resource_type: Some("site".into()),
                    resource_id: Some("demo-new".into()),
                    client_ip: Some("127.0.0.1".into()),
                    method: Some("DELETE".into()),
                    path: Some("/api/sites/demo-new".into()),
                    status: Some(200),
                    success: 1,
                    message: Some("deleted".into()),
                    metadata_json: Some(r#"{"force":true}"#.into()),
                })
                .await
                .unwrap();

            let events = cache.list_audit_events(10).await.unwrap();
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].id, "new");
            assert_eq!(events[0].action, "site.destroyed");
            assert_eq!(events[1].id, "old");

            let limited = cache.list_audit_events(1).await.unwrap();
            assert_eq!(limited.len(), 1);
            assert_eq!(limited[0].id, "new");
        });
    }
}
