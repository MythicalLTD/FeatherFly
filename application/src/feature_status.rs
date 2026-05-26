use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use utoipa::openapi::OpenApi;
use utoipa::openapi::extensions::ExtensionsBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    Implemented,
    Prototype,
    Missing,
    Deprecated,
}

impl FeatureStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Prototype => "prototype",
            Self::Missing => "missing",
            Self::Deprecated => "deprecated",
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubsystemStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub status: FeatureStatus,
    pub summary: &'static str,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EndpointStatus {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub status: FeatureStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FeatureCatalogResponse {
    pub product_readiness: &'static str,
    pub plesk_replacement: bool,
    pub subsystems: Vec<SubsystemStatus>,
    pub endpoints: Vec<EndpointStatus>,
}

struct EndpointEntry {
    operation_id: &'static str,
    status: FeatureStatus,
    note: Option<&'static str>,
}

const SUBSYSTEMS: &[SubsystemStatus] = &[
    SubsystemStatus {
        id: "daemon",
        name: "Daemon core",
        status: FeatureStatus::Implemented,
        summary: "Startup, config, logging, OpenAPI, CLI, and panel WebSocket client.",
    },
    SubsystemStatus {
        id: "health",
        name: "Health and readiness",
        status: FeatureStatus::Implemented,
        summary: "Dependency-aware /health, /live, and /ready probes.",
    },
    SubsystemStatus {
        id: "auth",
        name: "Authentication",
        status: FeatureStatus::Implemented,
        summary: "Single node bearer token (Wings-compatible). Panel signs JWTs for browser downloads and WebSockets.",
    },
    SubsystemStatus {
        id: "audit",
        name: "Audit logging",
        status: FeatureStatus::Implemented,
        summary: "SQLite audit events for config, sites, deploy, backup, exec, files, DNS, mail, FTP, and scheduler.",
    },
    SubsystemStatus {
        id: "docker",
        name: "Docker integration",
        status: FeatureStatus::Implemented,
        summary: "Container CRUD, exec, logs, stats, networks, volumes, and explicit disabled/unavailable behavior.",
    },
    SubsystemStatus {
        id: "proxy",
        name: "Reverse proxy / TLS",
        status: FeatureStatus::Prototype,
        summary: "Traefik auto-provision and config-level ACME; no per-site cert upload API.",
    },
    SubsystemStatus {
        id: "eggs",
        name: "Service eggs",
        status: FeatureStatus::Implemented,
        summary: "Local JSON egg catalog (Wings runtime contract); supports FeatherFly eggs and Pterodactyl PTDL_v2 import.",
    },
    SubsystemStatus {
        id: "sites",
        name: "Site hosting",
        status: FeatureStatus::Prototype,
        summary: "Site CRUD with update/suspend/unsuspend; sites provision from local egg JSON (Wings/Pterodactyl-style), not YAML templates.",
    },
    SubsystemStatus {
        id: "deploy",
        name: "Deployments",
        status: FeatureStatus::Prototype,
        summary: "Git/webhook/blue-green deploy with pre-traffic health checks and automatic rollback on failed probes.",
    },
    SubsystemStatus {
        id: "backups",
        name: "Backups",
        status: FeatureStatus::Prototype,
        summary: "Site volume tar.gz only; no DB/mail/DNS bundling or remote storage.",
    },
    SubsystemStatus {
        id: "dns",
        name: "DNS",
        status: FeatureStatus::Prototype,
        summary: "Local zone files and metadata; not published to authoritative DNS or providers.",
    },
    SubsystemStatus {
        id: "mail",
        name: "Mail hosting",
        status: FeatureStatus::Prototype,
        summary: "docker-mailserver containers; plaintext mailbox passwords; no DKIM/SPF/DMARC product.",
    },
    SubsystemStatus {
        id: "ftp",
        name: "FTP/SFTP",
        status: FeatureStatus::Prototype,
        summary: "Account records and shared gateway containers; isolation gaps remain.",
    },
    SubsystemStatus {
        id: "databases",
        name: "Databases",
        status: FeatureStatus::Prototype,
        summary: "Per-site addon containers; no user/grant/backup product.",
    },
    SubsystemStatus {
        id: "files",
        name: "File manager",
        status: FeatureStatus::Prototype,
        summary: "Volume shell operations with path sanitization; command safety not production-grade.",
    },
    SubsystemStatus {
        id: "scheduler",
        name: "Scheduler",
        status: FeatureStatus::Prototype,
        summary: "Cron/once/on-demand tasks; limited persistence and retry visibility.",
    },
    SubsystemStatus {
        id: "quotas",
        name: "Quotas and limits",
        status: FeatureStatus::Prototype,
        summary: "Memory/CPU apply to Docker containers; API/JWT uploads check disk quota. Container-internal disk writes and bandwidth are not enforced.",
    },
    SubsystemStatus {
        id: "tenancy",
        name: "Multi-tenancy",
        status: FeatureStatus::Missing,
        summary: "Customer isolation lives in FeatherPanel; daemon is single-node with one panel token.",
    },
    SubsystemStatus {
        id: "plugins",
        name: "Plugin system",
        status: FeatureStatus::Prototype,
        summary: "Native in-process .so hooks; no sandbox, timeouts, or capability limits.",
    },
    SubsystemStatus {
        id: "observability",
        name: "Observability",
        status: FeatureStatus::Prototype,
        summary: "Prometheus metrics and host stats; no alerts, diagnostics bundle, or job retry UI.",
    },
    SubsystemStatus {
        id: "packaging",
        name: "Packaging and upgrade",
        status: FeatureStatus::Missing,
        summary: "CLI install/configure exists; no supported OS matrix or migration tests.",
    },
];

const ENDPOINTS: &[EndpointEntry] = &[
    // Health
    entry("get_health", FeatureStatus::Implemented, None),
    entry("get_live", FeatureStatus::Implemented, None),
    entry("get_ready", FeatureStatus::Implemented, None),
    // Public JWT downloads (panel-signed, no bearer)
    entry(
        "get_download_file",
        FeatureStatus::Implemented,
        Some("JWT query token; one-time unique_id"),
    ),
    entry(
        "get_download_backup",
        FeatureStatus::Implemented,
        Some("JWT query token; one-time unique_id"),
    ),
    entry(
        "get_sites_ws",
        FeatureStatus::Implemented,
        Some("JWT auth event after upgrade; no bearer"),
    ),
    entry(
        "post_upload_file",
        FeatureStatus::Implemented,
        Some(
            "JWT query token + multipart file field; checks site disk quota and returns 413 on quota failures",
        ),
    ),
    // System
    entry("get_system", FeatureStatus::Implemented, None),
    entry("get_system_overview", FeatureStatus::Implemented, None),
    entry("get_system_config", FeatureStatus::Implemented, None),
    entry("put_system_config", FeatureStatus::Implemented, None),
    entry("post_system_restart", FeatureStatus::Implemented, None),
    entry("get_system_audit", FeatureStatus::Implemented, None),
    entry("get_system_features", FeatureStatus::Implemented, None),
    entry("get_system_plugins", FeatureStatus::Implemented, None),
    entry(
        "get_system_plugins_schema",
        FeatureStatus::Implemented,
        None,
    ),
    entry(
        "post_system_plugins_reload",
        FeatureStatus::Implemented,
        None,
    ),
    entry("get_system_update", FeatureStatus::Implemented, None),
    entry("post_system_update_apply", FeatureStatus::Implemented, None),
    entry("post_system_upgrade", FeatureStatus::Implemented, None),
    // Infrastructure
    entry("get_docker_status", FeatureStatus::Implemented, None),
    entry(
        "get_proxy_status",
        FeatureStatus::Prototype,
        Some("Traefik status only; TLS via config ACME"),
    ),
    entry("get_metrics", FeatureStatus::Implemented, None),
    entry("get_stats", FeatureStatus::Implemented, None),
    // Containers
    entry("get_containers", FeatureStatus::Implemented, None),
    entry("post_containers_create", FeatureStatus::Implemented, None),
    entry("get_containers_inspect", FeatureStatus::Implemented, None),
    entry("post_containers_restart", FeatureStatus::Implemented, None),
    entry("post_containers_pull", FeatureStatus::Implemented, None),
    entry("get_containers_logs", FeatureStatus::Implemented, None),
    entry("post_containers_exec", FeatureStatus::Implemented, None),
    entry("get_containers_exec_ws", FeatureStatus::Implemented, None),
    entry("get_containers_stats", FeatureStatus::Implemented, None),
    entry("post_containers_start", FeatureStatus::Implemented, None),
    entry("post_containers_stop", FeatureStatus::Implemented, None),
    entry("delete_containers", FeatureStatus::Implemented, None),
    // Sites — core
    entry("get_sites", FeatureStatus::Implemented, None),
    entry("post_sites", FeatureStatus::Implemented, None),
    entry("get_sites_by_id", FeatureStatus::Implemented, None),
    entry("patch_sites", FeatureStatus::Implemented, None),
    entry("post_sites_suspend", FeatureStatus::Implemented, None),
    entry("post_sites_unsuspend", FeatureStatus::Implemented, None),
    entry("delete_sites", FeatureStatus::Implemented, None),
    entry("get_eggs", FeatureStatus::Implemented, None),
    entry("get_eggs_by_id", FeatureStatus::Implemented, None),
    entry(
        "get_sites_logs",
        FeatureStatus::Implemented,
        Some("Supports tail, grep, and download=1"),
    ),
    entry("post_sites_logs_snapshot", FeatureStatus::Implemented, None),
    entry("get_sites_logs_archives", FeatureStatus::Implemented, None),
    entry("get_sites_logs_archive", FeatureStatus::Implemented, None),
    entry(
        "post_sites_reinstall",
        FeatureStatus::Implemented,
        Some("Runs egg install script"),
    ),
    entry("get_sites_hosting", FeatureStatus::Implemented, None),
    entry(
        "put_sites_hosting_limits",
        FeatureStatus::Prototype,
        Some(
            "Memory/CPU applied live; disk quota applies to daemon-mediated uploads; bandwidth metadata only",
        ),
    ),
    entry("get_sites_stats", FeatureStatus::Implemented, None),
    // Deploy
    entry(
        "post_sites_deploy",
        FeatureStatus::Implemented,
        Some("Blue/green waits for health before routing traffic"),
    ),
    entry(
        "post_sites_deploy_webhook",
        FeatureStatus::Prototype,
        Some("Uses standard deploy path"),
    ),
    // Backups
    entry(
        "post_sites_backups",
        FeatureStatus::Prototype,
        Some("Volume tar.gz only"),
    ),
    entry(
        "get_sites_backups",
        FeatureStatus::Prototype,
        Some("Volume tar.gz only"),
    ),
    entry(
        "post_sites_backup_restore",
        FeatureStatus::Prototype,
        Some("Volume tar.gz only"),
    ),
    // Databases
    entry(
        "get_sites_databases",
        FeatureStatus::Prototype,
        Some("Addon containers; no user/grant management"),
    ),
    entry(
        "post_sites_databases",
        FeatureStatus::Prototype,
        Some("Addon containers; no user/grant management"),
    ),
    entry(
        "delete_sites_database",
        FeatureStatus::Prototype,
        Some("Addon containers; no user/grant management"),
    ),
    // Scheduler
    entry("get_sites_tasks", FeatureStatus::Prototype, None),
    entry("post_sites_tasks", FeatureStatus::Prototype, None),
    entry("post_sites_task_run", FeatureStatus::Prototype, None),
    entry("delete_sites_task", FeatureStatus::Prototype, None),
    // DNS
    entry(
        "get_sites_dns",
        FeatureStatus::Prototype,
        Some("Local metadata; not published"),
    ),
    entry(
        "put_sites_dns",
        FeatureStatus::Prototype,
        Some("Local zone files only"),
    ),
    // Mail
    entry(
        "get_sites_mail",
        FeatureStatus::Prototype,
        Some("Container provisioning only"),
    ),
    entry(
        "get_sites_mail_accounts",
        FeatureStatus::Prototype,
        Some("Plaintext passwords in SQLite"),
    ),
    entry(
        "post_sites_mail_accounts",
        FeatureStatus::Prototype,
        Some("Plaintext passwords in SQLite"),
    ),
    entry(
        "delete_sites_mail_account",
        FeatureStatus::Prototype,
        Some("Plaintext passwords in SQLite"),
    ),
    entry(
        "put_sites_mail_sync",
        FeatureStatus::Prototype,
        Some("Container reprovision only"),
    ),
    // FTP
    entry("get_sites_ftp", FeatureStatus::Prototype, None),
    entry("get_sites_ftp_accounts", FeatureStatus::Prototype, None),
    entry(
        "post_sites_ftp_accounts",
        FeatureStatus::Prototype,
        Some("Shared gateway; isolation gaps"),
    ),
    entry("delete_sites_ftp_account", FeatureStatus::Prototype, None),
    entry("put_sites_ftp_sync", FeatureStatus::Prototype, None),
    // Exec and files
    entry(
        "post_sites_exec",
        FeatureStatus::Prototype,
        Some("Audited; no per-customer RBAC"),
    ),
    entry(
        "get_sites_files",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "get_sites_files_download",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "put_sites_files",
        FeatureStatus::Prototype,
        Some(
            "Docker volume shell; checks site disk quota before upload and returns 413 on quota failures",
        ),
    ),
    entry(
        "delete_sites_files",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "post_sites_files_mkdir",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "post_sites_files_rename",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "post_sites_files_move",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "post_sites_files_copy",
        FeatureStatus::Prototype,
        Some("Docker volume shell"),
    ),
    entry(
        "post_sites_files_archive",
        FeatureStatus::Prototype,
        Some("Archive extraction safety not production-grade"),
    ),
];

const fn entry(
    operation_id: &'static str,
    status: FeatureStatus,
    note: Option<&'static str>,
) -> EndpointEntry {
    EndpointEntry {
        operation_id,
        status,
        note,
    }
}

pub fn catalog_response() -> FeatureCatalogResponse {
    FeatureCatalogResponse {
        product_readiness: "experimental",
        plesk_replacement: false,
        subsystems: SUBSYSTEMS.to_vec(),
        endpoints: Vec::new(),
    }
}

pub fn endpoint_catalog() -> HashMap<&'static str, (&'static FeatureStatus, Option<&'static str>)> {
    ENDPOINTS
        .iter()
        .map(|e| (e.operation_id, (&e.status, e.note)))
        .collect()
}

pub fn apply_openapi_extensions(openapi: &mut OpenApi) {
    let catalog = endpoint_catalog();

    for item in openapi.paths.paths.values_mut() {
        for operation in [
            &mut item.get,
            &mut item.post,
            &mut item.put,
            &mut item.patch,
            &mut item.delete,
        ]
        .into_iter()
        .flatten()
        {
            let Some(operation_id) = operation.operation_id.as_deref() else {
                continue;
            };

            let (status, note) = if let Some((status, note)) = catalog.get(operation_id) {
                (**status, *note)
            } else if operation
                .tags
                .as_ref()
                .is_some_and(|tags| tags.iter().any(|t| t == "Plugin routes"))
            {
                (FeatureStatus::Prototype, Some("Runtime plugin route"))
            } else {
                (
                    FeatureStatus::Missing,
                    Some("Not catalogued — update feature_status.rs"),
                )
            };

            let mut builder = ExtensionsBuilder::new().add("x-feature-status", status.as_str());
            if let Some(note) = note {
                builder = builder.add("x-feature-status-note", note);
            }
            operation.extensions = Some(builder.build());
        }
    }
}

pub fn collect_openapi_endpoint_statuses(openapi: &OpenApi) -> Vec<EndpointStatus> {
    let catalog = endpoint_catalog();
    let mut endpoints = Vec::new();

    for (path, item) in &openapi.paths.paths {
        for (method, operation) in [
            ("GET", &item.get),
            ("POST", &item.post),
            ("PUT", &item.put),
            ("PATCH", &item.patch),
            ("DELETE", &item.delete),
        ] {
            let Some(operation) = operation else {
                continue;
            };
            let operation_id = operation
                .operation_id
                .clone()
                .unwrap_or_else(|| format!("{method}_{path}"));

            let (status, note) = if let Some((status, note)) = catalog.get(operation_id.as_str()) {
                (**status, *note)
            } else if operation
                .tags
                .as_ref()
                .is_some_and(|tags| tags.iter().any(|t| t == "Plugin routes"))
            {
                (FeatureStatus::Prototype, Some("Runtime plugin route"))
            } else {
                (
                    FeatureStatus::Missing,
                    Some("Not catalogued — update feature_status.rs"),
                )
            };

            endpoints.push(EndpointStatus {
                operation_id,
                method: method.into(),
                path: path.clone(),
                status,
                note,
            });
        }
    }

    endpoints.sort_by(|a, b| a.path.cmp(&b.path).then(a.method.cmp(&b.method)));
    endpoints
}

pub fn uncatalogued_operation_ids(openapi: &OpenApi) -> Vec<String> {
    let catalog = endpoint_catalog();
    let mut missing = Vec::new();

    for item in openapi.paths.paths.values() {
        for operation in [&item.get, &item.post, &item.put, &item.patch, &item.delete]
            .into_iter()
            .flatten()
        {
            let Some(operation_id) = operation.operation_id.as_deref() else {
                continue;
            };
            let is_plugin = operation
                .tags
                .as_ref()
                .is_some_and(|tags| tags.iter().any(|t| t == "Plugin routes"));
            if !catalog.contains_key(operation_id) && !is_plugin {
                missing.push(operation_id.into());
            }
        }
    }

    missing.sort();
    missing.dedup();
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_spec::build_openapi;

    #[test]
    fn every_openapi_operation_is_catalogued() {
        let openapi = build_openapi("FeatherFly");
        let missing = uncatalogued_operation_ids(&openapi);
        assert!(
            missing.is_empty(),
            "uncatalogued operation_ids: {missing:?}"
        );
    }

    #[test]
    fn openapi_operations_have_feature_status_extension() {
        let mut openapi = build_openapi("FeatherFly");
        apply_openapi_extensions(&mut openapi);

        for item in openapi.paths.paths.values() {
            for operation in [&item.get, &item.post, &item.put, &item.patch, &item.delete]
                .into_iter()
                .flatten()
            {
                let extensions = operation.extensions.as_ref().expect("extensions");
                let value = serde_json::to_value(extensions).unwrap();
                assert!(
                    value.get("x-feature-status").is_some(),
                    "operation {:?} missing x-feature-status",
                    operation.operation_id
                );
            }
        }
    }

    #[test]
    fn quota_catalog_status_matches_partial_enforcement() {
        let quotas = SUBSYSTEMS
            .iter()
            .find(|subsystem| subsystem.id == "quotas")
            .expect("quotas subsystem");
        assert_eq!(quotas.status, FeatureStatus::Prototype);
        assert!(quotas.summary.contains("Memory/CPU"));
        assert!(quotas.summary.contains("disk quota"));
        assert!(quotas.summary.contains("bandwidth"));

        let catalog = endpoint_catalog();
        let (_, note) = catalog
            .get("put_sites_hosting_limits")
            .expect("hosting limits endpoint");
        let note = note.expect("hosting limits note");
        assert!(note.contains("Memory/CPU"));
        assert!(note.contains("disk quota"));
        assert!(note.contains("bandwidth"));
    }
}
