use crate::{
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use serde::Serialize;

#[derive(Serialize)]
struct Response<'a> {
    uuid: &'a str,
    token_id: &'a str,
    remote: &'a str,
    architecture: &'static str,
    cpu_count: usize,
    kernel_version: String,
    os: &'static str,
    version: &'a str,
}

pub async fn get(state: GetState) -> ApiResponseResult {
    let inner = state.config.load();
    ApiResponse::new_serialized(Response {
        uuid: &inner.uuid,
        token_id: &inner.token_id,
        remote: &inner.remote,
        architecture: std::env::consts::ARCH,
        cpu_count: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        kernel_version: sysinfo::System::kernel_long_version(),
        os: std::env::consts::OS,
        version: &state.version,
    })
    .ok()
}
