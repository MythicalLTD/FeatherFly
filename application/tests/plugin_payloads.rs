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
    assert_serializes!(
        plugin_config_mutated,
        PluginConfigMutatedPayload {
            hook_count: 1,
            input_len: 128,
            output_len: 256,
        }
    );
    assert_serializes!(
        plugin_request_short_circuited,
        PluginRequestShortCircuitedPayload {
            phase: "request.intercept",
            method: "GET",
            path: "/api/system",
            status: 401,
        }
    );
    assert_serializes!(
        plugin_json_mutated,
        PluginJsonMutatedPayload {
            method: "GET",
            path: "/api/system",
            input_len: 32,
            output_len: 48,
        }
    );
    assert_serializes!(
        plugin_route_invoked,
        PluginRouteInvokedPayload {
            plugin: "hello",
            method: "GET",
            path: "/plugins/hello/ping",
            status: 200,
        }
    );
    assert_serializes!(
        plugin_route_failed,
        PluginRouteFailedPayload {
            plugin: "hello",
            method: "GET",
            path: "/plugins/hello/ping",
            error: "handler failed",
        }
    );
}

#[test]
fn new_infrastructure_and_plugin_events_are_in_catalog() {
    for name in [
        "config.upgraded",
        "hosting.reconciled",
        "hosting.shared_stack_synced",
        "hosting.shared_database_servers_ready",
        "hosting.phpmyadmin_provisioned",
        "hosting.error_pages_ready",
        "proxy.traefik_provisioned",
        "hosting.images_pulled",
        "plugins.config_mutated",
        "plugins.request_short_circuited",
        "plugins.json_response_mutated",
        "plugins.json_actions_mutated",
        "plugins.route_invoked",
        "plugins.route_failed",
    ] {
        assert!(
            PluginEvent::all_names().contains(&name),
            "missing event {name}"
        );
    }
}
