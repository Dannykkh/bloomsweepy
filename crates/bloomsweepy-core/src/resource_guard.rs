//! Cooperative resource policy, sampled at most twice a second. This is an
//! early-stop safeguard, not a hard OS memory quota or a total app-tree meter.
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

const MIB: u64 = 1024 * 1024;
const MIN_AVAILABLE: u64 = 256 * MIB;
const MAX_HOST_MEMORY: u64 = 512 * MIB;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

struct SampleCache {
    system: System,
    last_sample: Option<Instant>,
    result: Result<(), String>,
}

/// Gate expensive local work. No user files, command lines or other process
/// details are collected. Only global memory and the calling process are read.
pub fn ensure_operation_memory() -> Result<(), String> {
    static SAMPLE: OnceLock<Mutex<SampleCache>> = OnceLock::new();
    let sample = SAMPLE.get_or_init(|| {
        Mutex::new(SampleCache {
            system: System::new(),
            last_sample: None,
            result: Ok(()),
        })
    });
    let mut sample = sample
        .lock()
        .map_err(|_| "메모리 안전 상태를 읽지 못했습니다")?;
    if sample
        .last_sample
        .is_some_and(|last| last.elapsed() < SAMPLE_INTERVAL)
    {
        return sample.result.clone();
    }
    let pid = Pid::from_u32(std::process::id());
    sample
        .system
        .refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    sample.system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    sample.result = check_sample(
        sample.system.total_memory(),
        sample.system.available_memory(),
        sample.system.process(pid).map(|process| process.memory()),
    );
    sample.last_sample = Some(Instant::now());
    sample.result.clone()
}

fn check_sample(total: u64, available: u64, host: Option<u64>) -> Result<(), String> {
    let Some(host) = host.filter(|_| total > 0) else {
        return Err(
            "메모리 상태를 확인하지 못해 작업을 중단했습니다. 잠시 후 다시 시도해 주세요".into(),
        );
    };
    if available < MIN_AVAILABLE {
        return Err("시스템 여유 메모리가 부족해 작업을 중단했습니다. 다른 작업을 마친 뒤 다시 시도해 주세요".into());
    }
    if host > MAX_HOST_MEMORY {
        return Err("BroomSweepy의 작업 메모리 한도(512 MiB)에 도달했습니다. 더 작은 폴더로 다시 시도해 주세요".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_resource_gate_is_repeatable_without_allocating_pressure() {
        assert!(check_sample(8 * 1024 * MIB, MIN_AVAILABLE, Some(MAX_HOST_MEMORY)).is_ok());
        assert!(check_sample(8 * 1024 * MIB, MIN_AVAILABLE - 1, Some(30 * MIB)).is_err());
        assert!(check_sample(8 * 1024 * MIB, 1024 * MIB, Some(MAX_HOST_MEMORY + 1)).is_err());
        assert!(check_sample(0, 0, None).is_err());
        assert!(check_sample(8 * 1024 * MIB, 1024 * MIB, None).is_err());
        assert!(check_sample(8 * 1024 * MIB, 1024 * MIB, Some(30 * MIB)).is_ok());
    }
}
