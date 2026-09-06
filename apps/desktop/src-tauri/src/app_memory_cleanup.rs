use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::State;

#[cfg(target_os = "macos")]
const OBSERVATION_SETTLE_TIME: Duration = Duration::from_millis(50);

#[derive(Default)]
pub(crate) struct AppMemoryCleanupState {
    in_progress: AtomicBool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Unsupported is emitted by non-macOS builds.
enum AppMemoryCleanupOutcome {
    Completed,
    Busy,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppMemoryCleanupResult {
    outcome: AppMemoryCleanupOutcome,
    allocator_released_bytes: u64,
    app_resident_before_bytes: Option<u64>,
    app_resident_after_bytes: Option<u64>,
    system_available_before_bytes: Option<u64>,
    system_available_after_bytes: Option<u64>,
    requested_at_unix_ms: u64,
    completed_at_unix_ms: u64,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct MemoryObservation {
    app_resident_bytes: Option<u64>,
    system_available_bytes: u64,
}

struct CleanupLease<'a> {
    in_progress: &'a AtomicBool,
}

impl Drop for CleanupLease<'_> {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
    }
}

impl AppMemoryCleanupState {
    fn begin(&self) -> Option<CleanupLease<'_>> {
        self.in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| CleanupLease {
                in_progress: &self.in_progress,
            })
    }

    fn clean(&self) -> AppMemoryCleanupResult {
        let requested_at_unix_ms = unix_time_ms();
        let Some(_lease) = self.begin() else {
            return outcome_without_observations(
                AppMemoryCleanupOutcome::Busy,
                requested_at_unix_ms,
            );
        };

        #[cfg(not(target_os = "macos"))]
        {
            outcome_without_observations(AppMemoryCleanupOutcome::Unsupported, requested_at_unix_ms)
        }

        #[cfg(target_os = "macos")]
        {
            let before = observe_memory();
            let allocator_released_bytes = macos::release_current_process_allocator_memory();
            std::thread::sleep(OBSERVATION_SETTLE_TIME);
            let after = observe_memory();

            AppMemoryCleanupResult {
                outcome: AppMemoryCleanupOutcome::Completed,
                allocator_released_bytes,
                app_resident_before_bytes: before.app_resident_bytes,
                app_resident_after_bytes: after.app_resident_bytes,
                system_available_before_bytes: Some(before.system_available_bytes),
                system_available_after_bytes: Some(after.system_available_bytes),
                requested_at_unix_ms,
                completed_at_unix_ms: unix_time_ms(),
            }
        }
    }
}

#[tauri::command]
pub(crate) async fn clean_app_memory(
    state: State<'_, Arc<AppMemoryCleanupState>>,
) -> Result<AppMemoryCleanupResult, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || state.clean())
        .await
        .map_err(|error| format!("앱 메모리 정리 작업을 완료하지 못했습니다: {error}"))
}

#[cfg(target_os = "macos")]
fn observe_memory() -> MemoryObservation {
    let mut system = System::new();
    system.refresh_memory();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory().without_tasks(),
    );

    MemoryObservation {
        app_resident_bytes: system.process(pid).map(|process| process.memory()),
        system_available_bytes: system.available_memory(),
    }
}

fn outcome_without_observations(
    outcome: AppMemoryCleanupOutcome,
    requested_at_unix_ms: u64,
) -> AppMemoryCleanupResult {
    AppMemoryCleanupResult {
        outcome,
        allocator_released_bytes: 0,
        app_resident_before_bytes: None,
        app_resident_after_bytes: None,
        system_available_before_bytes: None,
        system_available_after_bytes: None,
        requested_at_unix_ms,
        completed_at_unix_ms: unix_time_ms(),
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

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::ptr;

    unsafe extern "C" {
        fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
    }

    pub(super) fn release_current_process_allocator_memory() -> u64 {
        // SAFETY: Apple's public API accepts a null zone to inspect every malloc zone in the
        // current process and a zero goal to request maximal relief. It does not accept or touch
        // memory owned by another process. The returned value is the number of bytes unmapped.
        let released = unsafe { malloc_zone_pressure_relief(ptr::null_mut(), 0) };
        released.try_into().unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_lease_blocks_overlap_and_resets_when_dropped() {
        let state = AppMemoryCleanupState::default();
        let lease = state.begin().expect("first cleanup lease");
        assert!(state.begin().is_none());
        drop(lease);
        assert!(state.begin().is_some());
    }

    #[test]
    fn completed_result_serializes_without_inventing_a_system_reclaim_value() {
        let result = AppMemoryCleanupResult {
            outcome: AppMemoryCleanupOutcome::Completed,
            allocator_released_bytes: 4_096,
            app_resident_before_bytes: Some(20_000),
            app_resident_after_bytes: Some(18_000),
            system_available_before_bytes: Some(100_000),
            system_available_after_bytes: Some(101_000),
            requested_at_unix_ms: 10,
            completed_at_unix_ms: 20,
        };
        let value = serde_json::to_value(result).expect("serialize cleanup result");

        assert_eq!(value["outcome"], "completed");
        assert_eq!(value["allocatorReleasedBytes"], 4_096);
        assert_eq!(value["appResidentBeforeBytes"], 20_000);
        assert_eq!(value["appResidentAfterBytes"], 18_000);
        assert_eq!(value["systemAvailableBeforeBytes"], 100_000);
        assert_eq!(value["systemAvailableAfterBytes"], 101_000);
        assert!(value.get("freedBytes").is_none());
    }

    #[test]
    fn non_completed_outcomes_do_not_report_observations() {
        let result = outcome_without_observations(AppMemoryCleanupOutcome::Busy, 10);
        assert_eq!(result.outcome, AppMemoryCleanupOutcome::Busy);
        assert_eq!(result.allocator_released_bytes, 0);
        assert!(result.app_resident_before_bytes.is_none());
        assert!(result.app_resident_after_bytes.is_none());
        assert!(result.system_available_before_bytes.is_none());
        assert!(result.system_available_after_bytes.is_none());
    }
}
