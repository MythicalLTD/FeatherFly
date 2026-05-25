use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use bollard::container::Config;
use bollard::models::{Mount, MountTypeEnum};

use crate::{
    cache::{DatabaseRecord, SiteRecord},
    config::InnerConfig,
    databases::DatabaseEngine,
    docker::{DockerManager, connect_container_network},
    routes::State,
    utils::plugin_events::{self, SharedDatabaseServersReadyPayload},
};
use featherfly_plugin_sdk::PluginEvent;

pub const SHARED_MYSQL: &str = "featherfly-mysql";
pub const SHARED_POSTGRES: &str = "featherfly-postgres";
pub const SHARED_REDIS: &str = "featherfly-redis";
pub const SHARED_MONGODB: &str = "featherfly-mongodb";

pub fn shared_db_host(engine: DatabaseEngine) -> &'static str {
    match engine {
        DatabaseEngine::Mysql => SHARED_MYSQL,
        DatabaseEngine::Postgres => SHARED_POSTGRES,
        DatabaseEngine::Redis => SHARED_REDIS,
        DatabaseEngine::Mongodb => SHARED_MONGODB,
    }
}

pub fn shared_db_logical_name(site_id: &str, db_id: &str, name: Option<&str>) -> String {
    name.map(|n| format!("{site_id}__{n}"))
        .unwrap_or_else(|| format!("{site_id}__{db_id}"))
}

pub fn shared_db_user(site_id: &str, username: Option<&str>, db_id: &str) -> String {
    crate::hosting::shared::shared_username(site_id, username.unwrap_or(db_id))
}

fn root_secret_path(inner: &InnerConfig) -> PathBuf {
    PathBuf::from(&inner.system.data).join("shared-db-root.secret")
}

pub fn ensure_database_root_password(inner: &InnerConfig) -> Result<String, anyhow::Error> {
    if !inner.hosting.database_root_password.is_empty() {
        return Ok(inner.hosting.database_root_password.clone());
    }
    let path = root_secret_path(inner);
    if path.exists() {
        return Ok(std::fs::read_to_string(&path)?.trim().to_string());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let secret = uuid::Uuid::new_v4().to_string();
    std::fs::write(&path, &secret)?;
    Ok(secret)
}

/// Start shared MySQL/Postgres/Redis/MongoDB on the proxy network.
pub async fn ensure_shared_database_servers(
    state: &State,
    inner: &InnerConfig,
    sites: &[SiteRecord],
) -> Result<(), anyhow::Error> {
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let root = ensure_database_root_password(inner)?;

    let mysql = ensure_mysql(docker, inner, &root).await?;
    let postgres = ensure_postgres(docker, inner, &root).await?;
    let redis = ensure_redis(docker, inner, &root).await?;
    let mongo = ensure_mongodb(docker, inner, &root).await?;

    let mut site_networks_connected = 0usize;
    for site in sites {
        if let Some(network) = &site.network {
            for cid in [&mysql, &postgres, &redis, &mongo] {
                if connect_container_network(docker.client(), network, cid)
                    .await
                    .is_ok()
                {
                    site_networks_connected += 1;
                }
            }
        }
    }

    plugin_events::emit_state_event(
        state,
        PluginEvent::SharedDatabaseServersReady,
        &SharedDatabaseServersReadyPayload {
            mysql: true,
            postgres: true,
            redis: true,
            mongodb: true,
            site_networks_connected,
        },
    );

    Ok(())
}

pub async fn provision_shared_database(
    state: &State,
    site_id: &str,
    engine: DatabaseEngine,
    db_id: &str,
    username: Option<&str>,
    password: &str,
    database: Option<&str>,
) -> Result<(String, String, i64, String), anyhow::Error> {
    let inner = state.config.load();
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let cache = state.cache.as_ref().context("cache unavailable")?;
    let sites = cache.list_sites().await?;
    ensure_shared_database_servers(state, &inner, &sites).await?;

    let root = ensure_database_root_password(&inner)?;
    let db_name = shared_db_logical_name(site_id, db_id, database);
    let user = shared_db_user(site_id, username, db_id);
    let host = shared_db_host(engine).to_string();
    let port = engine.default_port();

    let container_id = match engine {
        DatabaseEngine::Mysql => {
            provision_mysql(docker, SHARED_MYSQL, &root, &db_name, &user, password).await?;
            docker
                .inspect_container(SHARED_MYSQL)
                .await
                .ok()
                .and_then(|c| c.id)
                .unwrap_or_else(|| SHARED_MYSQL.to_string())
        }
        DatabaseEngine::Postgres => {
            provision_postgres(docker, SHARED_POSTGRES, &root, &db_name, &user, password).await?;
            SHARED_POSTGRES.to_string()
        }
        DatabaseEngine::Redis => {
            provision_redis(docker, SHARED_REDIS, &root, &user, password).await?;
            SHARED_REDIS.to_string()
        }
        DatabaseEngine::Mongodb => {
            provision_mongodb(docker, SHARED_MONGODB, &root, &db_name, &user, password).await?;
            SHARED_MONGODB.to_string()
        }
    };

    Ok((host, db_name, port, container_id))
}

pub async fn deprovision_shared_database(
    state: &State,
    record: &DatabaseRecord,
) -> Result<(), anyhow::Error> {
    let inner = state.config.load();
    if !inner.hosting.shared_services {
        return Ok(());
    }
    let docker = state.docker.as_ref().context("docker unavailable")?;
    let root = ensure_database_root_password(&inner)?;
    let user = record
        .username
        .clone()
        .unwrap_or_else(|| shared_db_user(&record.site_id, None, &record.id));
    let db_name = record
        .database_name
        .clone()
        .unwrap_or_else(|| shared_db_logical_name(&record.site_id, &record.id, None));

    match record.engine.as_str() {
        "mysql" => deprovision_mysql(docker, SHARED_MYSQL, &root, &db_name, &user)
            .await
            .ok(),
        "postgres" => deprovision_postgres(docker, SHARED_POSTGRES, &root, &db_name, &user)
            .await
            .ok(),
        "redis" => deprovision_redis(docker, SHARED_REDIS, &root, &user)
            .await
            .ok(),
        "mongodb" => deprovision_mongodb(docker, SHARED_MONGODB, &root, &db_name, &user)
            .await
            .ok(),
        _ => None,
    };
    Ok(())
}

async fn ensure_mysql(
    docker: &DockerManager,
    inner: &InnerConfig,
    root: &str,
) -> Result<String, anyhow::Error> {
    docker.create_volume("featherfly-mysql-data").await.ok();
    let ports = crate::hosting::mysql_port_bindings(&inner.hosting.ports);
    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        ports,
    );
    host_config.mounts = Some(vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some("featherfly-mysql-data".into()),
        target: Some("/var/lib/mysql".into()),
        ..Default::default()
    }]);

    let config = Config {
        image: Some(inner.hosting.mysql_image.clone()),
        env: Some(vec![format!("MYSQL_ROOT_PASSWORD={root}")]),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-mysql".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };
    crate::hosting::shared::ensure_named_public(docker, SHARED_MYSQL, config).await
}

async fn ensure_postgres(
    docker: &DockerManager,
    inner: &InnerConfig,
    root: &str,
) -> Result<String, anyhow::Error> {
    docker.create_volume("featherfly-postgres-data").await.ok();
    let ports = crate::hosting::postgres_port_bindings(&inner.hosting.ports);
    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        ports,
    );
    host_config.mounts = Some(vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some("featherfly-postgres-data".into()),
        target: Some("/var/lib/postgresql/data".into()),
        ..Default::default()
    }]);

    let config = Config {
        image: Some(inner.hosting.postgres_image.clone()),
        env: Some(vec![
            format!("POSTGRES_PASSWORD={root}"),
            "POSTGRES_USER=postgres".into(),
        ]),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-postgres".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };
    crate::hosting::shared::ensure_named_public(docker, SHARED_POSTGRES, config).await
}

async fn ensure_redis(
    docker: &DockerManager,
    inner: &InnerConfig,
    root: &str,
) -> Result<String, anyhow::Error> {
    docker.create_volume("featherfly-redis-data").await.ok();
    let ports = crate::hosting::redis_port_bindings(&inner.hosting.ports);
    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        ports,
    );
    host_config.mounts = Some(vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some("featherfly-redis-data".into()),
        target: Some("/data".into()),
        ..Default::default()
    }]);

    let config = Config {
        image: Some(inner.hosting.redis_image.clone()),
        cmd: Some(vec![
            "redis-server".into(),
            "--requirepass".into(),
            root.to_string(),
        ]),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-redis".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };
    crate::hosting::shared::ensure_named_public(docker, SHARED_REDIS, config).await
}

async fn ensure_mongodb(
    docker: &DockerManager,
    inner: &InnerConfig,
    root: &str,
) -> Result<String, anyhow::Error> {
    docker.create_volume("featherfly-mongodb-data").await.ok();
    let ports = crate::hosting::mongodb_port_bindings(&inner.hosting.ports);
    let mut host_config = crate::docker::secure_host_config(
        &inner.proxy.network,
        inner.docker.container_pid_limit,
        None,
        None,
        ports,
    );
    host_config.mounts = Some(vec![Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some("featherfly-mongodb-data".into()),
        target: Some("/data/db".into()),
        ..Default::default()
    }]);

    let config = Config {
        image: Some(inner.hosting.mongodb_image.clone()),
        env: Some(vec![
            "MONGO_INITDB_ROOT_USERNAME=root".into(),
            format!("MONGO_INITDB_ROOT_PASSWORD={root}"),
        ]),
        labels: Some(HashMap::from([(
            "featherfly.role".into(),
            "shared-mongodb".into(),
        )])),
        host_config: Some(host_config),
        ..Default::default()
    };
    crate::hosting::shared::ensure_named_public(docker, SHARED_MONGODB, config).await
}

fn sql_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "''")
}

async fn provision_mysql(
    docker: &DockerManager,
    container: &str,
    root: &str,
    db: &str,
    user: &str,
    pass: &str,
) -> Result<(), anyhow::Error> {
    let sql = format!(
        "CREATE DATABASE IF NOT EXISTS `{db}`; \
         CREATE USER IF NOT EXISTS '{user}'@'%' IDENTIFIED BY '{}'; \
         GRANT ALL PRIVILEGES ON `{db}`.* TO '{user}'@'%'; FLUSH PRIVILEGES;",
        sql_escape(pass)
    );
    run_sh(
        docker,
        container,
        &format!(
            "MYSQL_PWD='{}' mysql -uroot -e \"{}\"",
            sql_escape(root),
            sql.replace('"', "\\\"")
        ),
    )
    .await
}

async fn deprovision_mysql(
    docker: &DockerManager,
    container: &str,
    root: &str,
    db: &str,
    user: &str,
) -> Result<(), anyhow::Error> {
    let sql = format!("DROP DATABASE IF EXISTS `{db}`; DROP USER IF EXISTS '{user}'@'%';");
    run_sh(
        docker,
        container,
        &format!("MYSQL_PWD='{}' mysql -uroot -e \"{sql}\"", sql_escape(root)),
    )
    .await
}

async fn provision_postgres(
    docker: &DockerManager,
    container: &str,
    root: &str,
    db: &str,
    user: &str,
    pass: &str,
) -> Result<(), anyhow::Error> {
    let create_user = format!(
        "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '{user}') THEN \
         CREATE USER \"{user}\" WITH PASSWORD '{}'; END IF; END $$;",
        sql_escape(pass)
    );
    run_sh(
        docker,
        container,
        &format!(
            "PGPASSWORD='{}' psql -U postgres -c \"{create_user}\"",
            sql_escape(root)
        ),
    )
    .await?;
    let create_db = format!(
        "SELECT 'CREATE DATABASE \"{db}\" OWNER \"{user}\"' WHERE NOT EXISTS \
         (SELECT FROM pg_database WHERE datname = '{db}')\\gexec"
    );
    run_sh(
        docker,
        container,
        &format!(
            "PGPASSWORD='{}' psql -U postgres -c \"{create_db}\"",
            sql_escape(root)
        ),
    )
    .await
}

async fn deprovision_postgres(
    docker: &DockerManager,
    container: &str,
    root: &str,
    db: &str,
    user: &str,
) -> Result<(), anyhow::Error> {
    let sql = format!("DROP DATABASE IF EXISTS \"{db}\"; DROP USER IF EXISTS \"{user}\";");
    run_sh(
        docker,
        container,
        &format!(
            "PGPASSWORD='{}' psql -U postgres -c \"{sql}\"",
            sql_escape(root)
        ),
    )
    .await
}

async fn provision_redis(
    docker: &DockerManager,
    container: &str,
    root: &str,
    user: &str,
    pass: &str,
) -> Result<(), anyhow::Error> {
    run_sh(
        docker,
        container,
        &format!(
            "redis-cli -a '{}' ACL SETUSER {} on >{} ~* +@all",
            sql_escape(root),
            user,
            sql_escape(pass)
        ),
    )
    .await
}

async fn deprovision_redis(
    docker: &DockerManager,
    container: &str,
    root: &str,
    user: &str,
) -> Result<(), anyhow::Error> {
    run_sh(
        docker,
        container,
        &format!("redis-cli -a '{}' ACL DELUSER {}", sql_escape(root), user),
    )
    .await
}

async fn provision_mongodb(
    docker: &DockerManager,
    container: &str,
    root: &str,
    db: &str,
    user: &str,
    pass: &str,
) -> Result<(), anyhow::Error> {
    let script = format!(
        "db.getSiblingDB('{db}').createUser({{user:'{user}',pwd:'{}',roles:[{{role:'readWrite',db:'{db}'}}]}})",
        sql_escape(pass)
    );
    run_sh(
        docker,
        container,
        &format!(
            "mongosh -u root -p '{}' --authenticationDatabase admin --eval \"{script}\"",
            sql_escape(root)
        ),
    )
    .await
}

async fn deprovision_mongodb(
    docker: &DockerManager,
    container: &str,
    root: &str,
    db: &str,
    user: &str,
) -> Result<(), anyhow::Error> {
    let script = format!(
        "db.getSiblingDB('{db}').dropUser('{user}'); db.getSiblingDB('{db}').dropDatabase();"
    );
    run_sh(
        docker,
        container,
        &format!(
            "mongosh -u root -p '{}' --authenticationDatabase admin --eval \"{script}\"",
            sql_escape(root)
        ),
    )
    .await
}

async fn run_sh(
    docker: &DockerManager,
    container: &str,
    script: &str,
) -> Result<(), anyhow::Error> {
    docker
        .exec_run(container, vec!["sh".into(), "-c".into(), script.into()])
        .await
        .with_context(|| format!("exec failed in {container}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn shared_db_host_maps_all_engines() {
        assert_eq!(shared_db_host(DatabaseEngine::Mysql), SHARED_MYSQL);
        assert_eq!(shared_db_host(DatabaseEngine::Postgres), SHARED_POSTGRES);
        assert_eq!(shared_db_host(DatabaseEngine::Redis), SHARED_REDIS);
        assert_eq!(shared_db_host(DatabaseEngine::Mongodb), SHARED_MONGODB);
    }

    #[test]
    fn shared_db_logical_name_uses_custom_or_id() {
        assert_eq!(
            shared_db_logical_name("site1", "db1", Some("appdb")),
            "site1__appdb"
        );
        assert_eq!(shared_db_logical_name("site1", "db1", None), "site1__db1");
    }

    #[test]
    fn shared_db_user_scopes_username_to_site() {
        assert_eq!(shared_db_user("site1", Some("app"), "db1"), "site1__app");
        assert_eq!(shared_db_user("site1", None, "db1"), "site1__db1");
    }

    #[test]
    fn ensure_database_root_password_respects_config_value() {
        let inner = Config::parse_preview(
            br#"
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  data: ./data/volumes
  pid_file: ./featherfly.pid
hosting:
  database_root_password: configured-root
"#,
        )
        .unwrap();

        assert_eq!(
            ensure_database_root_password(&inner).unwrap(),
            "configured-root"
        );
    }

    #[test]
    fn ensure_database_root_password_generates_and_reuses_secret_file() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("volumes");
        std::fs::create_dir_all(&data).unwrap();
        let data = data.to_string_lossy();

        let inner = Config::parse_preview(
            format!(
                r#"
api:
  port: 9090
system:
  root_directory: ./data
  log_directory: ./logs
  tmp_directory: ./tmp
  data: {data}
  pid_file: ./featherfly.pid
hosting:
  database_root_password: ""
"#
            )
            .as_bytes(),
        )
        .unwrap();

        let first = ensure_database_root_password(&inner).unwrap();
        assert!(!first.is_empty());
        let second = ensure_database_root_password(&inner).unwrap();
        assert_eq!(first, second);
        assert!(root_secret_path(&inner).exists());
    }
}
