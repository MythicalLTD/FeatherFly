//! Every plugin event payload type must serialize to JSON without error.

use featherfly::utils::plugin_events::*;
use featherfly_plugin_sdk::PluginEvent;

macro_rules! assert_serializes {
    ($name:ident, $payload:expr) => {
        serde_json::to_vec(&$payload).expect(concat!("serialize ", stringify!($name)));
    };
}

#[test]
fn all_payload_structs_serialize() {
    assert_serializes!(
        request,
        RequestPayload {
            client_ip: "127.0.0.1",
            method: "GET",
            path: "/api/health",
            query: "",
        }
    );
    assert_serializes!(
        completed,
        RequestCompletedPayload {
            client_ip: "127.0.0.1",
            method: "POST",
            path: "/api/sites",
            query: "debug=1",
            status: 201,
            duration_ms: 42,
        }
    );
    assert_serializes!(
        database_created,
        DatabaseCreatedPayload {
            site_id: "s1",
            database_id: "db1",
            engine: "mysql",
            host: "featherfly-mysql",
            port: 3306,
        }
    );
    assert_serializes!(
        shared_stack,
        SharedStackSyncedPayload {
            mail: true,
            roundcube: true,
            sftp: false,
            ftp: true,
            phpmyadmin: true,
            database_servers: true,
        }
    );
    assert_serializes!(
        shared_db,
        SharedDatabaseServersReadyPayload {
            mysql: true,
            postgres: true,
            redis: true,
            mongodb: true,
            site_networks_connected: 4,
        }
    );
    assert_serializes!(
        reconcile,
        HostingReconciledPayload {
            sites_total: 3,
            sites_started: 2,
            mail_synced: 1,
            ftp_synced: 1,
            shared_stack: true,
        }
    );
    assert_serializes!(
        config_upgraded,
        ConfigUpgradedPayload {
            path: "/etc/featherfly/config.yml",
        }
    );
    assert_serializes!(
        traefik,
        TraefikProvisionedPayload {
            result: "created",
            image: "traefik:v3.3",
            network: "featherfly_nw",
            tls_enabled: true,
        }
    );
    assert_serializes!(
        images,
        HostingImagesPulledPayload {
            image_count: 2,
            images: vec!["mysql:8".into(), "redis:7-alpine".into()],
        }
    );
}

#[test]
fn new_hosting_events_are_in_catalog() {
    for name in [
        "config.upgraded",
        "hosting.reconciled",
        "hosting.shared_stack_synced",
        "hosting.shared_database_servers_ready",
        "hosting.phpmyadmin_provisioned",
        "hosting.error_pages_ready",
        "proxy.traefik_provisioned",
        "hosting.images_pulled",
    ] {
        assert!(
            PluginEvent::all_names().contains(&name),
            "missing event {name}"
        );
    }
}
