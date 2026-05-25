use utoipa::openapi::security::SecurityRequirement;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{OpenApi, ServerBuilder};
use utoipa::openapi::{
    path::{HttpMethod, OperationBuilder},
    response::Response,
};
use utoipa_axum::router::OpenApiRouter;

use crate::plugins::events::{RegisteredRoute, code_to_method};
use crate::routes::{AppContainerType, AppState};
use crate::{PROJECT_LICENSE, PROJECT_WEBSITE, VERSION};

pub fn build_openapi(app_name: &str) -> OpenApi {
    let state = dummy_state(app_name);
    let app = OpenApiRouter::new().merge(crate::routes::router(&state));
    let (_, openapi) = app.split_for_parts();
    finalize_openapi(openapi, app_name)
}

pub fn add_plugin_routes(mut openapi: OpenApi, routes: &[RegisteredRoute]) -> OpenApi {
    for route in routes {
        let Some(method) = plugin_route_method(route.method) else {
            continue;
        };
        if openapi
            .paths
            .get_path_operation(&route.path, method.clone())
            .is_some()
        {
            continue;
        }

        let operation = OperationBuilder::new()
            .tag("Plugin routes")
            .summary(Some(format!("Plugin route registered by {}", route.plugin)))
            .description(Some(
                "Runtime plugin route. Request and response bodies are owned by the plugin.",
            ))
            .operation_id(Some(plugin_operation_id(route)))
            .response("200", Response::new("Plugin route response"))
            .response("500", Response::new("Plugin route handler failed"))
            .build();

        openapi
            .paths
            .add_path_operation(&route.path, vec![method], operation);
    }

    openapi
}

pub fn finalize_openapi(mut openapi: OpenApi, app_name: &str) -> OpenApi {
    openapi.info.version = VERSION.into();
    openapi.info.description =
        Some("FeatherFly web hosting daemon API — part of the FeatherPanel ecosystem.".into());
    openapi.info.title = format!("{app_name} API");
    openapi.info.contact = Some(
        utoipa::openapi::ContactBuilder::new()
            .name(Some("FeatherPanel"))
            .url(Some(PROJECT_WEBSITE))
            .email(Some("support@featherpanel.com"))
            .build(),
    );
    openapi.info.license = Some(utoipa::openapi::License::new(PROJECT_LICENSE));

    openapi.components.as_mut().unwrap().add_security_scheme(
        "bearer_auth",
        SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
    );

    let security_req = SecurityRequirement::new("bearer_auth", Vec::<String>::new());
    for (path, item) in openapi.paths.paths.iter_mut() {
        let operations = [
            ("get", &mut item.get),
            ("post", &mut item.post),
            ("put", &mut item.put),
            ("patch", &mut item.patch),
            ("delete", &mut item.delete),
        ];

        let operation_suffix = path
            .replace('/', "_")
            .replace(|c| ['{', '}'].contains(&c), "");

        for (method, operation) in operations {
            if let Some(operation) = operation {
                if operation.operation_id.as_deref() == Some("route") {
                    operation.operation_id = Some(format!("{method}{operation_suffix}"));
                }
                if path == "/health" {
                    operation.security = None;
                } else {
                    operation.security = Some(vec![security_req.clone()]);
                }
            }
        }
    }

    openapi.servers = Some(vec![ServerBuilder::new().url("/").build()]);
    openapi
}

fn plugin_route_method(method: u32) -> Option<HttpMethod> {
    match code_to_method(method) {
        "GET" => Some(HttpMethod::Get),
        "POST" => Some(HttpMethod::Post),
        "PUT" => Some(HttpMethod::Put),
        "PATCH" => Some(HttpMethod::Patch),
        "DELETE" => Some(HttpMethod::Delete),
        _ => None,
    }
}

fn plugin_operation_id(route: &RegisteredRoute) -> String {
    let mut suffix = format!("{}_{}", code_to_method(route.method), route.path)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while suffix.contains("__") {
        suffix = suffix.replace("__", "_");
    }
    let suffix = suffix.trim_matches('_');
    format!("plugin_{}_{}", sanitize_segment(&route.plugin), suffix)
}

fn sanitize_segment(input: &str) -> String {
    let sanitized = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('_').to_string()
}

fn dummy_state(app_name: &str) -> std::sync::Arc<AppState> {
    std::sync::Arc::new(AppState {
        start_time: std::time::Instant::now(),
        container_type: AppContainerType::None,
        version: crate::full_version(),
        config: crate::config::Config::for_openapi_docs(app_name),
        plugins: crate::plugins::PluginRegistry::empty(),
        probe_guard: crate::middlewares::probe::ProbeGuard::new(),
        docker: None,
        panel: arc_swap::ArcSwapOption::from(None),
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use featherfly_plugin_sdk::{HTTP_METHOD_GET, ROUTE_OK, RouteHandlerContext};
    use utoipa::openapi::path::HttpMethod;

    extern "C" fn route_handler(_ctx: *const RouteHandlerContext) -> i32 {
        ROUTE_OK
    }

    #[test]
    fn openapi_contains_all_builtin_routes() {
        let openapi = build_openapi("FeatherFly");

        for (path, method) in [
            ("/api/sites/{id}/deploy/webhook", HttpMethod::Post),
            ("/api/containers/{id}/exec/ws", HttpMethod::Get),
            ("/api/system/plugins/schema", HttpMethod::Get),
        ] {
            assert!(
                openapi.paths.get_path_operation(path, method).is_some(),
                "OpenAPI missing {path}"
            );
        }
    }

    #[test]
    fn health_route_is_unauthenticated_in_openapi() {
        let openapi = build_openapi("FeatherFly");
        let operation = openapi
            .paths
            .get_path_operation("/health", HttpMethod::Get)
            .expect("health operation");

        assert!(operation.security.is_none());
    }

    #[test]
    fn plugin_routes_can_be_added_to_openapi() {
        let route = RegisteredRoute {
            plugin: "demo".into(),
            method: HTTP_METHOD_GET,
            path: "/plugins/demo/ping".into(),
            callback: route_handler,
        };
        let openapi = add_plugin_routes(build_openapi("FeatherFly"), &[route]);

        assert!(
            openapi
                .paths
                .get_path_operation("/plugins/demo/ping", HttpMethod::Get)
                .is_some()
        );
    }
}
