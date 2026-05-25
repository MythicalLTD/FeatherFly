use std::collections::HashMap;

use bollard::models::PortBinding;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::config::HostingPortsConfig;

pub fn mail_port_bindings(ports: &HostingPortsConfig) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 25, ports.smtp);
    bind(&mut map, 587, ports.submission);
    bind(&mut map, 465, ports.smtps);
    bind(&mut map, 143, ports.imap);
    bind(&mut map, 993, ports.imaps);
    bind(&mut map, 110, ports.pop3);
    bind(&mut map, 995, ports.pop3s);
    map
}

pub fn ftp_port_bindings(ports: &HostingPortsConfig) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 21, ports.ftp);
    for p in ports.ftp_passive_min..=ports.ftp_passive_max {
        bind(&mut map, p, p);
    }
    map
}

pub fn sftp_port_bindings(ports: &HostingPortsConfig) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 22, ports.sftp);
    map
}

pub fn mysql_port_bindings(
    ports: &HostingPortsConfig,
) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 3306, ports.mysql);
    map
}

pub fn postgres_port_bindings(
    ports: &HostingPortsConfig,
) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 5432, ports.postgres);
    map
}

pub fn redis_port_bindings(
    ports: &HostingPortsConfig,
) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 6379, ports.redis);
    map
}

pub fn mongodb_port_bindings(
    ports: &HostingPortsConfig,
) -> HashMap<String, Option<Vec<PortBinding>>> {
    if !ports.publish_service_ports {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    bind(&mut map, 27017, ports.mongodb);
    map
}

fn bind(map: &mut HashMap<String, Option<Vec<PortBinding>>>, container: u16, host: u16) {
    map.insert(
        format!("{container}/tcp"),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".into()),
            host_port: Some(host.to_string()),
        }]),
    );
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishedPort {
    pub service: String,
    pub protocol: String,
    pub host_port: u16,
    pub container_port: u16,
    pub tls: bool,
}

pub fn list_published_ports(ports: &HostingPortsConfig) -> Vec<PublishedPort> {
    if !ports.publish_service_ports {
        return Vec::new();
    }
    vec![
        PublishedPort {
            service: "web".into(),
            protocol: "tcp".into(),
            host_port: ports.http,
            container_port: 80,
            tls: false,
        },
        PublishedPort {
            service: "websecure".into(),
            protocol: "tcp".into(),
            host_port: ports.https,
            container_port: 443,
            tls: true,
        },
        PublishedPort {
            service: "sftp".into(),
            protocol: "tcp".into(),
            host_port: ports.sftp,
            container_port: 22,
            tls: false,
        },
        PublishedPort {
            service: "ftp".into(),
            protocol: "tcp".into(),
            host_port: ports.ftp,
            container_port: 21,
            tls: false,
        },
        PublishedPort {
            service: "smtp".into(),
            protocol: "tcp".into(),
            host_port: ports.smtp,
            container_port: 25,
            tls: false,
        },
        PublishedPort {
            service: "submission".into(),
            protocol: "tcp".into(),
            host_port: ports.submission,
            container_port: 587,
            tls: true,
        },
        PublishedPort {
            service: "smtps".into(),
            protocol: "tcp".into(),
            host_port: ports.smtps,
            container_port: 465,
            tls: true,
        },
        PublishedPort {
            service: "imap".into(),
            protocol: "tcp".into(),
            host_port: ports.imap,
            container_port: 143,
            tls: false,
        },
        PublishedPort {
            service: "imaps".into(),
            protocol: "tcp".into(),
            host_port: ports.imaps,
            container_port: 993,
            tls: true,
        },
        PublishedPort {
            service: "pop3".into(),
            protocol: "tcp".into(),
            host_port: ports.pop3,
            container_port: 110,
            tls: false,
        },
        PublishedPort {
            service: "pop3s".into(),
            protocol: "tcp".into(),
            host_port: ports.pop3s,
            container_port: 995,
            tls: true,
        },
        PublishedPort {
            service: "mysql".into(),
            protocol: "tcp".into(),
            host_port: ports.mysql,
            container_port: 3306,
            tls: false,
        },
        PublishedPort {
            service: "postgres".into(),
            protocol: "tcp".into(),
            host_port: ports.postgres,
            container_port: 5432,
            tls: false,
        },
        PublishedPort {
            service: "redis".into(),
            protocol: "tcp".into(),
            host_port: ports.redis,
            container_port: 6379,
            tls: false,
        },
        PublishedPort {
            service: "mongodb".into(),
            protocol: "tcp".into(),
            host_port: ports.mongodb,
            container_port: 27017,
            tls: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HostingPortsConfig;

    #[test]
    fn database_port_bindings_publish_when_enabled() {
        let ports = HostingPortsConfig {
            mysql: 13306,
            postgres: 15432,
            redis: 16379,
            mongodb: 27018,
            ..HostingPortsConfig::default()
        };

        let mysql = mysql_port_bindings(&ports);
        assert_eq!(
            mysql
                .get("3306/tcp")
                .and_then(|b| b.as_ref())
                .and_then(|b| b.first()),
            Some(&bollard::models::PortBinding {
                host_ip: Some("0.0.0.0".into()),
                host_port: Some("13306".into()),
            })
        );

        let postgres = postgres_port_bindings(&ports);
        assert!(postgres.contains_key("5432/tcp"));

        let redis = redis_port_bindings(&ports);
        assert!(redis.contains_key("6379/tcp"));

        let mongo = mongodb_port_bindings(&ports);
        assert!(mongo.contains_key("27017/tcp"));
    }

    #[test]
    fn database_port_bindings_hidden_when_publish_disabled() {
        let ports = HostingPortsConfig {
            publish_service_ports: false,
            ..HostingPortsConfig::default()
        };
        assert!(mysql_port_bindings(&ports).is_empty());
        assert!(postgres_port_bindings(&ports).is_empty());
    }

    #[test]
    fn list_published_ports_includes_database_services() {
        let ports = HostingPortsConfig::default();
        let published = list_published_ports(&ports);
        let services: Vec<_> = published.iter().map(|p| p.service.as_str()).collect();
        assert!(services.contains(&"mysql"));
        assert!(services.contains(&"postgres"));
        assert!(services.contains(&"redis"));
        assert!(services.contains(&"mongodb"));
    }
}
