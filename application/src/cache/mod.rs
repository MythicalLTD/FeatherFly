use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqliteConnectOptions};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SiteRecord {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub template: String,
    pub container_id: Option<String>,
    pub network: Option<String>,
    pub volume: Option<String>,
    pub git_repo: Option<String>,
    pub memory_mb: Option<i64>,
    pub cpu_quota: Option<i64>,
    pub disk_quota_mb: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
    pub created_at: i64,
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

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn save_site(&self, site: &SiteRecord) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO sites (id, name, domain, template, container_id, network, volume, git_repo, memory_mb, cpu_quota, disk_quota_mb, bandwidth_mbps, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, domain=excluded.domain, template=excluded.template,
               container_id=excluded.container_id, network=excluded.network,
               volume=excluded.volume, git_repo=excluded.git_repo,
               memory_mb=excluded.memory_mb, cpu_quota=excluded.cpu_quota,
               disk_quota_mb=excluded.disk_quota_mb, bandwidth_mbps=excluded.bandwidth_mbps",
        )
        .bind(&site.id)
        .bind(&site.name)
        .bind(&site.domain)
        .bind(&site.template)
        .bind(&site.container_id)
        .bind(&site.network)
        .bind(&site.volume)
        .bind(&site.git_repo)
        .bind(site.memory_mb)
        .bind(site.cpu_quota)
        .bind(site.disk_quota_mb)
        .bind(site.bandwidth_mbps)
        .bind(site.created_at)
        .execute(&self.pool)
        .await
        .context("failed to save site")?;
        Ok(())
    }

    pub async fn get_site(&self, id: &str) -> Result<Option<SiteRecord>, anyhow::Error> {
        sqlx::query_as::<_, SiteRecord>(
            "SELECT id, name, domain, template, container_id, network, volume, git_repo, memory_mb, cpu_quota, disk_quota_mb, bandwidth_mbps, created_at
             FROM sites WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to get site")
    }

    pub async fn list_sites(&self) -> Result<Vec<SiteRecord>, anyhow::Error> {
        sqlx::query_as::<_, SiteRecord>(
            "SELECT id, name, domain, template, container_id, network, volume, git_repo, memory_mb, cpu_quota, disk_quota_mb, bandwidth_mbps, created_at
             FROM sites ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list sites")
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
}
