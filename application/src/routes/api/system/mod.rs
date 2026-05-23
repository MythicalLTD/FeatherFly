use crate::routes::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod config;
mod overview;
mod plugins;
mod restart;
mod update;
mod upgrade;

mod get {
    use crate::actions::{ApiAction, system_actions};
    use crate::{
        response::{ApiResponse, ApiResponseResult},
        routes::GetState,
    };
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response<'a> {
        architecture: &'static str,
        cpu_count: usize,
        kernel_version: String,
        os: &'static str,
        version: &'a str,
        actions: Vec<ApiAction>,
    }

    #[utoipa::path(
        get,
        path = "/",
        operation_id = "get_system",
        security(("bearer_auth" = [])),
        responses((status = OK, body = inline(Response))),
    )]
    pub async fn route(state: GetState) -> ApiResponseResult {
        ApiResponse::new_serialized(Response {
            architecture: std::env::consts::ARCH,
            cpu_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            kernel_version: sysinfo::System::kernel_long_version(),
            os: std::env::consts::OS,
            version: &state.version,
            actions: system_actions(),
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/config", config::router(state))
        .nest("/overview", overview::router(state))
        .nest("/plugins", plugins::router(state))
        .nest("/restart", restart::router(state))
        .nest("/update", update::router(state))
        .nest("/upgrade", upgrade::router(state))
        .with_state(state.clone())
}
