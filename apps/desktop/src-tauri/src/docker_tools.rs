use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const SETTINGS_KEY: &str = "dockerManagementEnabled";
const DATABASE_FILE_NAME: &str = "developer-tools-v1.sqlite3";
const DOCKER_PROGRESS_EVENT: &str = "docker-cleanup-progress";
const STATUS_TIMEOUT: Duration = Duration::from_secs(30);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const PREVIEW_TTL: Duration = Duration::from_secs(5 * 60);
const OUTPUT_LIMIT_BYTES: u64 = 1024 * 1024;
const ERROR_DISPLAY_CHARS: usize = 500;
const HISTORY_LIMIT: usize = 50;
const CLEANUP_AGE_FILTER: &str = "until=168h";

#[derive(Default)]
pub(crate) struct DockerManagerState {
    running: AtomicBool,
    cancellation: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    active_cleanup_child: Arc<Mutex<Option<Child>>>,
    preview: Mutex<Option<StoredDockerCleanupPreview>>,
}

struct DockerOperationLease<'a> {
    state: &'a DockerManagerState,
}

#[derive(Clone)]
struct TrackedDockerChild {
    closed: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

struct DockerCleanupRun<'a> {
    app: &'a AppHandle,
    database_path: &'a Path,
    workspace: &'a Path,
    program: &'a DockerProgram,
    selected_kinds: &'a [DockerCleanupKind],
    cancellation: Arc<AtomicBool>,
    tracked_child: TrackedDockerChild,
}

impl Drop for DockerOperationLease<'_> {
    fn drop(&mut self) {
        self.state.cancellation.store(false, Ordering::Release);
        self.state.running.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DockerUsageKind {
    Images,
    Containers,
    Volumes,
    BuildCache,
}

impl DockerUsageKind {
    fn label(self) -> &'static str {
        match self {
            Self::Images => "이미지",
            Self::Containers => "컨테이너",
            Self::Volumes => "볼륨",
            Self::BuildCache => "빌드 캐시",
        }
    }

    fn order(self) -> u8 {
        match self {
            Self::Images => 0,
            Self::Containers => 1,
            Self::Volumes => 2,
            Self::BuildCache => 3,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerUsageCategory {
    kind: DockerUsageKind,
    label: &'static str,
    total_count: u64,
    active_count: u64,
    size_bytes: u64,
    reclaimable_bytes: u64,
    size_display: String,
    reclaimable_display: String,
    cleanup_supported: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerCleanupHistorySummary {
    operation_id: String,
    finished_at_unix_ms: u64,
    kinds: Vec<DockerCleanupKind>,
    outcome: DockerCleanupOutcome,
    reported_reclaimed_bytes: u64,
    message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DockerCleanupOutcome {
    Completed,
    Partial,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerManagementStatus {
    enabled: bool,
    cli_installed: Option<bool>,
    daemon_running: Option<bool>,
    busy: bool,
    detail: String,
    client_version: Option<String>,
    server_version: Option<String>,
    captured_at_unix_ms: Option<u64>,
    total_size_bytes: u64,
    reclaimable_bytes: u64,
    categories: Vec<DockerUsageCategory>,
    last_cleanup: Option<DockerCleanupHistorySummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerAssistantContext {
    enabled: bool,
    available: bool,
    detail: String,
    captured_at_unix_ms: Option<u64>,
    total_size_bytes: u64,
    reclaimable_bytes: u64,
    volumes_excluded: bool,
    categories: Vec<DockerUsageCategory>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DockerCleanupKind {
    BuildCache,
    DanglingImages,
    StoppedContainers,
}

impl DockerCleanupKind {
    fn label(self) -> &'static str {
        match self {
            Self::BuildCache => "7일 이상 사용하지 않은 빌드 캐시",
            Self::DanglingImages => "7일 이상 된 매달린 이미지",
            Self::StoppedContainers => "7일 이상 된 중지 컨테이너",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::BuildCache => "다음 빌드 때 다시 만들어질 수 있습니다.",
            Self::DanglingImages => {
                "태그가 없는 이미지 계층이며 필요하면 다시 내려받거나 빌드해야 합니다."
            }
            Self::StoppedContainers => "중지된 컨테이너의 쓰기 계층은 복원할 수 없습니다.",
        }
    }

    fn usage_kind(self) -> DockerUsageKind {
        match self {
            Self::BuildCache => DockerUsageKind::BuildCache,
            Self::DanglingImages => DockerUsageKind::Images,
            Self::StoppedContainers => DockerUsageKind::Containers,
        }
    }
}

const CLEANUP_KINDS: [DockerCleanupKind; 3] = [
    DockerCleanupKind::BuildCache,
    DockerCleanupKind::DanglingImages,
    DockerCleanupKind::StoppedContainers,
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerCleanupPreviewItem {
    kind: DockerCleanupKind,
    label: &'static str,
    description: &'static str,
    estimated_reclaimable_bytes: u64,
    estimate_display: String,
    command_display: String,
    default_selected: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerCleanupPreview {
    preview_id: String,
    created_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    items: Vec<DockerCleanupPreviewItem>,
    volumes_excluded: bool,
}

#[derive(Clone, Debug)]
struct StoredDockerCleanupPreview {
    preview_id: String,
    expires_at_unix_ms: u64,
    allowed_kinds: Vec<DockerCleanupKind>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecuteDockerCleanupRequest {
    preview_id: String,
    selected_kinds: Vec<DockerCleanupKind>,
    irreversible_acknowledged: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerCleanupStepResult {
    kind: DockerCleanupKind,
    label: &'static str,
    completed: bool,
    cancelled: bool,
    reported_reclaimed_bytes: u64,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerCleanupResult {
    operation_id: String,
    outcome: DockerCleanupOutcome,
    started_at_unix_ms: u64,
    finished_at_unix_ms: u64,
    reported_reclaimed_bytes: u64,
    steps: Vec<DockerCleanupStepResult>,
    status_after: DockerManagementStatus,
    history_recorded: bool,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerCleanupProgress {
    message: String,
    completed_steps: usize,
    total_steps: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawDockerDfRow {
    active: String,
    reclaimable: String,
    size: String,
    total_count: String,
    #[serde(rename = "Type")]
    kind: String,
}

#[derive(Clone, Debug)]
enum DockerProgram {
    Direct(PathBuf),
}

impl DockerProgram {
    fn command(&self) -> Command {
        match self {
            Self::Direct(path) => Command::new(path),
        }
    }
}

#[derive(Debug)]
enum DockerCommandError {
    Cancelled,
    Timeout,
    OutputTooLarge,
    Failed(String),
    Launch(String),
}

impl DockerCommandError {
    fn user_message(&self) -> String {
        match self {
            Self::Cancelled => {
                "Docker 작업 중단을 요청했습니다. 이미 정리된 항목은 되돌릴 수 없습니다".to_owned()
            }
            Self::Timeout => {
                "Docker 응답 제한 시간을 넘었습니다. Docker Desktop 상태를 확인해 주세요".to_owned()
            }
            Self::OutputTooLarge => "Docker 출력이 안전한 처리 한도를 넘었습니다".to_owned(),
            Self::Failed(message) => format!("Docker가 작업을 완료하지 못했습니다. {message}"),
            Self::Launch(message) => format!("Docker CLI를 시작하지 못했습니다. {message}"),
        }
    }
}

#[tauri::command]
pub(crate) async fn get_docker_management_status(
    app: AppHandle,
    state: State<'_, DockerManagerState>,
) -> Result<DockerManagementStatus, String> {
    status_for_app(
        &app,
        state.running.load(Ordering::Acquire),
        Arc::clone(&state.cancellation),
    )
    .await
}

#[tauri::command]
pub(crate) async fn set_docker_management_enabled(
    app: AppHandle,
    state: State<'_, DockerManagerState>,
    enabled: bool,
) -> Result<DockerManagementStatus, String> {
    if state.running.load(Ordering::Acquire) {
        return Err("Docker 정리 중에는 이 설정을 바꿀 수 없습니다".to_owned());
    }
    let database_path = database_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || set_enabled_at(&database_path, enabled))
        .await
        .map_err(|error| format!("Docker 설정 저장 작업이 중단됐습니다: {error}"))??;
    *state
        .preview
        .lock()
        .map_err(|_| "Docker 정리 미리보기 잠금이 손상됐습니다".to_owned())? = None;
    state.cancellation.store(false, Ordering::Release);
    status_for_app(&app, false, Arc::clone(&state.cancellation)).await
}

#[tauri::command]
pub(crate) async fn create_docker_cleanup_preview(
    app: AppHandle,
    state: State<'_, DockerManagerState>,
) -> Result<DockerCleanupPreview, String> {
    if state.running.load(Ordering::Acquire) {
        return Err("이미 Docker 정리 작업이 진행 중입니다".to_owned());
    }
    let status = status_for_app(&app, false, Arc::new(AtomicBool::new(false))).await?;
    if !status.enabled {
        return Err("설정에서 Docker 용량 관리를 먼저 켜 주세요".to_owned());
    }
    if status.cli_installed != Some(true) || status.daemon_running != Some(true) {
        return Err(status.detail);
    }
    let preview = build_cleanup_preview(&status)?;
    let stored = StoredDockerCleanupPreview {
        preview_id: preview.preview_id.clone(),
        expires_at_unix_ms: preview.expires_at_unix_ms,
        allowed_kinds: preview.items.iter().map(|item| item.kind).collect(),
    };
    let mut preview_guard = state
        .preview
        .lock()
        .map_err(|_| "Docker 정리 미리보기 잠금이 손상됐습니다".to_owned())?;
    if state.running.load(Ordering::Acquire) {
        return Err("Docker 정리가 시작되어 미리보기를 만들지 않았습니다".to_owned());
    }
    *preview_guard = Some(stored);
    Ok(preview)
}

#[tauri::command]
pub(crate) async fn execute_docker_cleanup(
    app: AppHandle,
    state: State<'_, DockerManagerState>,
    request: ExecuteDockerCleanupRequest,
) -> Result<DockerCleanupResult, String> {
    validate_execute_request(&request)?;
    if state.closed.load(Ordering::Acquire) {
        return Err("앱이 종료 중이어서 Docker 정리를 시작하지 않았습니다".to_owned());
    }
    state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "이미 Docker 정리 작업이 진행 중입니다".to_owned())?;
    let _lease = DockerOperationLease { state: &state };
    state.cancellation.store(false, Ordering::Release);

    take_matching_preview(&state, &request)?;
    let database_path = database_path(&app)?;
    let workspace = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Docker 작업 폴더를 찾지 못했습니다: {error}"))?
        .join("docker-tools");
    let enabled = {
        let path = database_path.clone();
        tauri::async_runtime::spawn_blocking(move || load_enabled_at(&path))
            .await
            .map_err(|error| format!("Docker 설정 확인 작업이 중단됐습니다: {error}"))??
    };
    if !enabled {
        return Err("Docker 용량 관리 설정이 꺼져 있어 정리를 시작하지 않았습니다".to_owned());
    }
    let program = find_docker_program().ok_or_else(|| {
        "Docker CLI를 찾지 못했습니다. Docker를 설치한 뒤 다시 확인해 주세요".to_owned()
    })?;
    let selected_kinds = request.selected_kinds;
    let cancellation = Arc::clone(&state.cancellation);
    let closed = Arc::clone(&state.closed);
    let active_cleanup_child = Arc::clone(&state.active_cleanup_child);
    let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_cleanup_operation(DockerCleanupRun {
            app: &task_app,
            database_path: &database_path,
            workspace: &workspace,
            program: &program,
            selected_kinds: &selected_kinds,
            cancellation,
            tracked_child: TrackedDockerChild {
                closed,
                child: active_cleanup_child,
            },
        })
    })
    .await
    .map_err(|error| format!("Docker 정리 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) fn cancel_docker_cleanup(state: State<'_, DockerManagerState>) -> bool {
    if !state.running.load(Ordering::Acquire) {
        return false;
    }
    state.cancellation.store(true, Ordering::Release);
    true
}

pub(crate) fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<DockerManagerState>() {
        state.closed.store(true, Ordering::Release);
        state.cancellation.store(true, Ordering::Release);
        stop_tracked_child(&state.active_cleanup_child);
    }
}

pub(crate) async fn assistant_context(app: &AppHandle) -> Result<DockerAssistantContext, String> {
    let busy = app
        .try_state::<DockerManagerState>()
        .is_some_and(|state| state.running.load(Ordering::Acquire));
    let status = status_for_app(app, busy, Arc::new(AtomicBool::new(false))).await?;
    Ok(DockerAssistantContext {
        enabled: status.enabled,
        available: status.cli_installed == Some(true) && status.daemon_running == Some(true),
        detail: status.detail,
        captured_at_unix_ms: status.captured_at_unix_ms,
        total_size_bytes: status.total_size_bytes,
        reclaimable_bytes: status.reclaimable_bytes,
        volumes_excluded: true,
        categories: status.categories,
    })
}

async fn status_for_app(
    app: &AppHandle,
    busy: bool,
    cancellation: Arc<AtomicBool>,
) -> Result<DockerManagementStatus, String> {
    let database_path = database_path(app)?;
    let workspace = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Docker 상태 확인 폴더를 찾지 못했습니다: {error}"))?
        .join("docker-tools");
    tauri::async_runtime::spawn_blocking(move || {
        let enabled = load_enabled_at(&database_path)?;
        let last_cleanup = load_last_cleanup_at(&database_path)?;
        if !enabled {
            return Ok(disabled_status(busy, last_cleanup));
        }
        Ok(probe_docker_status(
            busy,
            last_cleanup,
            &workspace,
            cancellation,
        ))
    })
    .await
    .map_err(|error| format!("Docker 상태 확인 작업이 중단됐습니다: {error}"))?
}

fn disabled_status(
    busy: bool,
    last_cleanup: Option<DockerCleanupHistorySummary>,
) -> DockerManagementStatus {
    DockerManagementStatus {
        enabled: false,
        cli_installed: None,
        daemon_running: None,
        busy,
        detail:
            "Docker를 사용하는 경우에만 설정에서 켜세요. 현재는 Docker CLI를 확인하지 않습니다."
                .to_owned(),
        client_version: None,
        server_version: None,
        captured_at_unix_ms: None,
        total_size_bytes: 0,
        reclaimable_bytes: 0,
        categories: Vec::new(),
        last_cleanup,
    }
}

fn probe_docker_status(
    busy: bool,
    last_cleanup: Option<DockerCleanupHistorySummary>,
    workspace: &Path,
    cancellation: Arc<AtomicBool>,
) -> DockerManagementStatus {
    let Some(program) = find_docker_program() else {
        return DockerManagementStatus {
            enabled: true,
            cli_installed: Some(false),
            daemon_running: Some(false),
            busy,
            detail: "Docker CLI를 찾지 못했습니다. Docker를 설치한 뒤 다시 확인해 주세요."
                .to_owned(),
            client_version: None,
            server_version: None,
            captured_at_unix_ms: Some(unix_time_ms()),
            total_size_bytes: 0,
            reclaimable_bytes: 0,
            categories: Vec::new(),
            last_cleanup,
        };
    };

    let client_version = run_docker_command(
        &program,
        &["--version"],
        workspace,
        STATUS_TIMEOUT,
        Arc::clone(&cancellation),
        None,
    )
    .ok()
    .and_then(|output| parse_client_version(&output));
    let server_version = match run_docker_command(
        &program,
        &["version", "--format", "{{.Server.Version}}"],
        workspace,
        STATUS_TIMEOUT,
        Arc::clone(&cancellation),
        None,
    ) {
        Ok(output) if !output.trim().is_empty() => Some(output.trim().to_owned()),
        Err(error) => {
            return DockerManagementStatus {
                enabled: true,
                cli_installed: Some(true),
                daemon_running: Some(false),
                busy,
                detail: daemon_error_detail(&error),
                client_version,
                server_version: None,
                captured_at_unix_ms: Some(unix_time_ms()),
                total_size_bytes: 0,
                reclaimable_bytes: 0,
                categories: Vec::new(),
                last_cleanup,
            };
        }
        Ok(_) => None,
    };

    match run_docker_command(
        &program,
        &["system", "df", "--format", "json"],
        workspace,
        STATUS_TIMEOUT,
        cancellation,
        None,
    )
    .and_then(|output| parse_docker_df(&output).map_err(DockerCommandError::Failed))
    {
        Ok(categories) => {
            let (total_size_bytes, reclaimable_bytes) = summarize_categories(&categories);
            DockerManagementStatus {
                enabled: true,
                cli_installed: Some(true),
                daemon_running: Some(true),
                busy,
                detail: "Docker가 보고한 현재 사용량입니다.".to_owned(),
                client_version,
                server_version,
                captured_at_unix_ms: Some(unix_time_ms()),
                total_size_bytes,
                reclaimable_bytes,
                categories,
                last_cleanup,
            }
        }
        Err(error) => DockerManagementStatus {
            enabled: true,
            cli_installed: Some(true),
            daemon_running: Some(true),
            busy,
            detail: format!("Docker 사용량을 읽지 못했습니다. {}", error.user_message()),
            client_version,
            server_version,
            captured_at_unix_ms: Some(unix_time_ms()),
            total_size_bytes: 0,
            reclaimable_bytes: 0,
            categories: Vec::new(),
            last_cleanup,
        },
    }
}

fn summarize_categories(categories: &[DockerUsageCategory]) -> (u64, u64) {
    categories.iter().fold((0_u64, 0_u64), |totals, category| {
        (
            totals.0.saturating_add(category.size_bytes),
            if category.cleanup_supported {
                totals.1.saturating_add(category.reclaimable_bytes)
            } else {
                totals.1
            },
        )
    })
}

fn build_cleanup_preview(status: &DockerManagementStatus) -> Result<DockerCleanupPreview, String> {
    let created_at_unix_ms = unix_time_ms();
    let expires_at_unix_ms = created_at_unix_ms.saturating_add(PREVIEW_TTL.as_millis() as u64);
    let preview_id = generate_id("Docker 정리 미리보기")?;
    let items = CLEANUP_KINDS
        .into_iter()
        .map(|kind| {
            let estimate = status
                .categories
                .iter()
                .find(|category| category.kind == kind.usage_kind())
                .map_or(0, |category| category.reclaimable_bytes);
            DockerCleanupPreviewItem {
                kind,
                label: kind.label(),
                description: kind.description(),
                estimated_reclaimable_bytes: estimate,
                estimate_display: format_decimal_bytes(estimate),
                command_display: display_command(kind),
                default_selected: kind == DockerCleanupKind::BuildCache && estimate > 0,
            }
        })
        .collect();
    Ok(DockerCleanupPreview {
        preview_id,
        created_at_unix_ms,
        expires_at_unix_ms,
        items,
        volumes_excluded: true,
    })
}

fn validate_execute_request(request: &ExecuteDockerCleanupRequest) -> Result<(), String> {
    if request.preview_id.len() != 32
        || !request
            .preview_id
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err("Docker 정리 미리보기 번호가 올바르지 않습니다".to_owned());
    }
    if !request.irreversible_acknowledged {
        return Err("Docker 정리는 휴지통을 거치지 않는다는 확인이 필요합니다".to_owned());
    }
    if request.selected_kinds.is_empty() || request.selected_kinds.len() > CLEANUP_KINDS.len() {
        return Err("정리할 Docker 항목을 하나 이상 선택해 주세요".to_owned());
    }
    let unique = request
        .selected_kinds
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if unique.len() != request.selected_kinds.len() {
        return Err("같은 Docker 정리 항목이 중복되었습니다".to_owned());
    }
    Ok(())
}

fn take_matching_preview(
    state: &DockerManagerState,
    request: &ExecuteDockerCleanupRequest,
) -> Result<(), String> {
    let mut guard = state
        .preview
        .lock()
        .map_err(|_| "Docker 정리 미리보기 잠금이 손상됐습니다".to_owned())?;
    let Some(preview) = guard.as_ref() else {
        return Err("Docker 정리 미리보기가 없습니다. 사용량을 다시 확인해 주세요".to_owned());
    };
    if preview.preview_id != request.preview_id {
        return Err("현재 Docker 정리 미리보기와 번호가 다릅니다".to_owned());
    }
    if preview.expires_at_unix_ms < unix_time_ms() {
        *guard = None;
        return Err("Docker 정리 미리보기가 만료됐습니다. 사용량을 다시 확인해 주세요".to_owned());
    }
    if request
        .selected_kinds
        .iter()
        .any(|kind| !preview.allowed_kinds.contains(kind))
    {
        return Err("미리보기에 없는 Docker 정리 항목이 포함됐습니다".to_owned());
    }
    *guard = None;
    Ok(())
}

fn run_cleanup_operation(run: DockerCleanupRun<'_>) -> Result<DockerCleanupResult, String> {
    let DockerCleanupRun {
        app,
        database_path,
        workspace,
        program,
        selected_kinds,
        cancellation,
        tracked_child,
    } = run;
    let operation_id = generate_id("Docker 정리 작업")?;
    let started_at_unix_ms = unix_time_ms();
    let total_steps = selected_kinds.len();
    let mut steps = Vec::with_capacity(total_steps);

    for (index, kind) in selected_kinds.iter().copied().enumerate() {
        if cancellation.load(Ordering::Acquire) {
            steps.push(DockerCleanupStepResult {
                kind,
                label: kind.label(),
                completed: false,
                cancelled: true,
                reported_reclaimed_bytes: 0,
                message: "이 단계는 시작하지 않았습니다".to_owned(),
            });
            break;
        }
        emit_progress(
            app,
            DockerCleanupProgress {
                message: format!("{} 정리 중", kind.label()),
                completed_steps: index,
                total_steps,
            },
        );
        let args = cleanup_command_args(kind);
        match run_docker_command(
            program,
            &args,
            workspace,
            CLEANUP_TIMEOUT,
            Arc::clone(&cancellation),
            Some(tracked_child.clone()),
        ) {
            Ok(output) => steps.push(DockerCleanupStepResult {
                kind,
                label: kind.label(),
                completed: true,
                cancelled: false,
                reported_reclaimed_bytes: parse_reported_reclaimed_bytes(&output),
                message: "Docker가 이 정리 단계를 완료했습니다".to_owned(),
            }),
            Err(error) => {
                let cancelled = matches!(error, DockerCommandError::Cancelled);
                steps.push(DockerCleanupStepResult {
                    kind,
                    label: kind.label(),
                    completed: false,
                    cancelled,
                    reported_reclaimed_bytes: 0,
                    message: error.user_message(),
                });
                break;
            }
        }
    }

    emit_progress(
        app,
        DockerCleanupProgress {
            message: "Docker 사용량 다시 확인 중".to_owned(),
            completed_steps: steps.iter().filter(|step| step.completed).count(),
            total_steps,
        },
    );
    let last_cleanup_before = load_last_cleanup_at(database_path).ok().flatten();
    let mut status_after = probe_docker_status(
        true,
        last_cleanup_before,
        workspace,
        Arc::new(AtomicBool::new(false)),
    );
    let outcome = cleanup_outcome(&steps, total_steps);
    let reported_reclaimed_bytes = steps.iter().fold(0_u64, |total, step| {
        total.saturating_add(step.reported_reclaimed_bytes)
    });
    let finished_at_unix_ms = unix_time_ms();
    let message = cleanup_result_message(outcome, &steps, total_steps);
    let history = DockerCleanupHistorySummary {
        operation_id: operation_id.clone(),
        finished_at_unix_ms,
        kinds: selected_kinds.to_vec(),
        outcome,
        reported_reclaimed_bytes,
        message: message.clone(),
    };
    let history_recorded = record_cleanup_history_at(database_path, &history).is_ok();
    status_after.busy = false;
    if history_recorded {
        status_after.last_cleanup = Some(history);
    }
    Ok(DockerCleanupResult {
        operation_id,
        outcome,
        started_at_unix_ms,
        finished_at_unix_ms,
        reported_reclaimed_bytes,
        steps,
        status_after,
        history_recorded,
        message,
    })
}

fn cleanup_outcome(
    steps: &[DockerCleanupStepResult],
    expected_steps: usize,
) -> DockerCleanupOutcome {
    if steps.iter().any(|step| step.cancelled) {
        if steps.iter().any(|step| step.completed) {
            DockerCleanupOutcome::Partial
        } else {
            DockerCleanupOutcome::Cancelled
        }
    } else if steps.len() == expected_steps && steps.iter().all(|step| step.completed) {
        DockerCleanupOutcome::Completed
    } else if steps.iter().any(|step| step.completed) {
        DockerCleanupOutcome::Partial
    } else {
        DockerCleanupOutcome::Failed
    }
}

fn cleanup_result_message(
    outcome: DockerCleanupOutcome,
    steps: &[DockerCleanupStepResult],
    expected_steps: usize,
) -> String {
    let completed = steps.iter().filter(|step| step.completed).count();
    match outcome {
        DockerCleanupOutcome::Completed => {
            format!("선택한 Docker 정리 {completed}개를 완료했습니다")
        }
        DockerCleanupOutcome::Partial => format!(
            "Docker 정리 {completed}/{expected_steps}개를 완료했습니다. 완료된 단계는 되돌릴 수 없습니다"
        ),
        DockerCleanupOutcome::Cancelled => {
            "Docker 정리를 시작하기 전 또는 실행 중에 중단했습니다. 사용량을 다시 확인했습니다"
                .to_owned()
        }
        DockerCleanupOutcome::Failed => steps.last().map_or_else(
            || "Docker 정리를 시작하지 못했습니다".to_owned(),
            |step| step.message.clone(),
        ),
    }
}

fn emit_progress(app: &AppHandle, progress: DockerCleanupProgress) {
    let _ = app.emit(DOCKER_PROGRESS_EVENT, progress);
}

fn cleanup_command_args(kind: DockerCleanupKind) -> Vec<&'static str> {
    match kind {
        DockerCleanupKind::BuildCache => {
            vec![
                "builder",
                "prune",
                "--force",
                "--filter",
                CLEANUP_AGE_FILTER,
            ]
        }
        DockerCleanupKind::DanglingImages => {
            vec!["image", "prune", "--force", "--filter", CLEANUP_AGE_FILTER]
        }
        DockerCleanupKind::StoppedContainers => vec![
            "container",
            "prune",
            "--force",
            "--filter",
            CLEANUP_AGE_FILTER,
        ],
    }
}

fn display_command(kind: DockerCleanupKind) -> String {
    format!("docker {}", cleanup_command_args(kind).join(" "))
}

struct DockerOutputFiles {
    output_path: PathBuf,
    error_path: PathBuf,
}

impl Drop for DockerOutputFiles {
    fn drop(&mut self) {
        cleanup_output_files(&self.output_path, &self.error_path);
    }
}

fn run_docker_command(
    program: &DockerProgram,
    args: &[&str],
    workspace: &Path,
    timeout: Duration,
    cancellation: Arc<AtomicBool>,
    tracked_child: Option<TrackedDockerChild>,
) -> Result<String, DockerCommandError> {
    fs::create_dir_all(workspace)
        .map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())))?;
    let nonce = generate_id("Docker 명령 출력").map_err(DockerCommandError::Launch)?;
    let output_path = workspace.join(format!("docker-{nonce}.out"));
    let error_path = workspace.join(format!("docker-{nonce}.err"));
    let _output_files = DockerOutputFiles {
        output_path: output_path.clone(),
        error_path: error_path.clone(),
    };
    let output_file = File::create(&output_path)
        .map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())))?;
    let error_file = File::create(&error_path)
        .map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())))?;
    let mut command = program.command();
    command
        .args(args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output_file))
        .stderr(Stdio::from(error_file));
    let mut local_child = None;
    if let Some(tracker) = tracked_child.as_ref() {
        let mut guard = tracker.child.lock().map_err(|_| {
            DockerCommandError::Launch("Docker 프로세스 잠금이 손상됐습니다".to_owned())
        })?;
        if tracker.closed.load(Ordering::Acquire) || cancellation.load(Ordering::Acquire) {
            return Err(DockerCommandError::Cancelled);
        }
        if guard.is_some() {
            return Err(DockerCommandError::Launch(
                "이전 Docker 프로세스가 아직 종료되지 않았습니다".to_owned(),
            ));
        }
        *guard = Some(
            command
                .spawn()
                .map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())))?,
        );
    } else {
        local_child = Some(
            command
                .spawn()
                .map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())))?,
        );
    }
    let started = Instant::now();
    let status = loop {
        if cancellation.load(Ordering::Acquire) {
            stop_running_child(&mut local_child, &tracked_child);
            return Err(DockerCommandError::Cancelled);
        }
        if file_exceeds_limit(&output_path) || file_exceeds_limit(&error_path) {
            stop_running_child(&mut local_child, &tracked_child);
            return Err(DockerCommandError::OutputTooLarge);
        }
        match with_running_child(&mut local_child, &tracked_child, Child::try_wait) {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                stop_running_child(&mut local_child, &tracked_child);
                return Err(DockerCommandError::Timeout);
            }
            Err(error) => {
                stop_running_child(&mut local_child, &tracked_child);
                return Err(error);
            }
        }
    };
    release_running_child(&mut local_child, &tracked_child);
    let output = read_bounded_file(&output_path);
    let error = read_bounded_tail(&error_path).unwrap_or_default();
    if !status.success() {
        return Err(DockerCommandError::Failed(sanitize_error(&error)));
    }
    output.map_err(|error| DockerCommandError::Launch(sanitize_error(&error)))
}

fn with_running_child<T>(
    local_child: &mut Option<Child>,
    tracked_child: &Option<TrackedDockerChild>,
    action: impl FnOnce(&mut Child) -> std::io::Result<T>,
) -> Result<T, DockerCommandError> {
    if let Some(child) = local_child.as_mut() {
        return action(child)
            .map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())));
    }
    let Some(tracker) = tracked_child else {
        return Err(DockerCommandError::Launch(
            "실행 중인 Docker 프로세스를 찾지 못했습니다".to_owned(),
        ));
    };
    let mut guard = tracker.child.lock().map_err(|_| {
        DockerCommandError::Launch("Docker 프로세스 잠금이 손상됐습니다".to_owned())
    })?;
    let Some(child) = guard.as_mut() else {
        return Err(DockerCommandError::Cancelled);
    };
    action(child).map_err(|error| DockerCommandError::Launch(sanitize_error(&error.to_string())))
}

fn stop_running_child(local_child: &mut Option<Child>, tracked_child: &Option<TrackedDockerChild>) {
    if let Some(mut child) = local_child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Some(tracker) = tracked_child {
        stop_tracked_child(&tracker.child);
    }
}

fn release_running_child(
    local_child: &mut Option<Child>,
    tracked_child: &Option<TrackedDockerChild>,
) {
    local_child.take();
    if let Some(tracker) = tracked_child
        && let Ok(mut guard) = tracker.child.lock()
    {
        guard.take();
    }
}

fn stop_tracked_child(child_slot: &Arc<Mutex<Option<Child>>>) -> bool {
    if let Ok(mut guard) = child_slot.lock()
        && let Some(mut child) = guard.take()
    {
        let _ = child.kill();
        return child.wait().is_ok();
    }
    false
}

fn file_exceeds_limit(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.len() > OUTPUT_LIMIT_BYTES)
}

fn read_bounded_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > OUTPUT_LIMIT_BYTES {
        return Err("출력 한도를 넘었습니다".to_owned());
    }
    let mut bytes = Vec::new();
    file.take(OUTPUT_LIMIT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > OUTPUT_LIMIT_BYTES {
        return Err("출력 한도를 넘었습니다".to_owned());
    }
    String::from_utf8(bytes).map_err(|_| "Docker 출력이 UTF-8 텍스트가 아닙니다".to_owned())
}

fn read_bounded_tail(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let length = file.metadata().map_err(|error| error.to_string())?.len();
    if length > OUTPUT_LIMIT_BYTES {
        return Err("오류 출력 한도를 넘었습니다".to_owned());
    }
    let start = length.saturating_sub(16 * 1024);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn cleanup_output_files(output_path: &Path, error_path: &Path) {
    let _ = fs::remove_file(output_path);
    let _ = fs::remove_file(error_path);
}

fn parse_docker_df(output: &str) -> Result<Vec<DockerUsageCategory>, String> {
    let mut categories = Vec::new();
    for line in output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(16)
    {
        let row = serde_json::from_str::<RawDockerDfRow>(line)
            .map_err(|error| format!("Docker 사용량 JSON 형식이 올바르지 않습니다: {error}"))?;
        let kind = match row.kind.as_str() {
            "Images" => DockerUsageKind::Images,
            "Containers" => DockerUsageKind::Containers,
            "Local Volumes" => DockerUsageKind::Volumes,
            "Build Cache" => DockerUsageKind::BuildCache,
            _ => continue,
        };
        categories.push(DockerUsageCategory {
            kind,
            label: kind.label(),
            total_count: row.total_count.parse().unwrap_or(0),
            active_count: row.active.parse().unwrap_or(0),
            size_bytes: parse_decimal_bytes(&row.size)?,
            reclaimable_bytes: parse_decimal_bytes(&row.reclaimable)?,
            size_display: row.size,
            reclaimable_display: row.reclaimable,
            cleanup_supported: kind != DockerUsageKind::Volumes,
        });
    }
    categories.sort_by_key(|category| category.kind.order());
    if categories.is_empty() {
        return Err("Docker가 사용량 범주를 반환하지 않았습니다".to_owned());
    }
    Ok(categories)
}

fn parse_decimal_bytes(value: &str) -> Result<u64, String> {
    let token = value
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .split('(')
        .next()
        .unwrap_or_default()
        .trim();
    let number_end = token
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_digit() && character != '.').then_some(index)
        })
        .unwrap_or(token.len());
    let (number, unit) = token.split_at(number_end);
    let amount = number
        .parse::<f64>()
        .map_err(|_| format!("Docker 용량 값 `{token}`을 읽을 수 없습니다"))?;
    if !amount.is_finite() || amount < 0.0 {
        return Err("Docker 용량 값이 올바르지 않습니다".to_owned());
    }
    let multiplier = match unit.to_ascii_lowercase().as_str() {
        "b" | "" => 1_f64,
        "kb" => 1_000_f64,
        "mb" => 1_000_000_f64,
        "gb" => 1_000_000_000_f64,
        "tb" => 1_000_000_000_000_f64,
        "kib" => 1_024_f64,
        "mib" => 1_048_576_f64,
        "gib" => 1_073_741_824_f64,
        "tib" => 1_099_511_627_776_f64,
        _ => return Err(format!("Docker 용량 단위 `{unit}`를 읽을 수 없습니다")),
    };
    Ok((amount * multiplier).round().clamp(0.0, u64::MAX as f64) as u64)
}

fn parse_reported_reclaimed_bytes(output: &str) -> u64 {
    output
        .lines()
        .rev()
        .find_map(|line| {
            let (_, value) = line.split_once(':')?;
            line.to_ascii_lowercase()
                .contains("reclaimed")
                .then(|| parse_decimal_bytes(value).ok())
                .flatten()
        })
        .unwrap_or(0)
}

fn parse_client_version(output: &str) -> Option<String> {
    output
        .trim()
        .strip_prefix("Docker version ")
        .and_then(|rest| rest.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn daemon_error_detail(error: &DockerCommandError) -> String {
    match error {
        DockerCommandError::Failed(_) => {
            "Docker CLI는 있지만 Docker 서비스에 연결하지 못했습니다. Docker Desktop 또는 Docker Engine을 실행한 뒤 다시 확인해 주세요.".to_owned()
        }
        _ => error.user_message(),
    }
}

fn sanitize_error(value: &str) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = single_line.chars();
    let bounded = chars.by_ref().take(ERROR_DISPLAY_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{bounded}…")
    } else if bounded.is_empty() {
        "자세한 오류 정보가 없습니다".to_owned()
    } else {
        bounded
    }
}

fn format_decimal_bytes(bytes: u64) -> String {
    const UNITS: [(&str, f64); 5] = [
        ("TB", 1_000_000_000_000_f64),
        ("GB", 1_000_000_000_f64),
        ("MB", 1_000_000_f64),
        ("kB", 1_000_f64),
        ("B", 1_f64),
    ];
    for (unit, divisor) in UNITS {
        if bytes as f64 >= divisor || unit == "B" {
            let value = bytes as f64 / divisor;
            return if value >= 10.0 || unit == "B" {
                format!("{value:.0} {unit}")
            } else {
                format!("{value:.1} {unit}")
            };
        }
    }
    "0 B".to_owned()
}

fn find_docker_program() -> Option<DockerProgram> {
    docker_program_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(DockerProgram::Direct)
}

fn docker_program_candidates() -> Vec<PathBuf> {
    let executable_name = if cfg!(windows) {
        "docker.exe"
    } else {
        "docker"
    };
    let mut candidates = env::var_os("PATH")
        .map(|path| {
            env::split_paths(&path)
                .map(|directory| directory.join(executable_name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    #[cfg(windows)]
    {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("DockerDesktop")
                    .join("resources")
                    .join("bin")
                    .join("docker.exe"),
            );
        }
        if let Some(program_files) = env::var_os("ProgramFiles") {
            candidates.push(
                PathBuf::from(program_files)
                    .join("Docker")
                    .join("Docker")
                    .join("resources")
                    .join("bin")
                    .join("docker.exe"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/usr/local/bin/docker"));
        if let Some(home) = env::var_os("HOME") {
            candidates.push(
                PathBuf::from(home)
                    .join(".docker")
                    .join("bin")
                    .join("docker"),
            );
        }
        candidates.push(PathBuf::from(
            "/Applications/Docker.app/Contents/Resources/bin/docker",
        ));
    }

    candidates
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("개발 도구 설정 폴더를 찾지 못했습니다: {error}"))?
        .join(DATABASE_FILE_NAME))
}

fn open_database(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("개발 도구 설정 폴더를 만들지 못했습니다: {error}"))?;
    }
    let connection = Connection::open(path)
        .map_err(|error| format!("개발 도구 설정 저장소를 열지 못했습니다: {error}"))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| format!("개발 도구 설정 저널을 준비하지 못했습니다: {error}"))?;
    connection
        .pragma_update(None, "synchronous", "NORMAL")
        .map_err(|error| format!("개발 도구 설정 동기화를 준비하지 못했습니다: {error}"))?;
    connection
        .pragma_update(None, "secure_delete", true)
        .map_err(|error| format!("개발 도구 설정 삭제 정책을 준비하지 못했습니다: {error}"))?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                bool_value INTEGER NOT NULL CHECK (bool_value IN (0, 1)),
                updated_at_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS docker_cleanup_history (
                operation_id TEXT PRIMARY KEY NOT NULL,
                finished_at_unix_ms INTEGER NOT NULL,
                kinds_json TEXT NOT NULL,
                outcome TEXT NOT NULL,
                reported_reclaimed_bytes INTEGER NOT NULL,
                message TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS docker_cleanup_history_finished
             ON docker_cleanup_history(finished_at_unix_ms DESC);",
        )
        .map_err(|error| format!("개발 도구 설정 구조를 준비하지 못했습니다: {error}"))?;
    Ok(connection)
}

fn load_enabled_at(path: &Path) -> Result<bool, String> {
    let connection = open_database(path)?;
    connection
        .query_row(
            "SELECT bool_value FROM app_settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|value| value.unwrap_or(0) == 1)
        .map_err(|error| format!("Docker 설정을 읽지 못했습니다: {error}"))
}

fn set_enabled_at(path: &Path, enabled: bool) -> Result<(), String> {
    let connection = open_database(path)?;
    connection
        .execute(
            "INSERT INTO app_settings(key, bool_value, updated_at_unix_ms)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
               bool_value = excluded.bool_value,
               updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![SETTINGS_KEY, i64::from(enabled), unix_time_ms_i64()?],
        )
        .map_err(|error| format!("Docker 설정을 저장하지 못했습니다: {error}"))?;
    Ok(())
}

fn load_last_cleanup_at(path: &Path) -> Result<Option<DockerCleanupHistorySummary>, String> {
    let connection = open_database(path)?;
    connection
        .query_row(
            "SELECT operation_id, finished_at_unix_ms, kinds_json, outcome,
                    reported_reclaimed_bytes, message
             FROM docker_cleanup_history
             ORDER BY finished_at_unix_ms DESC
             LIMIT 1",
            [],
            |row| {
                let kinds_json = row.get::<_, String>(2)?;
                let outcome = row.get::<_, String>(3)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    kinds_json,
                    outcome,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("Docker 정리 이력을 읽지 못했습니다: {error}"))?
        .map(
            |(operation_id, finished_at, kinds_json, outcome, reclaimed, message)| {
                Ok(DockerCleanupHistorySummary {
                    operation_id,
                    finished_at_unix_ms: non_negative(finished_at)?,
                    kinds: serde_json::from_str(&kinds_json).map_err(|error| {
                        format!("Docker 정리 이력 항목을 읽지 못했습니다: {error}")
                    })?,
                    outcome: parse_outcome(&outcome)?,
                    reported_reclaimed_bytes: non_negative(reclaimed)?,
                    message,
                })
            },
        )
        .transpose()
}

fn record_cleanup_history_at(
    path: &Path,
    history: &DockerCleanupHistorySummary,
) -> Result<(), String> {
    let connection = open_database(path)?;
    let kinds_json = serde_json::to_string(&history.kinds)
        .map_err(|error| format!("Docker 정리 이력을 준비하지 못했습니다: {error}"))?;
    connection
        .execute(
            "INSERT INTO docker_cleanup_history(
                operation_id, finished_at_unix_ms, kinds_json, outcome,
                reported_reclaimed_bytes, message
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                history.operation_id,
                i64::try_from(history.finished_at_unix_ms).unwrap_or(i64::MAX),
                kinds_json,
                outcome_wire_name(history.outcome),
                i64::try_from(history.reported_reclaimed_bytes).unwrap_or(i64::MAX),
                sanitize_error(&history.message),
            ],
        )
        .map_err(|error| format!("Docker 정리 이력을 저장하지 못했습니다: {error}"))?;
    connection
        .execute(
            "DELETE FROM docker_cleanup_history
             WHERE operation_id NOT IN (
               SELECT operation_id FROM docker_cleanup_history
               ORDER BY finished_at_unix_ms DESC
               LIMIT ?1
             )",
            params![HISTORY_LIMIT as i64],
        )
        .map_err(|error| format!("Docker 정리 이력을 정돈하지 못했습니다: {error}"))?;
    Ok(())
}

fn outcome_wire_name(outcome: DockerCleanupOutcome) -> &'static str {
    match outcome {
        DockerCleanupOutcome::Completed => "completed",
        DockerCleanupOutcome::Partial => "partial",
        DockerCleanupOutcome::Cancelled => "cancelled",
        DockerCleanupOutcome::Failed => "failed",
    }
}

fn parse_outcome(value: &str) -> Result<DockerCleanupOutcome, String> {
    match value {
        "completed" => Ok(DockerCleanupOutcome::Completed),
        "partial" => Ok(DockerCleanupOutcome::Partial),
        "cancelled" => Ok(DockerCleanupOutcome::Cancelled),
        "failed" => Ok(DockerCleanupOutcome::Failed),
        _ => Err("Docker 정리 이력 결과가 올바르지 않습니다".to_owned()),
    }
}

fn non_negative(value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "Docker 정리 이력 숫자가 올바르지 않습니다".to_owned())
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

fn unix_time_ms_i64() -> Result<i64, String> {
    i64::try_from(unix_time_ms()).map_err(|_| "현재 시각 값이 너무 큽니다".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn docker_df_json_is_parsed_and_ordered() {
        let output = concat!(
            r#"{"Active":"0","Reclaimable":"21.1GB","Size":"26.66GB","TotalCount":"222","Type":"Build Cache"}"#,
            "\n",
            r#"{"Active":"18","Reclaimable":"2.747GB (7%)","Size":"39.19GB","TotalCount":"24","Type":"Images"}"#,
            "\n",
            r#"{"Active":"11","Reclaimable":"4.627GB (92%)","Size":"5.006GB","TotalCount":"79","Type":"Local Volumes"}"#,
        );
        let categories = parse_docker_df(output).expect("parse docker df");
        assert_eq!(categories.len(), 3);
        assert_eq!(categories[0].kind, DockerUsageKind::Images);
        assert_eq!(categories[1].kind, DockerUsageKind::Volumes);
        assert_eq!(categories[2].kind, DockerUsageKind::BuildCache);
        assert_eq!(categories[0].reclaimable_bytes, 2_747_000_000);
        assert!(!categories[1].cleanup_supported);
        let (total_size, cleanup_upper_bound) = summarize_categories(&categories);
        assert_eq!(total_size, 70_856_000_000);
        assert_eq!(cleanup_upper_bound, 23_847_000_000);
    }

    #[test]
    fn decimal_and_binary_docker_sizes_are_bounded() {
        assert_eq!(parse_decimal_bytes("8.102MB (0%)").unwrap(), 8_102_000);
        assert_eq!(parse_decimal_bytes("1GiB").unwrap(), 1_073_741_824);
        assert_eq!(parse_decimal_bytes("0B").unwrap(), 0);
        assert!(parse_decimal_bytes("forever").is_err());
    }

    #[test]
    fn cleanup_commands_are_allowlisted_and_never_touch_volumes() {
        for kind in CLEANUP_KINDS {
            let args = cleanup_command_args(kind);
            assert!(args.contains(&"--force"));
            assert!(args.contains(&CLEANUP_AGE_FILTER));
            assert!(!args.contains(&"--all"));
            assert!(!args.contains(&"--volumes"));
            assert!(!args.contains(&"system"));
        }
    }

    #[test]
    fn docker_setting_defaults_off_and_persists() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.sqlite3");
        assert!(!load_enabled_at(&path).unwrap());
        set_enabled_at(&path, true).unwrap();
        assert!(load_enabled_at(&path).unwrap());
        set_enabled_at(&path, false).unwrap();
        assert!(!load_enabled_at(&path).unwrap());
    }

    #[test]
    fn preview_excludes_volumes_and_defaults_to_build_cache() {
        let status = DockerManagementStatus {
            enabled: true,
            cli_installed: Some(true),
            daemon_running: Some(true),
            busy: false,
            detail: String::new(),
            client_version: None,
            server_version: None,
            captured_at_unix_ms: Some(1),
            total_size_bytes: 30_000,
            reclaimable_bytes: 20_000,
            categories: vec![DockerUsageCategory {
                kind: DockerUsageKind::BuildCache,
                label: "빌드 캐시",
                total_count: 2,
                active_count: 0,
                size_bytes: 30_000,
                reclaimable_bytes: 20_000,
                size_display: "30kB".to_owned(),
                reclaimable_display: "20kB".to_owned(),
                cleanup_supported: true,
            }],
            last_cleanup: None,
        };
        let preview = build_cleanup_preview(&status).unwrap();
        assert!(preview.volumes_excluded);
        assert_eq!(preview.items.len(), 3);
        assert!(
            preview
                .items
                .iter()
                .find(|item| item.kind == DockerCleanupKind::BuildCache)
                .is_some_and(|item| item.default_selected)
        );
    }

    #[test]
    fn reclaimed_output_uses_the_reported_total() {
        assert_eq!(
            parse_reported_reclaimed_bytes("Deleted: a\nTotal reclaimed space: 1.84GB\n"),
            1_840_000_000
        );
    }

    #[test]
    fn tracked_cleanup_child_is_stopped_and_reaped() {
        #[cfg(windows)]
        let child = Command::new("ping.exe")
            .args(["-n", "30", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start harmless Windows wait process");
        #[cfg(not(windows))]
        let child = Command::new("sleep")
            .arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start harmless Unix wait process");

        let child_slot = Arc::new(Mutex::new(Some(child)));
        assert!(stop_tracked_child(&child_slot));
        assert!(child_slot.lock().unwrap().is_none());
    }
}
