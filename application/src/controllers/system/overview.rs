use crate::{
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use serde::Serialize;
use sysinfo::System;

#[derive(Serialize)]
struct ResponseCpu<'a> {
    name: &'a str,
    brand: &'a str,
    vendor_id: &'a str,
    frequency_mhz: u64,
    cpu_count: usize,
}

#[derive(Serialize)]
struct ResponseMemory {
    total_bytes: u64,
    free_bytes: u64,
    used_bytes: u64,
    used_bytes_process: u64,
}

#[derive(Serialize)]
struct Response<'a> {
    version: &'a str,
    local_time: chrono::DateTime<chrono::Local>,
    container_type: crate::routes::AppContainerType,
    cpu: ResponseCpu<'a>,
    memory: ResponseMemory,
    architecture: &'static str,
    kernel_version: String,
}

pub async fn get(state: GetState) -> ApiResponseResult {
    let mut sys = System::new_all();
    sys.refresh_cpu_all();

    let mut used_bytes_process = 0;
    if let Ok(current_pid) = sysinfo::get_current_pid() {
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[current_pid]),
            false,
            sysinfo::ProcessRefreshKind::nothing().with_memory(),
        );

        if let Some(process) = sys.process(current_pid) {
            used_bytes_process = process.memory();
        }
    }

    let cpu = sys.cpus().first();

    ApiResponse::new_serialized(Response {
        version: &state.version,
        local_time: chrono::Local::now(),
        container_type: state.container_type,
        cpu: ResponseCpu {
            name: cpu.map_or("unknown", sysinfo::Cpu::name),
            brand: cpu.map_or("unknown", sysinfo::Cpu::brand),
            vendor_id: cpu.map_or("unknown", sysinfo::Cpu::vendor_id),
            frequency_mhz: cpu.map_or(0, sysinfo::Cpu::frequency),
            cpu_count: sys.cpus().len(),
        },
        memory: ResponseMemory {
            total_bytes: sys.total_memory(),
            free_bytes: sys.free_memory(),
            used_bytes: sys.used_memory(),
            used_bytes_process,
        },
        architecture: std::env::consts::ARCH,
        kernel_version: System::kernel_long_version(),
    })
    .ok()
}
