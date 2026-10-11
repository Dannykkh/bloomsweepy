//! One opt-in metadata polling worker. Only preferences survive a restart;
//! complete snapshots and the bounded latest-change history stay in memory.
use bloomsweepy_core::{
    FileChangeEntry, FileChangeKind, FileChangeReport, FileChangeSnapshot, FileReviewState,
    ScanError, capture_file_change_snapshot, compare_file_change_snapshots,
    validate_local_directory_path,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, WebviewWindow};

const MAX_HISTORY_ENTRIES: usize = 250;
const MAX_HISTORY_BYTES: usize = 1024 * 1024;
const PAGE_SIZE: usize = 20;
const MAX_ROOT_BYTES: usize = 16 * 1024;
const MAX_PREFERENCES_BYTES: usize = 32 * 1024;
const MAX_DATABASE_BYTES: u64 = 256 * 1024;
const MAX_STORAGE_BYTES: u64 = 768 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FileChangeReviewConfig {
    pub(crate) enabled: bool,
    pub(crate) root: Option<String>,
    pub(crate) interval_minutes: u32,
}

impl Default for FileChangeReviewConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            root: None,
            interval_minutes: 15,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileChangeReviewPhase {
    Off,
    Baseline,
    Checking,
    Idle,
    Incomplete,
    Error,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileChangeReviewDisposition {
    ReviewNeeded,
    Skipped,
}

impl From<FileChangeReviewDisposition> for FileReviewState {
    fn from(value: FileChangeReviewDisposition) -> Self {
        match value {
            FileChangeReviewDisposition::ReviewNeeded => Self::ReviewNeeded,
            FileChangeReviewDisposition::Skipped => Self::Skipped,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileChangeDecisionSource {
    Rule,
    Manual,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileChangeReviewEntry {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) kind: FileChangeKind,
    pub(crate) review_state: FileReviewState,
    pub(crate) reasons: Vec<String>,
    pub(crate) is_directory: bool,
    pub(crate) logical_bytes: u64,
    pub(crate) modified_at_unix_ms: Option<u128>,
    pub(crate) decision_source: FileChangeDecisionSource,
}

impl FileChangeReviewEntry {
    fn new(id: String, change: FileChangeEntry) -> Self {
        Self {
            id,
            path: change.path,
            name: change.name,
            previous_path: change.previous_path,
            kind: change.kind,
            review_state: change.review_state,
            reasons: change.reasons,
            is_directory: change.is_directory,
            logical_bytes: change.logical_bytes,
            modified_at_unix_ms: change.modified_at_unix_ms,
            decision_source: FileChangeDecisionSource::Rule,
        }
    }

    fn retained_bytes(&self) -> usize {
        // Include serialized escaping and each allocation's bookkeeping in the
        // retained-data budget; this is a data limit, not a process RSS promise.
        self.path
            .len()
            .saturating_add(self.name.len())
            .saturating_add(self.previous_path.as_ref().map_or(0, String::len))
            .saturating_add(self.id.len())
            .saturating_add(self.reasons.iter().map(String::len).sum::<usize>())
            .saturating_mul(6)
            .saturating_add(512 + self.reasons.len().saturating_mul(32))
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileChangeReviewStatus {
    pub(crate) config: FileChangeReviewConfig,
    pub(crate) phase: FileChangeReviewPhase,
    pub(crate) baseline_entries: usize,
    pub(crate) last_checked_at_unix_ms: Option<u128>,
    pub(crate) next_check_at_unix_ms: Option<u128>,
    pub(crate) history_count: usize,
    pub(crate) review_needed_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) history_truncated: bool,
    pub(crate) error: Option<String>,
    pub(crate) revision: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileChangeReviewPage {
    pub(crate) status: FileChangeReviewStatus,
    pub(crate) entries: Vec<FileChangeReviewEntry>,
    pub(crate) offset: usize,
    pub(crate) total: usize,
}

struct ReviewInner {
    config: FileChangeReviewConfig,
    phase: FileChangeReviewPhase,
    revision: u64,
    next_id: u64,
    baseline: Option<Arc<FileChangeSnapshot>>,
    history: VecDeque<FileChangeReviewEntry>,
    history_bytes: usize,
    history_truncated: bool,
    last_checked_at_unix_ms: Option<u128>,
    next_check_at_unix_ms: Option<u128>,
    next_deadline: Option<Instant>,
    error: Option<String>,
    active: Option<Arc<AtomicBool>>,
    in_progress: bool,
    closed: bool,
}

impl Default for ReviewInner {
    fn default() -> Self {
        Self {
            config: FileChangeReviewConfig::default(),
            phase: FileChangeReviewPhase::Off,
            revision: 0,
            next_id: 0,
            baseline: None,
            history: VecDeque::new(),
            history_bytes: 0,
            history_truncated: false,
            last_checked_at_unix_ms: None,
            next_check_at_unix_ms: None,
            next_deadline: None,
            error: None,
            active: None,
            in_progress: false,
            closed: false,
        }
    }
}

impl ReviewInner {
    fn status(&self) -> FileChangeReviewStatus {
        let review_needed_count = self
            .history
            .iter()
            .filter(|entry| entry.review_state == FileReviewState::ReviewNeeded)
            .count();
        FileChangeReviewStatus {
            config: self.config.clone(),
            phase: self.phase,
            baseline_entries: self
                .baseline
                .as_ref()
                .map_or(0, |value| value.entry_count()),
            last_checked_at_unix_ms: self.last_checked_at_unix_ms,
            next_check_at_unix_ms: self.next_check_at_unix_ms,
            history_count: self.history.len(),
            review_needed_count,
            skipped_count: self.history.len() - review_needed_count,
            history_truncated: self.history_truncated,
            error: self.error.clone(),
            revision: self.revision,
        }
    }

    fn schedule(&mut self, delay: Duration) {
        self.next_deadline = Some(Instant::now() + delay);
        self.next_check_at_unix_ms = Some(now_ms().saturating_add(delay.as_millis()));
    }

    fn invalidate(&mut self) {
        if let Some(cancellation) = self.active.take() {
            cancellation.store(true, Ordering::Release);
        }
        self.revision = self.revision.wrapping_add(1);
        self.in_progress = false;
        self.baseline = None;
        self.last_checked_at_unix_ms = None;
        self.next_deadline = None;
        self.next_check_at_unix_ms = None;
    }

    fn change_config(&mut self, config: FileChangeReviewConfig) {
        let root_changed = self.config.root != config.root;
        self.invalidate();
        if root_changed {
            self.history.clear();
            self.history_bytes = 0;
            self.history_truncated = false;
        }
        self.config = config;
        self.error = None;
        self.phase = if self.config.enabled {
            self.schedule(Duration::ZERO);
            FileChangeReviewPhase::Baseline
        } else {
            FileChangeReviewPhase::Off
        };
    }

    fn retain_changes(&mut self, report: FileChangeReport) {
        self.history_truncated |= report.truncated || report.total_changes > report.changes.len();
        for change in report.changes {
            // A new observation replaces the decision for that exact latest
            // path. Rename also retires the record at its former path.
            let mut index = 0;
            while index < self.history.len() {
                let entry = &self.history[index];
                if entry.path == change.path
                    || change
                        .previous_path
                        .as_ref()
                        .is_some_and(|path| path == &entry.path)
                {
                    let removed = self.history.remove(index).expect("history index exists");
                    self.history_bytes =
                        self.history_bytes.saturating_sub(removed.retained_bytes());
                } else {
                    index += 1;
                }
            }
            self.next_id = self.next_id.wrapping_add(1);
            let entry =
                FileChangeReviewEntry::new(format!("{}:{}", self.revision, self.next_id), change);
            let bytes = entry.retained_bytes();
            if bytes > MAX_HISTORY_BYTES {
                self.history_truncated = true;
                continue;
            }
            while self.history.len() >= MAX_HISTORY_ENTRIES
                || self.history_bytes.saturating_add(bytes) > MAX_HISTORY_BYTES
            {
                if let Some(removed) = self.history.pop_front() {
                    self.history_bytes =
                        self.history_bytes.saturating_sub(removed.retained_bytes());
                    self.history_truncated = true;
                } else {
                    break;
                }
            }
            self.history_bytes += bytes;
            self.history.push_back(entry);
        }
    }
}

#[derive(Default)]
pub(crate) struct FileChangeReviewState {
    inner: Arc<Mutex<ReviewInner>>,
    database: Mutex<Option<PathBuf>>,
    app_data: Mutex<Option<PathBuf>>,
    // Serialize preference writes independently from the brief state lock.
    config_write: Mutex<()>,
    wake: Mutex<Option<SyncSender<()>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn validate_config_shape(config: &FileChangeReviewConfig) -> Result<(), String> {
    if !matches!(config.interval_minutes, 5 | 15 | 30) {
        return Err("검사 주기는 5분, 15분 또는 30분이어야 합니다".into());
    }
    if config
        .root
        .as_ref()
        .is_some_and(|root| root.is_empty() || root.len() > MAX_ROOT_BYTES)
    {
        return Err("대상 폴더 경로가 비어 있거나 경로 크기 한도를 초과했습니다".into());
    }
    if config.enabled && config.root.is_none() {
        return Err("변경을 확인할 로컬 폴더를 먼저 선택해 주세요".into());
    }
    Ok(())
}

fn validate_enabled_config(
    config: &mut FileChangeReviewConfig,
    app_data: &Path,
) -> Result<(), String> {
    validate_config_shape(config)?;
    if !config.enabled {
        return Ok(());
    }
    let root = validate_local_directory_path(Path::new(config.root.as_deref().unwrap()))
        .map_err(|error| error.to_string())?;
    // Reject the application's storage tree and its ancestors: writes from
    // this feature and the app's other databases cannot observe themselves.
    if app_data.starts_with(&root) || root.starts_with(app_data) {
        return Err("앱 데이터 폴더를 포함하는 범위는 검사할 수 없습니다. 더 작은 로컬 폴더를 선택해 주세요".into());
    }
    let root = root.to_str().ok_or("대상 폴더 경로를 표시할 수 없습니다")?;
    if root.len() > MAX_ROOT_BYTES {
        return Err("대상 폴더 경로 크기 한도를 초과했습니다".into());
    }
    config.root = Some(root.to_owned());
    Ok(())
}

fn resolved_storage_root(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|_| "변경 검토 설정 폴더를 확인하지 못했습니다".into());
    }
    let mut suffix = Vec::new();
    let mut cursor = path;
    while !cursor.exists() {
        suffix.push(
            cursor
                .file_name()
                .ok_or("변경 검토 설정 위치가 올바르지 않습니다")?,
        );
        cursor = cursor
            .parent()
            .ok_or("변경 검토 설정 위치가 올바르지 않습니다")?;
    }
    let mut result = cursor
        .canonicalize()
        .map_err(|_| "변경 검토 설정 폴더를 확인하지 못했습니다")?;
    for component in suffix.into_iter().rev() {
        result.push(component);
    }
    Ok(result)
}

pub(crate) fn setup(app: &AppHandle) -> Result<(), String> {
    let result = setup_inner(app);
    if let Err(error) = &result
        && let Some(state) = app.try_state::<FileChangeReviewState>()
        && let Ok(mut inner) = state.inner.lock()
    {
        inner.invalidate();
        inner.config.enabled = false;
        inner.phase = FileChangeReviewPhase::Error;
        inner.error = Some(error.clone());
    }
    result
}

fn setup_inner(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<FileChangeReviewState>();
    let mut worker = state
        .worker
        .lock()
        .map_err(|_| "변경 검토 작업 상태를 잠글 수 없습니다")?;
    if worker.is_some() {
        return Ok(());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "변경 검토 설정 위치를 찾지 못했습니다")?;
    let app_data = resolved_storage_root(&app_data)?;
    let database = app_data.join("file-change-review-v1.sqlite3");
    *state
        .database
        .lock()
        .map_err(|_| "변경 검토 저장 상태를 잠글 수 없습니다")? = Some(database.clone());
    *state
        .app_data
        .lock()
        .map_err(|_| "변경 검토 저장 상태를 잠글 수 없습니다")? = Some(app_data.clone());
    {
        let mut inner = state
            .inner
            .lock()
            .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
        match load_preferences(&database) {
            Ok(mut config) => match validate_enabled_config(&mut config, &app_data) {
                Ok(()) => inner.change_config(config),
                Err(error) => {
                    config.enabled = false;
                    inner.change_config(config);
                    inner.error = Some(error);
                }
            },
            Err(error) => inner.error = Some(error),
        }
    }
    let (wake, receiver) = mpsc::sync_channel(1);
    *state
        .wake
        .lock()
        .map_err(|_| "변경 검토 작업 상태를 잠글 수 없습니다")? = Some(wake);
    let inner = Arc::clone(&state.inner);
    let app = app.clone();
    *worker = Some(
        thread::Builder::new()
            .name("file-change-review".into())
            .spawn(move || scheduler(app, inner, receiver))
            .map_err(|_| "변경 검토 작업을 시작하지 못했습니다")?,
    );
    Ok(())
}

fn wake_worker(state: &FileChangeReviewState) -> Result<(), String> {
    let wake = state
        .wake
        .lock()
        .map_err(|_| "변경 검토 작업 상태를 잠글 수 없습니다")?;
    let sender = wake
        .as_ref()
        .ok_or("변경 검토 작업이 준비되지 않았습니다")?;
    match sender.try_send(()) {
        Ok(()) | Err(mpsc::TrySendError::Full(())) => Ok(()),
        Err(mpsc::TrySendError::Disconnected(())) => Err("변경 검토 작업이 종료되었습니다".into()),
    }
}

pub(crate) fn shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<FileChangeReviewState>() else {
        return;
    };
    if let Ok(mut inner) = state.inner.lock() {
        inner.closed = true;
        inner.invalidate();
        inner.phase = FileChangeReviewPhase::Off;
    }
    if let Ok(mut wake) = state.wake.lock() {
        // Dropping the only sender wakes a waiting scheduler even when OFF.
        wake.take();
    }
    if let Ok(mut worker) = state.worker.lock()
        && let Some(worker) = worker.take()
    {
        let _ = worker.join();
    }
}

fn scheduler(app: AppHandle, inner: Arc<Mutex<ReviewInner>>, receiver: Receiver<()>) {
    let shared = Arc::clone(&inner);
    scheduler_loop(inner, receiver, || run_check(&app, &shared));
}

fn scheduler_loop(inner: Arc<Mutex<ReviewInner>>, receiver: Receiver<()>, mut check: impl FnMut()) {
    loop {
        let wait = {
            let Ok(inner) = inner.lock() else { return };
            if inner.closed {
                return;
            }
            if inner.config.enabled {
                inner
                    .next_deadline
                    .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            } else {
                None
            }
        };
        match wait {
            Some(delay) if delay.is_zero() => check(),
            Some(delay) => match receiver.recv_timeout(delay) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            },
            None => {
                if receiver.recv().is_err() {
                    return;
                }
            }
        }
    }
}

fn can_commit(inner: &ReviewInner, revision: u64, cancellation: &Arc<AtomicBool>) -> bool {
    !inner.closed
        && inner.config.enabled
        && inner.revision == revision
        && !cancellation.load(Ordering::Acquire)
        && inner
            .active
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, cancellation))
}

fn run_check(app: &AppHandle, shared: &Arc<Mutex<ReviewInner>>) {
    let runtime = app.state::<crate::ScanRuntime>();
    let (root, revision, cancellation, previous, _completion) = {
        let Ok(mut inner) = shared.lock() else { return };
        if inner.closed || !inner.config.enabled {
            return;
        }
        let cancellation = match runtime.begin() {
            Ok(cancellation) => cancellation,
            Err(error) => {
                inner.phase = if runtime.is_running() {
                    FileChangeReviewPhase::Busy
                } else {
                    FileChangeReviewPhase::Error
                };
                inner.error = (!runtime.is_running()).then_some(error);
                let delay = Duration::from_secs(u64::from(inner.config.interval_minutes) * 60);
                inner.schedule(delay);
                return;
            }
        };
        let completion = crate::ScanCompletionGuard::new(app.clone());
        inner.active = Some(Arc::clone(&cancellation));
        inner.in_progress = true;
        inner.phase = if inner.baseline.is_none() {
            FileChangeReviewPhase::Baseline
        } else {
            FileChangeReviewPhase::Checking
        };
        inner.error = None;
        inner.next_check_at_unix_ms = None;
        inner.next_deadline = None;
        (
            inner.config.root.clone().unwrap_or_default(),
            inner.revision,
            cancellation,
            inner.baseline.clone(),
            completion,
        )
    };
    let result = capture_file_change_snapshot(&root, || cancellation.load(Ordering::Acquire))
        .and_then(|snapshot| {
            let report = previous
                .as_ref()
                .map(|previous| compare_file_change_snapshots(previous, &snapshot))
                .transpose()?;
            Ok((snapshot, report))
        });
    let Ok(mut inner) = shared.lock() else { return };
    if !can_commit(&inner, revision, &cancellation) {
        // A new generation owns its scheduling; a late old result cannot
        // clear a newer worker's marker, state, baseline or history.
        if inner.revision == revision {
            inner.active = None;
            inner.in_progress = false;
            inner.phase = FileChangeReviewPhase::Incomplete;
            inner.error = Some("검사가 취소되어 이전의 완전한 결과를 유지합니다".into());
            let delay = Duration::from_secs(u64::from(inner.config.interval_minutes) * 60);
            inner.schedule(delay);
        }
        return;
    }
    inner.active = None;
    inner.in_progress = false;
    match result {
        Ok((snapshot, report)) => {
            if let Some(report) = report {
                inner.retain_changes(report);
            }
            inner.last_checked_at_unix_ms = Some(snapshot.checked_at_unix_ms());
            inner.baseline = Some(Arc::new(snapshot));
            inner.phase = FileChangeReviewPhase::Idle;
            inner.error = None;
        }
        Err(error) => {
            inner.phase = match &error {
                ScanError::MissingPath(_) | ScanError::NotDirectory(_) => {
                    FileChangeReviewPhase::Error
                }
                _ => FileChangeReviewPhase::Incomplete,
            };
            inner.error = Some(error.to_string());
        }
    }
    let delay = Duration::from_secs(u64::from(inner.config.interval_minutes) * 60);
    // Schedule from completion and never enqueue missed ticks or a busy retry.
    inner.schedule(delay);
}

fn selected_by_filter(entry: &FileChangeReviewEntry, filter: &str) -> bool {
    match filter {
        "review_needed" => entry.review_state == FileReviewState::ReviewNeeded,
        "skipped" => entry.review_state == FileReviewState::Skipped,
        _ => true,
    }
}

fn read_page(
    inner: &ReviewInner,
    offset: Option<usize>,
    filter: Option<&str>,
) -> Result<FileChangeReviewPage, String> {
    let filter = filter.unwrap_or("all");
    if !matches!(filter, "all" | "review_needed" | "skipped") {
        return Err("지원하지 않는 변경 검토 필터입니다".into());
    }
    let total = inner
        .history
        .iter()
        .filter(|entry| selected_by_filter(entry, filter))
        .count();
    let offset = offset.unwrap_or(0).min(total);
    let entries = inner
        .history
        .iter()
        .rev()
        .filter(|entry| selected_by_filter(entry, filter))
        .skip(offset)
        .take(PAGE_SIZE)
        .cloned()
        .collect();
    Ok(FileChangeReviewPage {
        status: inner.status(),
        entries,
        offset,
        total,
    })
}

pub(crate) fn page(
    app: &AppHandle,
    offset: Option<usize>,
    filter: Option<&str>,
) -> Result<FileChangeReviewPage, String> {
    let state = app.state::<FileChangeReviewState>();
    let inner = state
        .inner
        .lock()
        .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
    read_page(&inner, offset, filter)
}

pub(crate) fn status(app: &AppHandle) -> Result<FileChangeReviewStatus, String> {
    let state = app.state::<FileChangeReviewState>();
    let inner = state
        .inner
        .lock()
        .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
    Ok(inner.status())
}

fn require_main_window(label: &str) -> Result<(), String> {
    if label == "main" {
        Ok(())
    } else {
        Err("변경 검토 설정과 처리는 메인 앱 창에서만 사용할 수 있습니다".into())
    }
}

#[tauri::command]
pub(crate) fn get_file_change_review(
    app: AppHandle,
    offset: Option<usize>,
    filter: Option<String>,
) -> Result<FileChangeReviewPage, String> {
    page(&app, offset, filter.as_deref())
}

#[tauri::command]
pub(crate) async fn set_file_change_review_config(
    window: WebviewWindow,
    app: AppHandle,
    config: FileChangeReviewConfig,
) -> Result<FileChangeReviewPage, String> {
    require_main_window(window.label())?;
    tauri::async_runtime::spawn_blocking(move || update_config(&app, config))
        .await
        .map_err(|_| "변경 검토 설정 작업이 종료되었습니다")?
}

fn update_config(
    app: &AppHandle,
    mut config: FileChangeReviewConfig,
) -> Result<FileChangeReviewPage, String> {
    validate_config_shape(&config)?;
    let state = app.state::<FileChangeReviewState>();
    let _write = state
        .config_write
        .lock()
        .map_err(|_| "변경 검토 설정 상태를 잠글 수 없습니다")?;
    let database = state
        .database
        .lock()
        .map_err(|_| "변경 검토 저장 상태를 잠글 수 없습니다")?
        .clone()
        .ok_or("변경 검토 설정 저장소가 준비되지 않았습니다")?;
    let app_data = state
        .app_data
        .lock()
        .map_err(|_| "변경 검토 저장 상태를 잠글 수 없습니다")?
        .clone()
        .ok_or("변경 검토 설정 위치가 준비되지 않았습니다")?;
    if !config.enabled {
        {
            let mut inner = state
                .inner
                .lock()
                .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
            if inner.closed {
                return Err("앱을 종료하고 있습니다".into());
            }
            inner.change_config(config.clone());
        }
        let result = save_preferences(&database, &config);
        if let Err(error) = result {
            let mut inner = state
                .inner
                .lock()
                .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
            inner.error = Some(format!(
                "현재 검사는 꺼졌지만 설정을 저장하지 못했습니다. 다음 앱 실행에서 이전 설정이 복원될 수 있습니다: {error}"
            ));
        }
        let _ = wake_worker(&state);
        return page(app, None, None);
    }
    validate_enabled_config(&mut config, &app_data)?;
    {
        let inner = state
            .inner
            .lock()
            .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
        if inner.closed {
            return Err("앱을 종료하고 있습니다".into());
        }
        if inner.config == config {
            return read_page(&inner, None, None);
        }
    }
    // A failed enable/save leaves the current configuration untouched.
    save_preferences(&database, &config)?;
    {
        let mut inner = state
            .inner
            .lock()
            .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
        if inner.closed {
            return Err("앱을 종료하고 있습니다".into());
        }
        inner.change_config(config);
    }
    wake_worker(&state)?;
    page(app, None, None)
}

#[tauri::command]
pub(crate) fn request_file_change_review_check(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<FileChangeReviewPage, String> {
    require_main_window(window.label())?;
    let state = app.state::<FileChangeReviewState>();
    {
        let mut inner = state
            .inner
            .lock()
            .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
        if inner.closed {
            return Err("앱을 종료하고 있습니다".into());
        }
        if !inner.config.enabled {
            return Err("변경 검토 옵션을 먼저 켜 주세요".into());
        }
        if !inner.in_progress {
            inner.error = None;
            inner.schedule(Duration::ZERO);
        }
    }
    wake_worker(&state)?;
    page(&app, None, None)
}

#[tauri::command]
pub(crate) fn set_file_change_review_disposition(
    window: WebviewWindow,
    app: AppHandle,
    entry_id: String,
    review_state: FileChangeReviewDisposition,
) -> Result<FileChangeReviewPage, String> {
    require_main_window(window.label())?;
    let state = app.state::<FileChangeReviewState>();
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| "변경 검토 상태를 잠글 수 없습니다")?;
    if inner.closed {
        return Err("앱을 종료하고 있습니다".into());
    }
    let entry = inner.history.iter_mut().find(|entry| entry.id == entry_id)
        .ok_or("변경 항목이 갱신되었거나 보관 한도를 넘어 목록에서 제외되었습니다. 목록을 다시 확인해 주세요")?;
    entry.review_state = review_state.into();
    entry.decision_source = FileChangeDecisionSource::Manual;
    read_page(&inner, None, None)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedPreferences {
    version: u32,
    config: FileChangeReviewConfig,
}

fn storage_file_bytes(path: &Path) -> Result<u64, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err("변경 검토 설정 파일의 접근 제한을 확인해 주세요".into());
                }
            }
            Ok(metadata.len())
        }
        Ok(_) => Err("변경 검토 설정 파일이 일반 파일이 아닙니다".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(_) => Err("변경 검토 설정 파일을 확인하지 못했습니다".into()),
    }
}

fn check_storage(path: &Path) -> Result<(), String> {
    let database_bytes = storage_file_bytes(path)?;
    if database_bytes > MAX_DATABASE_BYTES {
        return Err("변경 검토 설정 저장 크기 한도를 초과했습니다".into());
    }
    let mut total = database_bytes;
    for suffix in ["-journal", "-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        total = total.saturating_add(storage_file_bytes(Path::new(&sidecar))?);
    }
    if total > MAX_STORAGE_BYTES {
        return Err("변경 검토 설정 임시 저장 크기 한도를 초과했습니다".into());
    }
    Ok(())
}

fn configure_connection(connection: &Connection) -> Result<(), String> {
    connection
        .busy_timeout(Duration::from_millis(250))
        .map_err(|_| "변경 검토 설정 저장소를 준비하지 못했습니다")?;
    connection
        .execute_batch(
            "PRAGMA mmap_size=0;
         PRAGMA cache_size=-128;
         PRAGMA temp_store=MEMORY;
         PRAGMA trusted_schema=OFF;
         PRAGMA threads=0;",
        )
        .map_err(|_| "변경 검토 설정 저장소를 준비하지 못했습니다")?;
    let deadline = Instant::now() + Duration::from_millis(500);
    connection
        .progress_handler(100, Some(move || Instant::now() >= deadline))
        .map_err(|_| "변경 검토 설정 읽기 한도를 준비하지 못했습니다")?;
    Ok(())
}

fn load_preferences(path: &Path) -> Result<FileChangeReviewConfig, String> {
    check_storage(path)?;
    if !path.exists() {
        return Ok(FileChangeReviewConfig::default());
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "변경 검토 설정을 읽지 못했습니다")?;
    configure_connection(&connection)?;
    let payload: Option<Option<String>> = connection.query_row(
        "SELECT CASE WHEN length(CAST(payload AS BLOB)) <= ?1 THEN payload ELSE NULL END FROM file_change_review_preferences WHERE id=1",
        [MAX_PREFERENCES_BYTES as i64], |row| row.get(0),
    ).optional().map_err(|_| "변경 검토 설정을 읽지 못했습니다")?;
    let Some(payload) = payload else {
        return Ok(FileChangeReviewConfig::default());
    };
    let payload = payload.ok_or("변경 검토 설정 크기 한도를 초과했습니다")?;
    let stored: PersistedPreferences =
        serde_json::from_str(&payload).map_err(|_| "변경 검토 설정 형식이 올바르지 않습니다")?;
    if stored.version != 1 {
        return Err("지원하지 않는 변경 검토 설정 버전입니다".into());
    }
    validate_config_shape(&stored.config)?;
    Ok(stored.config)
}

fn save_preferences(path: &Path, config: &FileChangeReviewConfig) -> Result<(), String> {
    validate_config_shape(config)?;
    let payload = serde_json::to_string(&PersistedPreferences {
        version: 1,
        config: config.clone(),
    })
    .map_err(|_| "변경 검토 설정을 준비하지 못했습니다")?;
    if payload.len() > MAX_PREFERENCES_BYTES {
        return Err("변경 검토 설정 크기 한도를 초과했습니다".into());
    }
    let parent = path
        .parent()
        .ok_or("변경 검토 설정 위치가 올바르지 않습니다")?;
    fs::create_dir_all(parent).map_err(|_| "변경 검토 설정 폴더를 만들지 못했습니다")?;
    if fs::symlink_metadata(parent)
        .map_err(|_| "변경 검토 설정 폴더를 확인하지 못했습니다")?
        .file_type()
        .is_symlink()
    {
        return Err("변경 검토 설정 폴더는 링크일 수 없습니다".into());
    }
    check_storage(path)?;
    if !path.exists() {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(path)
            .map_err(|_| "변경 검토 설정 파일을 만들지 못했습니다")?;
    }
    let connection =
        Connection::open(path).map_err(|_| "변경 검토 설정 저장소를 열지 못했습니다")?;
    configure_connection(&connection)?;
    let page_size: i64 = connection
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .map_err(|_| "변경 검토 설정 저장 한도를 확인하지 못했습니다")?;
    let page_size = u64::try_from(page_size)
        .ok()
        .filter(|size| *size > 0)
        .ok_or("변경 검토 설정 페이지 크기가 올바르지 않습니다")?;
    let pages = (MAX_DATABASE_BYTES / page_size).max(1);
    let accepted: i64 = connection
        .query_row(&format!("PRAGMA max_page_count={pages}"), [], |row| {
            row.get(0)
        })
        .map_err(|_| "변경 검토 설정 저장 한도를 준비하지 못했습니다")?;
    let accepted = u64::try_from(accepted)
        .ok()
        .filter(|count| *count > 0)
        .ok_or("변경 검토 설정 페이지 한도가 올바르지 않습니다")?;
    if accepted > pages {
        return Err("변경 검토 설정 저장 크기 한도를 초과했습니다".into());
    }
    connection
        .execute_batch(
            "PRAGMA journal_mode=DELETE;
         PRAGMA synchronous=FULL;
         PRAGMA journal_size_limit=0;
         CREATE TABLE IF NOT EXISTS file_change_review_preferences (
             id INTEGER PRIMARY KEY CHECK(id=1),
             payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 32768)
         );",
        )
        .map_err(|_| "변경 검토 설정 저장소를 준비하지 못했습니다")?;
    connection.execute(
        "INSERT INTO file_change_review_preferences(id,payload) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",
        [payload],
    ).map_err(|_| "변경 검토 설정을 저장하지 못했습니다")?;
    check_storage(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(path: String) -> FileChangeEntry {
        FileChangeEntry {
            name: path.clone(),
            path,
            previous_path: None,
            kind: FileChangeKind::Modified,
            review_state: FileReviewState::ReviewNeeded,
            reasons: vec!["executable_extension".into()],
            is_directory: false,
            logical_bytes: 3,
            modified_at_unix_ms: Some(10),
        }
    }

    fn report(changes: Vec<FileChangeEntry>) -> FileChangeReport {
        FileChangeReport {
            total_changes: changes.len(),
            changes,
            truncated: false,
            baseline_entries: 0,
            checked_at_unix_ms: 10,
        }
    }

    #[test]
    fn default_is_off_and_only_preferences_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("preferences.sqlite3");
        assert!(!load_preferences(&database).unwrap().enabled);
        assert!(!database.exists());
        let config = FileChangeReviewConfig {
            enabled: true,
            root: Some("/selected/local".into()),
            interval_minutes: 5,
        };
        save_preferences(&database, &config).unwrap();
        assert_eq!(load_preferences(&database).unwrap(), config);
        let stored: String = Connection::open(&database)
            .unwrap()
            .query_row(
                "SELECT payload FROM file_change_review_preferences WHERE id=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&stored).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 2);
        assert!(value.get("history").is_none() && value.get("baseline").is_none());
        let mut restarted = ReviewInner::default();
        restarted.change_config(load_preferences(&database).unwrap());
        assert_eq!(restarted.phase, FileChangeReviewPhase::Baseline);
        assert!(restarted.baseline.is_none() && restarted.history.is_empty());
        assert_eq!(restarted.last_checked_at_unix_ms, None);
    }

    #[test]
    fn config_bounds_and_app_storage_overlap_are_rejected() {
        for interval_minutes in [0, 1, 10, 60, u32::MAX] {
            assert!(
                validate_config_shape(&FileChangeReviewConfig {
                    interval_minutes,
                    ..Default::default()
                })
                .is_err()
            );
        }
        assert!(
            validate_config_shape(&FileChangeReviewConfig {
                enabled: true,
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            validate_config_shape(&FileChangeReviewConfig {
                root: Some("x".repeat(MAX_ROOT_BYTES + 1)),
                ..Default::default()
            })
            .is_err()
        );
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let app_data = root.join("app-data");
        fs::create_dir(&app_data).unwrap();
        let mut config = FileChangeReviewConfig {
            enabled: true,
            root: Some(root.display().to_string()),
            interval_minutes: 15,
        };
        assert!(
            validate_enabled_config(&mut config, &app_data)
                .unwrap_err()
                .contains("앱 데이터")
        );
        config.enabled = false;
        config.root = Some("/missing/root".into());
        assert!(validate_enabled_config(&mut config, &app_data).is_ok());
    }

    #[test]
    fn bounded_history_replaces_manual_decisions_and_limits_pages() {
        let mut inner = ReviewInner::default();
        inner.retain_changes(report(
            (0..300)
                .map(|index| change(format!("file-{index}")))
                .collect(),
        ));
        assert_eq!(inner.history.len(), MAX_HISTORY_ENTRIES);
        assert!(inner.history_bytes <= MAX_HISTORY_BYTES && inner.history_truncated);
        assert_eq!(
            read_page(&inner, None, None).unwrap().entries.len(),
            PAGE_SIZE
        );
        let old_id = inner.history.back().unwrap().id.clone();
        inner.history.back_mut().unwrap().review_state = FileReviewState::Skipped;
        inner.history.back_mut().unwrap().decision_source = FileChangeDecisionSource::Manual;
        inner.retain_changes(report(vec![change("file-299".into())]));
        let latest = inner.history.back().unwrap();
        assert_ne!(latest.id, old_id);
        assert_eq!(latest.review_state, FileReviewState::ReviewNeeded);
        assert_eq!(latest.decision_source, FileChangeDecisionSource::Rule);
        assert_eq!(
            inner
                .history
                .iter()
                .filter(|entry| entry.path == "file-299")
                .count(),
            1
        );
        assert!(read_page(&inner, None, Some("unknown")).is_err());
        assert!(
            read_page(&inner, Some(usize::MAX), None)
                .unwrap()
                .entries
                .is_empty()
        );
        inner.retain_changes(report(vec![change("x".repeat(MAX_HISTORY_BYTES))]));
        assert!(inner.history_bytes <= MAX_HISTORY_BYTES);
    }

    #[test]
    fn disable_root_change_and_closed_generation_reject_late_results() {
        let mut inner = ReviewInner::default();
        inner.change_config(FileChangeReviewConfig {
            enabled: true,
            root: Some("/one".into()),
            interval_minutes: 15,
        });
        let cancellation = Arc::new(AtomicBool::new(false));
        inner.active = Some(Arc::clone(&cancellation));
        let revision = inner.revision;
        assert!(can_commit(&inner, revision, &cancellation));
        inner.change_config(FileChangeReviewConfig {
            enabled: false,
            root: Some("/one".into()),
            interval_minutes: 15,
        });
        assert!(cancellation.load(Ordering::Acquire));
        assert!(!can_commit(&inner, revision, &cancellation));
        assert!(inner.baseline.is_none() && inner.next_deadline.is_none());
        inner.retain_changes(report(vec![change("old-file".into())]));
        inner.change_config(FileChangeReviewConfig {
            enabled: true,
            root: Some("/two".into()),
            interval_minutes: 30,
        });
        assert!(inner.history.is_empty() && !inner.history_truncated);
        let cancellation = Arc::new(AtomicBool::new(false));
        inner.active = Some(Arc::clone(&cancellation));
        inner.closed = true;
        assert!(!can_commit(&inner, inner.revision, &cancellation));
    }

    #[test]
    fn repeated_enable_disable_reuses_one_worker_and_cancels_every_active_generation() {
        use std::sync::atomic::AtomicUsize;
        let shared = Arc::new(Mutex::new(ReviewInner::default()));
        let (wake, receiver) = mpsc::sync_channel(1);
        let (entered, starts) = mpsc::channel();
        let (exited, finishes) = mpsc::channel();
        let active_count = Arc::new(AtomicUsize::new(0));
        let peak_count = Arc::new(AtomicUsize::new(0));
        let worker_shared = Arc::clone(&shared);
        let worker_active = Arc::clone(&active_count);
        let worker_peak = Arc::clone(&peak_count);
        let worker = thread::spawn(move || {
            let check_shared = Arc::clone(&worker_shared);
            scheduler_loop(worker_shared, receiver, || {
                let (revision, cancellation) = {
                    let mut inner = check_shared.lock().unwrap();
                    let cancellation = Arc::new(AtomicBool::new(false));
                    inner.active = Some(Arc::clone(&cancellation));
                    inner.in_progress = true;
                    inner.next_deadline = None;
                    (inner.revision, cancellation)
                };
                let active = worker_active.fetch_add(1, Ordering::AcqRel) + 1;
                worker_peak.fetch_max(active, Ordering::AcqRel);
                entered.send(thread::current().id()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(2);
                while !cancellation.load(Ordering::Acquire) && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(1));
                }
                let rejected = !can_commit(&check_shared.lock().unwrap(), revision, &cancellation);
                worker_active.fetch_sub(1, Ordering::AcqRel);
                exited
                    .send((cancellation.load(Ordering::Acquire), rejected))
                    .unwrap();
            });
        });
        let mut first_thread = None;
        for _ in 0..12 {
            shared
                .lock()
                .unwrap()
                .change_config(FileChangeReviewConfig {
                    enabled: true,
                    root: Some("/fixture".into()),
                    interval_minutes: 5,
                });
            let _ = wake.try_send(());
            let thread_id = starts.recv_timeout(Duration::from_secs(2)).unwrap();
            if let Some(first) = first_thread {
                assert_eq!(thread_id, first);
            } else {
                first_thread = Some(thread_id);
            }
            shared
                .lock()
                .unwrap()
                .change_config(FileChangeReviewConfig {
                    enabled: false,
                    root: Some("/fixture".into()),
                    interval_minutes: 5,
                });
            let _ = wake.try_send(());
            assert_eq!(
                finishes.recv_timeout(Duration::from_secs(2)).unwrap(),
                (true, true)
            );
            assert_eq!(active_count.load(Ordering::Acquire), 0);
        }
        assert_eq!(peak_count.load(Ordering::Acquire), 1);
        assert!(starts.try_recv().is_err());
        shared.lock().unwrap().closed = true;
        drop(wake);
        worker.join().unwrap();
    }

    #[test]
    fn malformed_and_oversized_preferences_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("preferences.sqlite3");
        save_preferences(&database, &FileChangeReviewConfig::default()).unwrap();
        let connection = Connection::open(&database).unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        connection
            .execute(
                "UPDATE file_change_review_preferences SET payload=?1",
                ["x".repeat(MAX_PREFERENCES_BYTES + 1)],
            )
            .unwrap();
        drop(connection);
        assert!(load_preferences(&database).is_err());
        fs::OpenOptions::new()
            .write(true)
            .open(&database)
            .unwrap()
            .set_len(MAX_DATABASE_BYTES + 1)
            .unwrap();
        assert!(load_preferences(&database).is_err());
    }

    #[test]
    fn mutations_are_main_window_only() {
        assert!(require_main_window("main").is_ok());
        assert!(require_main_window("assistant").is_err());
        assert!(require_main_window("mcp").is_err());
    }
}
