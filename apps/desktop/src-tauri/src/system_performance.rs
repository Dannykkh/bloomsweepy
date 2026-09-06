use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{
    CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, Pid, ProcessRefreshKind, ProcessesToUpdate,
    System, Uid, UpdateKind,
};
use tauri::State;

const SNAPSHOT_TTL: Duration = Duration::from_secs(60);
const PREVIEW_TTL: Duration = Duration::from_secs(30);
const TERMINATION_OBSERVE_WINDOW: Duration = Duration::from_secs(3);
const TERMINATION_OBSERVE_INTERVAL: Duration = Duration::from_millis(100);
const MAX_STORED_SNAPSHOTS: usize = 4;
const MAX_STORED_PREVIEWS: usize = 8;
const TOP_PER_SORT: usize = 20;
const MAX_PARENT_DEPTH: usize = 32;
const REFRESH_AFTER_MS: u64 = 2_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PerformanceSnapshot {
    snapshot_id: String,
    captured_at_unix_ms: u64,
    sample_window_ms: u64,
    refresh_after_ms: u64,
    platform: &'static str,
    logical_cpu_count: usize,
    cpu_usage_percent: f32,
    memory: PerformanceMemory,
    processes: Vec<ProcessUsage>,
    processes_truncated: bool,
    capabilities: PerformanceCapabilities,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PerformanceMemory {
    total_bytes: u64,
    available_bytes: u64,
    used_bytes: u64,
    total_swap_bytes: u64,
    used_swap_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessUsage {
    target_id: Option<String>,
    display_name: String,
    bundle_identifier: Option<String>,
    pid: u32,
    kind: ProcessKind,
    cpu_core_percent: f32,
    cpu_machine_percent: f32,
    resident_bytes: u64,
    process_count: usize,
    can_request_termination: bool,
    termination_eligibility: TerminationEligibility,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Some wire variants are emitted only by the other desktop platform build.
enum ProcessKind {
    GuiApp,
    AccessoryApp,
    UserProcess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Unsupported is emitted by non-macOS builds.
enum TerminationEligibility {
    Eligible,
    SelfApp,
    ProtectedSystemApp,
    OtherUser,
    IdentityUnavailable,
    NotRegularApplication,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum ProcessScope {
    GuiApplications,
    TopProcesses,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PerformanceCapabilities {
    process_metrics_available: bool,
    graceful_termination_available: bool,
    app_memory_cleanup_available: bool,
    process_scope: ProcessScope,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminationPreviewResponse {
    outcome: TerminationPreviewOutcome,
    preview: Option<TerminationPreview>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum TerminationPreviewOutcome {
    Ready,
    StaleSnapshot,
    StaleTarget,
    ProtectedTarget,
    Unsupported,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminationPreview {
    preview_id: String,
    display_name: String,
    pid: u32,
    captured_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecuteTerminationRequest {
    preview_id: String,
    unsaved_work_acknowledged: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminationResult {
    outcome: TerminationOutcome,
    display_name: Option<String>,
    requested_at_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum TerminationOutcome {
    Terminated,
    RequestSent,
    RequestRejected,
    AlreadyExited,
    StaleTarget,
    ProtectedTarget,
    PreviewExpired,
    PreviewAlreadyUsed,
    AcknowledgementRequired,
    Unsupported,
}

pub(crate) struct PerformanceMonitorState {
    sampler: Mutex<PerformanceSampler>,
    records: Mutex<PerformanceRecords>,
}

impl Default for PerformanceMonitorState {
    fn default() -> Self {
        Self {
            sampler: Mutex::new(PerformanceSampler::default()),
            records: Mutex::new(PerformanceRecords::default()),
        }
    }
}

struct PerformanceSampler {
    system: System,
    warmed: bool,
    last_sample_at: Option<Instant>,
}

impl Default for PerformanceSampler {
    fn default() -> Self {
        Self {
            system: System::new(),
            warmed: false,
            last_sample_at: None,
        }
    }
}

#[derive(Default)]
struct PerformanceRecords {
    snapshots: VecDeque<StoredSnapshot>,
    previews: HashMap<String, StoredPreview>,
    consumed_preview_ids: VecDeque<String>,
}

struct StoredSnapshot {
    id: String,
    expires_at: Instant,
    targets: HashMap<String, ProcessIdentity>,
}

struct StoredPreview {
    expires_at: Instant,
    identity: ProcessIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProcessIdentity {
    pid: u32,
    start_time: u64,
    executable_path: PathBuf,
    user_id: Uid,
    bundle_identifier: Option<String>,
    native_executable_path: Option<String>,
    display_name: String,
    #[cfg(target_os = "macos")]
    native_application: macos::NativeApplicationIdentity,
}

struct UsageCandidate {
    display_name: String,
    bundle_identifier: Option<String>,
    pid: u32,
    kind: ProcessKind,
    cpu_core_percent: f32,
    resident_bytes: u64,
    process_count: usize,
    eligibility: TerminationEligibility,
    identity: Option<ProcessIdentity>,
}

struct FinalizedCandidates {
    processes: Vec<ProcessUsage>,
    targets: HashMap<String, ProcessIdentity>,
    truncated: bool,
}

#[tauri::command]
pub(crate) async fn get_performance_snapshot(
    state: State<'_, Arc<PerformanceMonitorState>>,
) -> Result<PerformanceSnapshot, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || state.collect_snapshot())
        .await
        .map_err(|error| format!("성능 상태 조회 작업을 완료하지 못했습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn prepare_graceful_process_termination(
    state: State<'_, Arc<PerformanceMonitorState>>,
    snapshot_id: String,
    target_id: String,
) -> Result<TerminationPreviewResponse, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        state.prepare_termination(&snapshot_id, &target_id)
    })
    .await
    .map_err(|error| format!("앱 종료 확인을 준비하지 못했습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn execute_graceful_process_termination(
    state: State<'_, Arc<PerformanceMonitorState>>,
    request: ExecuteTerminationRequest,
) -> Result<TerminationResult, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || state.execute_termination(request))
        .await
        .map_err(|error| format!("앱 종료 요청을 완료하지 못했습니다: {error}"))?
}

impl PerformanceMonitorState {
    fn collect_snapshot(&self) -> Result<PerformanceSnapshot, String> {
        let snapshot_id = generate_id("성능 측정")?;
        let (
            captured_at_unix_ms,
            sample_window_ms,
            memory,
            cpu_usage_percent,
            cpu_count,
            candidates,
        ) = {
            let mut sampler = self
                .sampler
                .lock()
                .map_err(|_| "성능 측정 상태 잠금이 손상됐습니다".to_owned())?;
            sampler.refresh()?;
            let captured_at = unix_time_ms();
            let memory = PerformanceMemory::from_system(&sampler.system);
            let cpu_count = sampler.system.cpus().len().max(1);
            let cpu_usage = sanitize_percent(sampler.system.global_cpu_usage(), 100.0);
            let candidates = collect_usage_candidates(&sampler.system, cpu_count);
            let sample_window = sampler
                .last_sample_at
                .map(|previous| {
                    previous
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX)
                })
                .unwrap_or(MINIMUM_CPU_UPDATE_INTERVAL.as_millis() as u64);
            sampler.last_sample_at = Some(Instant::now());
            (
                captured_at,
                sample_window,
                memory,
                cpu_usage,
                cpu_count,
                candidates,
            )
        };

        let finalized = finalize_candidates(candidates, cpu_count)?;
        let capabilities = PerformanceCapabilities {
            process_metrics_available: true,
            graceful_termination_available: cfg!(target_os = "macos"),
            app_memory_cleanup_available: cfg!(target_os = "macos"),
            process_scope: if cfg!(target_os = "macos") {
                ProcessScope::GuiApplications
            } else {
                ProcessScope::TopProcesses
            },
        };

        let mut records = self
            .records
            .lock()
            .map_err(|_| "성능 측정 기록 잠금이 손상됐습니다".to_owned())?;
        records.prune();
        records.snapshots.push_back(StoredSnapshot {
            id: snapshot_id.clone(),
            expires_at: Instant::now() + SNAPSHOT_TTL,
            targets: finalized.targets,
        });
        while records.snapshots.len() > MAX_STORED_SNAPSHOTS {
            records.snapshots.pop_front();
        }

        Ok(PerformanceSnapshot {
            snapshot_id,
            captured_at_unix_ms,
            sample_window_ms,
            refresh_after_ms: REFRESH_AFTER_MS,
            platform: std::env::consts::OS,
            logical_cpu_count: cpu_count,
            cpu_usage_percent,
            memory,
            processes: finalized.processes,
            processes_truncated: finalized.truncated,
            capabilities,
        })
    }

    fn prepare_termination(
        &self,
        snapshot_id: &str,
        target_id: &str,
    ) -> Result<TerminationPreviewResponse, String> {
        if !cfg!(target_os = "macos") {
            return Ok(preview_response(TerminationPreviewOutcome::Unsupported));
        }

        let identity = {
            let mut records = self
                .records
                .lock()
                .map_err(|_| "성능 측정 기록 잠금이 손상됐습니다".to_owned())?;
            records.prune();
            let Some(snapshot) = records
                .snapshots
                .iter_mut()
                .find(|snapshot| snapshot.id == snapshot_id)
            else {
                return Ok(preview_response(TerminationPreviewOutcome::StaleSnapshot));
            };
            let Some(identity) = snapshot.targets.remove(target_id) else {
                return Ok(preview_response(TerminationPreviewOutcome::ProtectedTarget));
            };
            identity
        };

        match validate_termination_identity(&identity) {
            IdentityValidation::Current => {}
            IdentityValidation::Exited | IdentityValidation::Changed => {
                return Ok(preview_response(TerminationPreviewOutcome::StaleTarget));
            }
            IdentityValidation::Protected => {
                return Ok(preview_response(TerminationPreviewOutcome::ProtectedTarget));
            }
            IdentityValidation::Unsupported => {
                return Ok(preview_response(TerminationPreviewOutcome::Unsupported));
            }
        }

        let preview_id = generate_id("종료 확인")?;
        let expires_at_unix_ms = unix_time_ms().saturating_add(PREVIEW_TTL.as_millis() as u64);
        let preview = TerminationPreview {
            preview_id: preview_id.clone(),
            display_name: identity.display_name.clone(),
            pid: identity.pid,
            captured_at_unix_ms: unix_time_ms(),
            expires_at_unix_ms,
        };
        let mut records = self
            .records
            .lock()
            .map_err(|_| "성능 측정 기록 잠금이 손상됐습니다".to_owned())?;
        records.prune();
        records.previews.insert(
            preview_id,
            StoredPreview {
                expires_at: Instant::now() + PREVIEW_TTL,
                identity,
            },
        );
        while records.previews.len() > MAX_STORED_PREVIEWS {
            if let Some(oldest_id) = records
                .previews
                .iter()
                .min_by_key(|(_, preview)| preview.expires_at)
                .map(|(id, _)| id.clone())
            {
                records.previews.remove(&oldest_id);
            } else {
                break;
            }
        }

        Ok(TerminationPreviewResponse {
            outcome: TerminationPreviewOutcome::Ready,
            preview: Some(preview),
        })
    }

    fn execute_termination(
        &self,
        request: ExecuteTerminationRequest,
    ) -> Result<TerminationResult, String> {
        let requested_at_unix_ms = unix_time_ms();
        if !request.unsaved_work_acknowledged {
            return Ok(TerminationResult {
                outcome: TerminationOutcome::AcknowledgementRequired,
                display_name: None,
                requested_at_unix_ms,
            });
        }

        let stored = {
            let mut records = self
                .records
                .lock()
                .map_err(|_| "성능 측정 기록 잠금이 손상됐습니다".to_owned())?;
            records.prune();
            if records
                .consumed_preview_ids
                .iter()
                .any(|id| id == &request.preview_id)
            {
                return Ok(TerminationResult {
                    outcome: TerminationOutcome::PreviewAlreadyUsed,
                    display_name: None,
                    requested_at_unix_ms,
                });
            }
            let Some(stored) = records.previews.remove(&request.preview_id) else {
                return Ok(TerminationResult {
                    outcome: TerminationOutcome::PreviewExpired,
                    display_name: None,
                    requested_at_unix_ms,
                });
            };
            records.consumed_preview_ids.push_back(request.preview_id);
            while records.consumed_preview_ids.len() > 32 {
                records.consumed_preview_ids.pop_front();
            }
            stored
        };

        if stored.expires_at <= Instant::now() {
            return Ok(TerminationResult {
                outcome: TerminationOutcome::PreviewExpired,
                display_name: Some(stored.identity.display_name),
                requested_at_unix_ms,
            });
        }

        let display_name = stored.identity.display_name.clone();
        let outcome = request_platform_termination(&stored.identity);
        Ok(TerminationResult {
            outcome,
            display_name: Some(display_name),
            requested_at_unix_ms,
        })
    }
}

impl PerformanceSampler {
    fn refresh(&mut self) -> Result<(), String> {
        if !self.warmed {
            self.system
                .refresh_cpu_list(CpuRefreshKind::nothing().with_cpu_usage());
            self.system.refresh_memory();
            self.system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                process_refresh_kind(),
            );
            thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(20));
            self.warmed = true;
        } else if let Some(last_sample_at) = self.last_sample_at {
            let elapsed = last_sample_at.elapsed();
            if elapsed < MINIMUM_CPU_UPDATE_INTERVAL {
                thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL - elapsed);
            }
        }

        self.system.refresh_memory();
        self.system.refresh_cpu_usage();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            process_refresh_kind(),
        );
        Ok(())
    }
}

impl PerformanceMemory {
    fn from_system(system: &System) -> Self {
        Self {
            total_bytes: system.total_memory(),
            available_bytes: system.available_memory(),
            used_bytes: system
                .total_memory()
                .saturating_sub(system.available_memory()),
            total_swap_bytes: system.total_swap(),
            used_swap_bytes: system.used_swap(),
        }
    }
}

impl PerformanceRecords {
    fn prune(&mut self) {
        let now = Instant::now();
        self.snapshots.retain(|snapshot| snapshot.expires_at > now);
        self.previews.retain(|_, preview| preview.expires_at > now);
    }
}

fn process_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_user(UpdateKind::OnlyIfNotSet)
        .with_exe(UpdateKind::OnlyIfNotSet)
        .without_tasks()
}

fn identity_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .with_user(UpdateKind::OnlyIfNotSet)
        .with_exe(UpdateKind::OnlyIfNotSet)
        .without_tasks()
}

#[cfg(target_os = "macos")]
fn collect_usage_candidates(system: &System, _cpu_count: usize) -> Vec<UsageCandidate> {
    let native_apps = macos::running_gui_apps();
    let root_pids: HashSet<u32> = native_apps.iter().map(|app| app.pid).collect();
    let own_pid = std::process::id();
    let own_user = system
        .process(Pid::from_u32(own_pid))
        .and_then(|process| process.user_id())
        .cloned();
    let mut aggregates: HashMap<u32, (f32, u64, usize)> = native_apps
        .iter()
        .map(|app| (app.pid, (0.0, 0, 0)))
        .collect();

    for (pid, process) in system.processes() {
        let Some(root_pid) = nearest_gui_root(pid.as_u32(), system, &root_pids) else {
            continue;
        };
        if let Some((cpu, memory, count)) = aggregates.get_mut(&root_pid) {
            *cpu += sanitize_percent(process.cpu_usage(), f32::MAX);
            *memory = memory.saturating_add(process.memory());
            *count += 1;
        }
    }

    native_apps
        .into_iter()
        .filter_map(|app| {
            let root = system.process(Pid::from_u32(app.pid))?;
            let (cpu, memory, count) = aggregates.remove(&app.pid).unwrap_or_default();
            let root_user = root.user_id().cloned();
            let executable_path = root.exe().map(PathBuf::from);
            let eligibility = mac_termination_eligibility(
                &app,
                own_pid,
                own_user.as_ref(),
                root_user.as_ref(),
                executable_path.as_ref(),
            );
            let identity = if eligibility == TerminationEligibility::Eligible {
                Some(ProcessIdentity {
                    pid: app.pid,
                    start_time: root.start_time(),
                    executable_path: executable_path?,
                    user_id: root_user?,
                    bundle_identifier: app.bundle_identifier.clone(),
                    native_executable_path: app.executable_path.clone(),
                    display_name: app.display_name.clone(),
                    native_application: app.native_application.clone(),
                })
            } else {
                None
            };
            Some(UsageCandidate {
                display_name: app.display_name,
                bundle_identifier: app.bundle_identifier,
                pid: app.pid,
                kind: if app.regular {
                    ProcessKind::GuiApp
                } else {
                    ProcessKind::AccessoryApp
                },
                cpu_core_percent: cpu,
                resident_bytes: memory,
                process_count: count.max(1),
                eligibility,
                identity,
            })
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn collect_usage_candidates(system: &System, _cpu_count: usize) -> Vec<UsageCandidate> {
    system
        .processes()
        .values()
        .filter(|process| process.pid().as_u32() > 1)
        .map(|process| UsageCandidate {
            display_name: process.name().to_string_lossy().into_owned(),
            bundle_identifier: None,
            pid: process.pid().as_u32(),
            kind: ProcessKind::UserProcess,
            cpu_core_percent: sanitize_percent(process.cpu_usage(), f32::MAX),
            resident_bytes: process.memory(),
            process_count: 1,
            eligibility: TerminationEligibility::Unsupported,
            identity: None,
        })
        .collect()
}

fn nearest_gui_root(pid: u32, system: &System, roots: &HashSet<u32>) -> Option<u32> {
    let mut current = pid;
    let mut visited = HashSet::new();
    for _ in 0..MAX_PARENT_DEPTH {
        if roots.contains(&current) {
            return Some(current);
        }
        if !visited.insert(current) {
            return None;
        }
        current = system.process(Pid::from_u32(current))?.parent()?.as_u32();
    }
    None
}

fn finalize_candidates(
    candidates: Vec<UsageCandidate>,
    cpu_count: usize,
) -> Result<FinalizedCandidates, String> {
    let total_candidates = candidates.len();
    let selected_indices = top_union_indices(&candidates);
    let mut targets = HashMap::new();
    let mut processes = Vec::with_capacity(selected_indices.len());

    for index in selected_indices {
        let candidate = &candidates[index];
        let target_id = if let Some(identity) = &candidate.identity {
            let id = generate_id("프로세스 대상")?;
            targets.insert(id.clone(), identity.clone());
            Some(id)
        } else {
            None
        };
        processes.push(ProcessUsage {
            target_id,
            display_name: candidate.display_name.clone(),
            bundle_identifier: candidate.bundle_identifier.clone(),
            pid: candidate.pid,
            kind: candidate.kind,
            cpu_core_percent: round_percent(candidate.cpu_core_percent),
            cpu_machine_percent: round_percent(sanitize_percent(
                candidate.cpu_core_percent / cpu_count.max(1) as f32,
                100.0,
            )),
            resident_bytes: candidate.resident_bytes,
            process_count: candidate.process_count,
            can_request_termination: candidate.identity.is_some(),
            termination_eligibility: candidate.eligibility,
        });
    }
    processes.sort_by(|left, right| {
        right
            .cpu_machine_percent
            .partial_cmp(&left.cpu_machine_percent)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.resident_bytes.cmp(&left.resident_bytes))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });

    let processes_truncated = total_candidates > processes.len();
    Ok(FinalizedCandidates {
        processes,
        targets,
        truncated: processes_truncated,
    })
}

fn top_union_indices(candidates: &[UsageCandidate]) -> Vec<usize> {
    let mut cpu_indices: Vec<usize> = (0..candidates.len()).collect();
    cpu_indices.sort_by(|left, right| {
        candidates[*right]
            .cpu_core_percent
            .partial_cmp(&candidates[*left].cpu_core_percent)
            .unwrap_or(Ordering::Equal)
    });
    let mut memory_indices: Vec<usize> = (0..candidates.len()).collect();
    memory_indices.sort_by_key(|index| std::cmp::Reverse(candidates[*index].resident_bytes));

    let mut seen = HashSet::new();
    cpu_indices
        .into_iter()
        .take(TOP_PER_SORT)
        .chain(memory_indices.into_iter().take(TOP_PER_SORT))
        .filter(|index| seen.insert(*index))
        .collect()
}

#[cfg(target_os = "macos")]
fn mac_termination_eligibility(
    app: &macos::NativeGuiApp,
    own_pid: u32,
    own_user: Option<&Uid>,
    root_user: Option<&Uid>,
    executable_path: Option<&PathBuf>,
) -> TerminationEligibility {
    if app.pid == own_pid {
        return TerminationEligibility::SelfApp;
    }
    if !app.regular {
        return TerminationEligibility::NotRegularApplication;
    }
    if is_protected_macos_app(app.bundle_identifier.as_deref(), &app.display_name) {
        return TerminationEligibility::ProtectedSystemApp;
    }
    let (Some(own_user), Some(root_user)) = (own_user, root_user) else {
        return TerminationEligibility::IdentityUnavailable;
    };
    if own_user != root_user {
        return TerminationEligibility::OtherUser;
    }
    if app.bundle_identifier.is_none() || app.executable_path.is_none() || executable_path.is_none()
    {
        return TerminationEligibility::IdentityUnavailable;
    }
    TerminationEligibility::Eligible
}

#[cfg(target_os = "macos")]
fn is_protected_macos_app(bundle_identifier: Option<&str>, display_name: &str) -> bool {
    const BUNDLE_IDS: &[&str] = &[
        "com.apple.dock",
        "com.apple.finder",
        "com.apple.SystemUIServer",
        "com.apple.loginwindow",
        "com.apple.WindowManager",
    ];
    const NAMES: &[&str] = &[
        "Dock",
        "Finder",
        "SystemUIServer",
        "loginwindow",
        "WindowManager",
    ];
    bundle_identifier.is_some_and(|bundle| BUNDLE_IDS.contains(&bundle))
        || NAMES.contains(&display_name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Unsupported is returned by non-macOS builds.
enum IdentityValidation {
    Current,
    Exited,
    Changed,
    Protected,
    Unsupported,
}

#[cfg(target_os = "macos")]
fn validate_termination_identity(identity: &ProcessIdentity) -> IdentityValidation {
    if identity.pid <= 1
        || identity.pid == std::process::id()
        || is_protected_macos_app(
            identity.bundle_identifier.as_deref(),
            &identity.display_name,
        )
    {
        return IdentityValidation::Protected;
    }
    let mut system = System::new();
    let pid = Pid::from_u32(identity.pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        identity_refresh_kind(),
    );
    let Some(process) = system.process(pid) else {
        return IdentityValidation::Exited;
    };
    if process.start_time() != identity.start_time
        || process.exe() != Some(identity.executable_path.as_path())
        || process.user_id() != Some(&identity.user_id)
    {
        return IdentityValidation::Changed;
    }
    macos::validate_native_identity(identity)
}

#[cfg(not(target_os = "macos"))]
fn validate_termination_identity(_identity: &ProcessIdentity) -> IdentityValidation {
    IdentityValidation::Unsupported
}

#[cfg(target_os = "macos")]
fn request_platform_termination(identity: &ProcessIdentity) -> TerminationOutcome {
    match validate_termination_identity(identity) {
        IdentityValidation::Current => macos::request_normal_termination(identity),
        IdentityValidation::Exited => TerminationOutcome::AlreadyExited,
        IdentityValidation::Changed => TerminationOutcome::StaleTarget,
        IdentityValidation::Protected => TerminationOutcome::ProtectedTarget,
        IdentityValidation::Unsupported => TerminationOutcome::Unsupported,
    }
}

#[cfg(not(target_os = "macos"))]
fn request_platform_termination(_identity: &ProcessIdentity) -> TerminationOutcome {
    TerminationOutcome::Unsupported
}

fn preview_response(outcome: TerminationPreviewOutcome) -> TerminationPreviewResponse {
    TerminationPreviewResponse {
        outcome,
        preview: None,
    }
}

fn sanitize_percent(value: f32, maximum: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, maximum)
    } else {
        0.0
    }
}

fn round_percent(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

fn generate_id(label: &str) -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("{label} 번호를 만들지 못했습니다: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
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
    use super::{
        IdentityValidation, ProcessIdentity, TERMINATION_OBSERVE_INTERVAL,
        TERMINATION_OBSERVE_WINDOW, TerminationOutcome,
    };
    use objc2::rc::{Retained, autoreleasepool};
    use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
    use std::thread;
    use std::time::Instant;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct NativeApplicationIdentity {
        application: Retained<NSRunningApplication>,
    }

    pub(super) struct NativeGuiApp {
        pub(super) pid: u32,
        pub(super) display_name: String,
        pub(super) bundle_identifier: Option<String>,
        pub(super) executable_path: Option<String>,
        pub(super) regular: bool,
        pub(super) native_application: NativeApplicationIdentity,
    }

    pub(super) fn running_gui_apps() -> Vec<NativeGuiApp> {
        autoreleasepool(|_| {
            let workspace = NSWorkspace::sharedWorkspace();
            workspace
                .runningApplications()
                .iter()
                .filter_map(|app| {
                    let policy = app.activationPolicy();
                    if policy != NSApplicationActivationPolicy::Regular
                        && policy != NSApplicationActivationPolicy::Accessory
                    {
                        return None;
                    }
                    let pid = u32::try_from(app.processIdentifier()).ok()?;
                    if pid <= 1 || app.isTerminated() {
                        return None;
                    }
                    Some(NativeGuiApp {
                        pid,
                        display_name: app
                            .localizedName()
                            .map(|name| name.to_string())
                            .unwrap_or_else(|| format!("PID {pid}")),
                        bundle_identifier: app.bundleIdentifier().map(|value| value.to_string()),
                        executable_path: app
                            .executableURL()
                            .and_then(|url| url.path())
                            .map(|value| value.to_string()),
                        regular: policy == NSApplicationActivationPolicy::Regular,
                        native_application: NativeApplicationIdentity { application: app },
                    })
                })
                .collect()
        })
    }

    pub(super) fn validate_native_identity(identity: &ProcessIdentity) -> IdentityValidation {
        autoreleasepool(|_| {
            let app = &identity.native_application.application;
            if app.isTerminated() {
                return IdentityValidation::Exited;
            }
            if u32::try_from(app.processIdentifier()).ok() != Some(identity.pid)
                || app.activationPolicy() != NSApplicationActivationPolicy::Regular
                || app.bundleIdentifier().map(|value| value.to_string())
                    != identity.bundle_identifier
                || app
                    .executableURL()
                    .and_then(|url| url.path())
                    .map(|value| value.to_string())
                    != identity.native_executable_path
            {
                return IdentityValidation::Changed;
            }
            IdentityValidation::Current
        })
    }

    pub(super) fn request_normal_termination(identity: &ProcessIdentity) -> TerminationOutcome {
        autoreleasepool(|_| {
            let app = &identity.native_application.application;
            match validate_native_identity(identity) {
                IdentityValidation::Current => {}
                IdentityValidation::Exited => return TerminationOutcome::AlreadyExited,
                IdentityValidation::Changed => return TerminationOutcome::StaleTarget,
                IdentityValidation::Protected => return TerminationOutcome::ProtectedTarget,
                IdentityValidation::Unsupported => return TerminationOutcome::Unsupported,
            }
            if !app.terminate() {
                return TerminationOutcome::RequestRejected;
            }
            let deadline = Instant::now() + TERMINATION_OBSERVE_WINDOW;
            while Instant::now() < deadline {
                if app.isTerminated() {
                    return TerminationOutcome::Terminated;
                }
                thread::sleep(TERMINATION_OBSERVE_INTERVAL);
            }
            TerminationOutcome::RequestSent
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(cpu: f32, memory: u64, pid: u32) -> UsageCandidate {
        UsageCandidate {
            display_name: format!("Process {pid}"),
            bundle_identifier: None,
            pid,
            kind: ProcessKind::UserProcess,
            cpu_core_percent: cpu,
            resident_bytes: memory,
            process_count: 1,
            eligibility: TerminationEligibility::Unsupported,
            identity: None,
        }
    }

    #[test]
    fn percent_values_are_finite_and_bounded() {
        assert_eq!(sanitize_percent(f32::NAN, 100.0), 0.0);
        assert_eq!(sanitize_percent(f32::INFINITY, 100.0), 0.0);
        assert_eq!(sanitize_percent(-1.0, 100.0), 0.0);
        assert_eq!(sanitize_percent(120.0, 100.0), 100.0);
    }

    #[test]
    fn top_cpu_and_memory_union_is_deduplicated_and_bounded() {
        let candidates: Vec<_> = (0..100)
            .map(|index| candidate(index as f32, (100 - index) as u64, index + 2))
            .collect();
        let selected = top_union_indices(&candidates);
        let unique: HashSet<_> = selected.iter().copied().collect();
        assert_eq!(selected.len(), unique.len());
        assert!(selected.len() <= TOP_PER_SORT * 2);
        assert!(selected.contains(&99));
        assert!(selected.contains(&0));
    }

    #[test]
    fn machine_cpu_is_normalized_by_logical_cpu_count() {
        let finalized =
            finalize_candidates(vec![candidate(250.0, 1, 2)], 10).expect("finalize process rows");
        assert_eq!(finalized.processes[0].cpu_core_percent, 250.0);
        assert_eq!(finalized.processes[0].cpu_machine_percent, 25.0);
    }

    #[test]
    fn random_ids_are_opaque_hex_values() {
        let first = generate_id("test").expect("generate id");
        let second = generate_id("test").expect("generate id");
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn snapshot_serialization_does_not_expose_process_identity_fields() {
        let row = ProcessUsage {
            target_id: None,
            display_name: "Example".to_owned(),
            bundle_identifier: None,
            pid: 42,
            kind: ProcessKind::UserProcess,
            cpu_core_percent: 1.0,
            cpu_machine_percent: 0.5,
            resident_bytes: 1024,
            process_count: 1,
            can_request_termination: false,
            termination_eligibility: TerminationEligibility::Unsupported,
        };
        let value = serde_json::to_value(row).expect("serialize process row");
        assert!(value.get("executablePath").is_none());
        assert!(value.get("command").is_none());
        assert!(value.get("environment").is_none());
        assert_eq!(value["displayName"], "Example");
    }

    #[test]
    fn memory_used_saturates_when_available_exceeds_total() {
        let used = 1_u64.saturating_sub(2);
        assert_eq!(used, 0);
    }

    #[test]
    fn expired_records_are_pruned() {
        let mut records = PerformanceRecords::default();
        records.snapshots.push_back(StoredSnapshot {
            id: "expired".to_owned(),
            expires_at: Instant::now() - Duration::from_millis(1),
            targets: HashMap::new(),
        });
        records.prune();
        assert!(records.snapshots.is_empty());
    }

    #[test]
    fn preview_response_never_contains_data_when_not_ready() {
        let response = preview_response(TerminationPreviewOutcome::StaleTarget);
        assert!(response.preview.is_none());
    }

    #[test]
    fn live_snapshot_is_finite_and_bounded() {
        let state = PerformanceMonitorState::default();
        let snapshot = state
            .collect_snapshot()
            .expect("collect live performance snapshot");
        assert!(snapshot.cpu_usage_percent.is_finite());
        assert!((0.0..=100.0).contains(&snapshot.cpu_usage_percent));
        assert!(snapshot.memory.total_bytes >= snapshot.memory.used_bytes);
        assert!(snapshot.processes.len() <= TOP_PER_SORT * 2);
        assert!(
            snapshot
                .processes
                .iter()
                .all(|process| process.cpu_machine_percent.is_finite())
        );
    }
}
