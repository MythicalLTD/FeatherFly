use crate::{
    routes::GetState,
    utils::{
        actions::{ApiAction, system_actions},
        response::{ApiResponse, ApiResponseResult},
    },
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
pub async fn get(state: GetState) -> ApiResponseResult {
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
