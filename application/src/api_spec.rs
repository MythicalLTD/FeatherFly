use utoipa::openapi::security::SecurityRequirement;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{OpenApi, ServerBuilder};
use utoipa_axum::router::OpenApiRouter;

use crate::routes::{AppContainerType, AppState};
use crate::{PROJECT_LICENSE, PROJECT_WEBSITE, VERSION};

pub fn build_openapi(app_name: &str) -> OpenApi {
    let state = dummy_state(app_name);
    let app = OpenApiRouter::new().merge(crate::routes::router(&state));
    let (_, openapi) = app.split_for_parts();
    finalize_openapi(openapi, app_name)
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
                operation.security = Some(vec![security_req.clone()]);
            }
        }
    }

    openapi.servers = Some(vec![ServerBuilder::new().url("/").build()]);
    openapi
}

fn dummy_state(app_name: &str) -> std::sync::Arc<AppState> {
    std::sync::Arc::new(AppState {
        start_time: std::time::Instant::now(),
        container_type: AppContainerType::None,
        version: crate::full_version(),
        config: crate::config::Config::for_openapi_docs(app_name),
        plugins: crate::plugins::PluginRegistry::empty(),
    })
}
