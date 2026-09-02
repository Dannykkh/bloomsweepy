use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemMemoryStatus {
    total_bytes: u64,
    available_bytes: u64,
    used_bytes: u64,
    total_swap_bytes: u64,
    used_swap_bytes: u64,
    captured_at_unix_ms: u64,
    platform: &'static str,
}

#[tauri::command]
pub(crate) async fn get_system_memory_status() -> Result<SystemMemoryStatus, String> {
    tauri::async_runtime::spawn_blocking(collect_system_memory_status)
        .await
        .map_err(|error| format!("시스템 메모리 상태를 조회하지 못했습니다: {error}"))
}

fn collect_system_memory_status() -> SystemMemoryStatus {
    let mut system = System::new();
    system.refresh_memory();

    build_system_memory_status(
        system.total_memory(),
        system.available_memory(),
        system.total_swap(),
        system.used_swap(),
        unix_time_ms(),
    )
}

fn build_system_memory_status(
    total_bytes: u64,
    available_bytes: u64,
    total_swap_bytes: u64,
    used_swap_bytes: u64,
    captured_at_unix_ms: u64,
) -> SystemMemoryStatus {
    SystemMemoryStatus {
        total_bytes,
        available_bytes,
        used_bytes: total_bytes.saturating_sub(available_bytes),
        total_swap_bytes,
        used_swap_bytes,
        captured_at_unix_ms,
        platform: std::env::consts::OS,
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_memory_is_total_less_available_and_saturates() {
        let status = build_system_memory_status(16_000, 6_000, 4_000, 1_000, 123);
        assert_eq!(status.used_bytes, 10_000);

        let inconsistent = build_system_memory_status(1, 2, 0, 0, 123);
        assert_eq!(inconsistent.used_bytes, 0);
    }

    #[test]
    fn response_serializes_with_the_frontend_contract() {
        let status = build_system_memory_status(16_000, 6_000, 4_000, 1_000, 123);
        let value = serde_json::to_value(status).expect("serialize memory status");

        assert_eq!(value["totalBytes"], 16_000);
        assert_eq!(value["availableBytes"], 6_000);
        assert_eq!(value["usedBytes"], 10_000);
        assert_eq!(value["totalSwapBytes"], 4_000);
        assert_eq!(value["usedSwapBytes"], 1_000);
        assert_eq!(value["capturedAtUnixMs"], 123);
        assert_eq!(value["platform"], std::env::consts::OS);
    }
}
