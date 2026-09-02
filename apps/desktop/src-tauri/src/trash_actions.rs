use super::{ScanCompletionGuard, ScanRuntime, StoredReports};
use bloomsweepy_core::{
    CleanupConfidence, VerifiedTrashItem, revalidate_verified_trash_item,
    validate_cleanup_trash_candidate, validate_duplicate_trash_selection,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const MAX_TRASH_ITEMS: usize = 500;
pub(crate) const MAX_ACTION_JOURNAL_BYTES: u64 = 8 * 1024 * 1024;
static OPERATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuplicateTrashRequest {
    pub(crate) groups: Vec<DuplicateTrashGroupSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuplicateTrashGroupSelection {
    pub(crate) content_hash: String,
    pub(crate) paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupTrashRequest {
    pub(crate) paths: Vec<String>,
    pub(crate) allow_review_candidates: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TrashProgressPhase {
    Preflight,
    Moving,
    Finalizing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrashProgress {
    phase: TrashProgressPhase,
    message: String,
    processed_items: usize,
    total_items: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TrashItemStatus {
    Moved,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrashItemResult {
    pub(crate) path: String,
    pub(crate) logical_bytes: u64,
    pub(crate) status: TrashItemStatus,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrashOperationResult {
    pub(crate) operation_id: String,
    pub(crate) requested_count: usize,
    pub(crate) moved_count: usize,
    pub(crate) moved_bytes: u64,
    pub(crate) cancelled: bool,
    pub(crate) stopped_early: bool,
    pub(crate) journal_complete: bool,
    pub(crate) journal_path: String,
    pub(crate) items: Vec<TrashItemResult>,
}

trait TrashBackend {
    fn move_to_trash(&self, path: &Path) -> Result<(), String>;
}

struct SystemTrash;

impl TrashBackend for SystemTrash {
    fn move_to_trash(&self, path: &Path) -> Result<(), String> {
        trash::delete(path).map_err(|error| error.to_string())
    }
}

struct Journal {
    path: PathBuf,
    writer: BufWriter<File>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum TrashActionKind {
    DuplicateFiles,
    CleanupCandidates,
}

impl Journal {
    fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("작업 기록 폴더를 만들지 못했습니다: {error}"))?;
        }
        rotate_journal_if_needed(&path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("작업 기록을 열지 못했습니다: {error}"))?;
        Ok(Self {
            path,
            writer: BufWriter::new(file),
        })
    }

    fn record(&mut self, value: serde_json::Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.writer, &value)
            .map_err(|error| format!("작업 기록을 직렬화하지 못했습니다: {error}"))?;
        self.writer
            .write_all(b"\n")
            .map_err(|error| format!("작업 기록을 쓰지 못했습니다: {error}"))?;
        self.writer
            .flush()
            .map_err(|error| format!("작업 기록을 비우지 못했습니다: {error}"))?;
        self.writer
            .get_ref()
            .sync_data()
            .map_err(|error| format!("작업 기록을 디스크에 동기화하지 못했습니다: {error}"))
    }
}

fn rotate_journal_if_needed(path: &Path) -> Result<(), String> {
    let size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("작업 기록 크기를 확인하지 못했습니다: {error}")),
    };
    if size < MAX_ACTION_JOURNAL_BYTES {
        return Ok(());
    }

    let previous = path.with_extension("previous.jsonl");
    if previous.exists() {
        fs::remove_file(&previous)
            .map_err(|error| format!("이전 작업 기록을 교체하지 못했습니다: {error}"))?;
    }
    fs::rename(path, &previous).map_err(|error| format!("작업 기록을 회전하지 못했습니다: {error}"))
}

#[tauri::command]
pub(crate) async fn trash_duplicate_files(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
    request: DuplicateTrashRequest,
) -> Result<TrashOperationResult, String> {
    trash_duplicate_files_internal(app, runtime.inner(), reports.inner(), request).await
}

pub(crate) async fn trash_duplicate_files_internal(
    app: AppHandle,
    runtime: &ScanRuntime,
    reports: &StoredReports,
    request: DuplicateTrashRequest,
) -> Result<TrashOperationResult, String> {
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let report = reports.scan_report()?;
    let journal_path = action_journal_path(&app)?;
    let app_for_worker = app.clone();
    let cancellation_for_worker = Arc::clone(&cancellation);

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let requested_count = request
            .groups
            .iter()
            .try_fold(0_usize, |total, group| total.checked_add(group.paths.len()))
            .ok_or_else(|| "선택 항목 수가 올바르지 않습니다".to_owned())?;
        validate_requested_count(requested_count)?;

        let mut seen_hashes = HashSet::new();
        let mut seen_paths = HashSet::new();
        let mut verified = Vec::with_capacity(requested_count);
        for (index, selection) in request.groups.iter().enumerate() {
            emit_progress(
                &app_for_worker,
                TrashProgressPhase::Preflight,
                format!(
                    "중복 그룹의 파일 신원과 전체 내용을 다시 확인하고 있습니다 ({}/{})",
                    index + 1,
                    request.groups.len()
                ),
                verified.len(),
                requested_count,
            );
            if !seen_hashes.insert(selection.content_hash.as_str()) {
                return Err("같은 중복 그룹이 두 번 요청되었습니다".to_owned());
            }
            for path in &selection.paths {
                if !seen_paths.insert(path.as_str()) {
                    return Err("같은 파일이 두 번 선택되었습니다".to_owned());
                }
            }
            let group = report
                .duplicate_groups
                .iter()
                .find(|group| group.content_hash == selection.content_hash)
                .ok_or_else(|| "현재 서버의 스캔 결과에 없는 중복 그룹입니다".to_owned())?;
            verified.extend(
                validate_duplicate_trash_selection(&report.root, group, &selection.paths, || {
                    cancellation_for_worker.load(Ordering::Acquire)
                })
                .map_err(|error| error.to_string())?,
            );
        }

        execute_verified_items(
            verified,
            journal_path,
            TrashActionKind::DuplicateFiles,
            &cancellation_for_worker,
            |progress| {
                let _ = app_for_worker.emit("trash-progress", progress);
            },
            &SystemTrash,
        )
    })
    .await
    .map_err(|error| format!("휴지통 이동 작업을 실행하지 못했습니다: {error}"));

    let clear_result = reports.clear_all();
    clear_result?;
    worker_result?
}

#[tauri::command]
pub(crate) async fn trash_cleanup_candidates(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
    request: CleanupTrashRequest,
) -> Result<TrashOperationResult, String> {
    trash_cleanup_candidates_internal(app, runtime.inner(), reports.inner(), request).await
}

pub(crate) async fn trash_cleanup_candidates_internal(
    app: AppHandle,
    runtime: &ScanRuntime,
    reports: &StoredReports,
    request: CleanupTrashRequest,
) -> Result<TrashOperationResult, String> {
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let report = reports.cleanup_report()?;
    let journal_path = action_journal_path(&app)?;
    let app_for_worker = app.clone();
    let cancellation_for_worker = Arc::clone(&cancellation);

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        validate_requested_count(request.paths.len())?;
        let mut seen_paths = HashSet::new();
        let mut verified = Vec::with_capacity(request.paths.len());
        for (index, path) in request.paths.iter().enumerate() {
            emit_progress(
                &app_for_worker,
                TrashProgressPhase::Preflight,
                format!(
                    "정리 후보의 구조와 변경 흔적을 다시 확인하고 있습니다 ({}/{})",
                    index + 1,
                    request.paths.len()
                ),
                index,
                request.paths.len(),
            );
            if !seen_paths.insert(path.as_str()) {
                return Err("같은 정리 후보가 두 번 선택되었습니다".to_owned());
            }
            let candidate = report
                .candidates
                .iter()
                .find(|candidate| candidate.path == *path)
                .ok_or_else(|| "현재 서버의 스캔 결과에 없는 정리 후보입니다".to_owned())?;
            if candidate.confidence == CleanupConfidence::Review && !request.allow_review_candidates
            {
                return Err("AppData 검토 후보에 대한 별도 확인이 필요합니다".to_owned());
            }
            verified.push(
                validate_cleanup_trash_candidate(candidate, || {
                    cancellation_for_worker.load(Ordering::Acquire)
                })
                .map_err(|error| error.to_string())?,
            );
        }

        execute_verified_items(
            verified,
            journal_path,
            TrashActionKind::CleanupCandidates,
            &cancellation_for_worker,
            |progress| {
                let _ = app_for_worker.emit("trash-progress", progress);
            },
            &SystemTrash,
        )
    })
    .await
    .map_err(|error| format!("휴지통 이동 작업을 실행하지 못했습니다: {error}"));

    let clear_result = reports.clear_all();
    clear_result?;
    worker_result?
}

fn execute_verified_items<B, F>(
    items: Vec<VerifiedTrashItem>,
    journal_path: PathBuf,
    action_kind: TrashActionKind,
    cancellation: &AtomicBool,
    mut on_progress: F,
    backend: &B,
) -> Result<TrashOperationResult, String>
where
    B: TrashBackend,
    F: FnMut(TrashProgress),
{
    let operation_id = operation_id();
    let requested_count = items.len();
    let planned_items: Vec<_> = items
        .iter()
        .map(|item| {
            json!({
                "path": recovery_path_string(item),
                "logicalBytes": item.logical_bytes(),
            })
        })
        .collect();
    let mut journal = Journal::open(journal_path)?;
    journal.record(json!({
        "schemaVersion": 1,
        "timestampUnixMs": unix_time_ms(),
        "operationId": operation_id,
        "event": "planned",
        "actionKind": action_kind,
        "recovery": "osTrash",
        "items": planned_items,
    }))?;

    let mut results = Vec::with_capacity(requested_count);
    let mut moved_count = 0_usize;
    let mut moved_bytes = 0_u64;
    let mut cancelled = false;
    let mut stopped_early = false;
    let mut journal_complete = true;

    for (index, item) in items.iter().enumerate() {
        if cancellation.load(Ordering::Acquire) {
            cancelled = true;
            stopped_early = true;
            push_skipped(
                &mut results,
                &items[index..],
                "사용자가 작업 중단을 요청했습니다",
            );
            break;
        }

        on_progress(TrashProgress {
            phase: TrashProgressPhase::Moving,
            message: format!(
                "휴지통 이동 직전 항목을 다시 확인하고 있습니다 ({}/{requested_count})",
                index + 1
            ),
            processed_items: index,
            total_items: requested_count,
        });
        if let Err(error) =
            revalidate_verified_trash_item(item, || cancellation.load(Ordering::Acquire))
        {
            if matches!(error, bloomsweepy_core::ActionValidationError::Cancelled) {
                cancelled = true;
                push_skipped(
                    &mut results,
                    &items[index..],
                    "사용자가 작업 중단을 요청했습니다",
                );
            } else {
                results.push(item_result(
                    item,
                    TrashItemStatus::Failed,
                    Some(error.to_string()),
                ));
                push_skipped(
                    &mut results,
                    &items[index + 1..],
                    "앞선 항목의 안전 검사 실패로 작업을 중단했습니다",
                );
            }
            stopped_early = true;
            break;
        }

        if let Err(error) = journal.record(json!({
            "timestampUnixMs": unix_time_ms(),
            "operationId": operation_id,
            "event": "moving",
            "path": recovery_path_string(item),
            "logicalBytes": item.logical_bytes(),
        })) {
            results.push(item_result(item, TrashItemStatus::Failed, Some(error)));
            push_skipped(
                &mut results,
                &items[index + 1..],
                "작업 기록을 보존할 수 없어 중단했습니다",
            );
            stopped_early = true;
            journal_complete = false;
            break;
        }

        match backend.move_to_trash(item.path()) {
            Ok(()) => {
                moved_count = moved_count.saturating_add(1);
                moved_bytes = moved_bytes.saturating_add(item.logical_bytes());
                results.push(item_result(item, TrashItemStatus::Moved, None));
                if journal
                    .record(json!({
                        "timestampUnixMs": unix_time_ms(),
                        "operationId": operation_id,
                        "event": "moved",
                        "path": recovery_path_string(item),
                        "logicalBytes": item.logical_bytes(),
                    }))
                    .is_err()
                {
                    journal_complete = false;
                    stopped_early = true;
                    push_skipped(
                        &mut results,
                        &items[index + 1..],
                        "휴지통 이동 후 작업 기록 동기화에 실패해 중단했습니다",
                    );
                    break;
                }
            }
            Err(error) => {
                let message = format!("운영체제 휴지통으로 이동하지 못했습니다: {error}");
                results.push(item_result(
                    item,
                    TrashItemStatus::Failed,
                    Some(message.clone()),
                ));
                if journal
                    .record(json!({
                        "timestampUnixMs": unix_time_ms(),
                        "operationId": operation_id,
                        "event": "failed",
                        "path": recovery_path_string(item),
                        "message": message,
                    }))
                    .is_err()
                {
                    journal_complete = false;
                }
                push_skipped(
                    &mut results,
                    &items[index + 1..],
                    "앞선 항목의 휴지통 이동 실패로 작업을 중단했습니다",
                );
                stopped_early = true;
                break;
            }
        }
    }

    on_progress(TrashProgress {
        phase: TrashProgressPhase::Finalizing,
        message: "휴지통 이동 결과와 작업 기록을 마무리하고 있습니다".to_owned(),
        processed_items: results.len(),
        total_items: requested_count,
    });
    if journal_complete
        && journal
            .record(json!({
                "timestampUnixMs": unix_time_ms(),
                "operationId": operation_id,
                "event": "completed",
                "actionKind": action_kind,
                "requestedCount": requested_count,
                "movedCount": moved_count,
                "movedBytes": moved_bytes,
                "cancelled": cancelled,
                "stoppedEarly": stopped_early,
            }))
            .is_err()
    {
        journal_complete = false;
    }

    Ok(TrashOperationResult {
        operation_id,
        requested_count,
        moved_count,
        moved_bytes,
        cancelled,
        stopped_early,
        journal_complete,
        journal_path: journal.path.to_string_lossy().into_owned(),
        items: results,
    })
}

fn emit_progress(
    app: &AppHandle,
    phase: TrashProgressPhase,
    message: String,
    processed_items: usize,
    total_items: usize,
) {
    let _ = app.emit(
        "trash-progress",
        TrashProgress {
            phase,
            message,
            processed_items,
            total_items,
        },
    );
}

fn validate_requested_count(count: usize) -> Result<(), String> {
    if count == 0 {
        return Err("휴지통으로 이동할 항목을 선택하세요".to_owned());
    }
    if count > MAX_TRASH_ITEMS {
        return Err(format!(
            "한 번에 최대 {MAX_TRASH_ITEMS}개까지 처리할 수 있습니다"
        ));
    }
    Ok(())
}

pub(crate) fn action_journal_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("action-journal.jsonl"))
        .map_err(|error| format!("작업 기록 위치를 확인하지 못했습니다: {error}"))
}

pub(crate) fn append_action_journal_records(
    path: PathBuf,
    records: impl IntoIterator<Item = serde_json::Value>,
) -> Result<(), String> {
    let mut journal = Journal::open(path)?;
    for record in records {
        journal.record(record)?;
    }
    Ok(())
}

fn operation_id() -> String {
    format!(
        "{}-{}-{}",
        unix_time_ms(),
        std::process::id(),
        OPERATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn item_result(
    item: &VerifiedTrashItem,
    status: TrashItemStatus,
    message: Option<String>,
) -> TrashItemResult {
    TrashItemResult {
        path: item.path().to_string_lossy().into_owned(),
        logical_bytes: item.logical_bytes(),
        status,
        message,
    }
}

#[cfg(windows)]
fn recovery_path_string(item: &VerifiedTrashItem) -> String {
    let path = item.recovery_path().to_string_lossy().replace('/', "\\");
    path.strip_prefix(r"\\?\UNC\")
        .map(|path| format!(r"\\{path}"))
        .or_else(|| path.strip_prefix(r"\\?\").map(str::to_owned))
        .unwrap_or(path)
}

#[cfg(not(windows))]
fn recovery_path_string(item: &VerifiedTrashItem) -> String {
    item.recovery_path().to_string_lossy().into_owned()
}

fn push_skipped(results: &mut Vec<TrashItemResult>, items: &[VerifiedTrashItem], message: &str) {
    results.extend(
        items
            .iter()
            .map(|item| item_result(item, TrashItemStatus::Skipped, Some(message.to_owned()))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bloomsweepy_core::{ScanConfig, scan_path};
    use std::sync::atomic::AtomicUsize;

    struct FailSecondMove {
        calls: AtomicUsize,
    }

    impl TrashBackend for FailSecondMove {
        fn move_to_trash(&self, _path: &Path) -> Result<(), String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 1 {
                Err("test failure".to_owned())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn partial_failure_stops_later_moves_and_records_each_outcome() {
        let temp = tempfile::tempdir().expect("create temp directory");
        for name in ["a.bin", "b.bin", "c.bin", "keeper.bin"] {
            fs::write(temp.path().join(name), b"identical").expect("write duplicate");
        }
        let report = scan_path(
            temp.path(),
            ScanConfig {
                min_large_file_bytes: 1,
                min_duplicate_file_bytes: 1,
                max_large_files: 10,
                max_duplicate_groups: 10,
                max_duplicate_candidates: 100,
                max_issues: 10,
            },
            |_| {},
            || false,
        )
        .expect("scan duplicates");
        let group = &report.duplicate_groups[0];
        let selected: Vec<String> = group
            .files
            .iter()
            .take(3)
            .map(|file| file.path.clone())
            .collect();
        let verified = validate_duplicate_trash_selection(&report.root, group, &selected, || false)
            .expect("validate selection");
        let journal_path = temp.path().join("journal.jsonl");
        let backend = FailSecondMove {
            calls: AtomicUsize::new(0),
        };

        let result = execute_verified_items(
            verified,
            journal_path.clone(),
            TrashActionKind::DuplicateFiles,
            &AtomicBool::new(false),
            |_| {},
            &backend,
        )
        .expect("execute trash plan");

        assert_eq!(result.moved_count, 1);
        assert!(result.stopped_early);
        assert_eq!(result.items[0].status, TrashItemStatus::Moved);
        assert_eq!(result.items[1].status, TrashItemStatus::Failed);
        assert_eq!(result.items[2].status, TrashItemStatus::Skipped);
        assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
        let journal = fs::read_to_string(journal_path).expect("read journal");
        assert!(journal.contains("\"event\":\"planned\""));
        assert!(journal.contains("\"event\":\"moved\""));
        assert!(journal.contains("\"event\":\"failed\""));
    }

    #[test]
    fn cancellation_before_the_first_move_never_calls_the_backend() {
        let temp = tempfile::tempdir().expect("create temp directory");
        for name in ["a.bin", "keeper.bin"] {
            fs::write(temp.path().join(name), b"identical").expect("write duplicate");
        }
        let report = scan_path(
            temp.path(),
            ScanConfig {
                min_large_file_bytes: 1,
                min_duplicate_file_bytes: 1,
                max_large_files: 10,
                max_duplicate_groups: 10,
                max_duplicate_candidates: 100,
                max_issues: 10,
            },
            |_| {},
            || false,
        )
        .expect("scan duplicates");
        let group = &report.duplicate_groups[0];
        let selected = vec![group.files[0].path.clone()];
        let verified = validate_duplicate_trash_selection(&report.root, group, &selected, || false)
            .expect("validate selection");
        let backend = FailSecondMove {
            calls: AtomicUsize::new(0),
        };
        let cancellation = AtomicBool::new(true);

        let result = execute_verified_items(
            verified,
            temp.path().join("cancel-journal.jsonl"),
            TrashActionKind::DuplicateFiles,
            &cancellation,
            |_| {},
            &backend,
        )
        .expect("cancel operation");

        assert!(result.cancelled);
        assert_eq!(result.moved_count, 0);
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
        assert_eq!(result.items[0].status, TrashItemStatus::Skipped);
    }

    #[test]
    fn action_journal_rotates_at_the_size_limit() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("action-journal.jsonl");
        let file = File::create(&path).expect("create journal");
        file.set_len(MAX_ACTION_JOURNAL_BYTES)
            .expect("extend journal to limit");
        drop(file);

        let mut journal = Journal::open(path.clone()).expect("open rotated journal");
        journal
            .record(json!({ "event": "test" }))
            .expect("write new journal");

        assert!(path.with_extension("previous.jsonl").exists());
        assert!(fs::metadata(path).expect("new journal metadata").len() < 1024);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "moves a tiny fixture to the Windows Recycle Bin"]
    fn real_windows_backend_moves_fixture_to_recycle_bin() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let fixture = temp.path().join(format!(
            "bloomsweepy-trash-smoke-{}-{}.txt",
            std::process::id(),
            unix_time_ms()
        ));
        fs::write(&fixture, b"BroomSweepy recycle-bin smoke test\n").expect("write fixture");

        SystemTrash
            .move_to_trash(&fixture)
            .expect("move fixture to Windows Recycle Bin");

        assert!(
            !fixture.exists(),
            "source path must disappear after trash move"
        );
    }
}
