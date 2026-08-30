use super::{ScanCompletionGuard, ScanRuntime};
use crate::trash_actions::{
    MAX_ACTION_JOURNAL_BYTES, action_journal_path, append_action_journal_records,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};

const MAX_RECOVERY_OPERATIONS: usize = 200;
const MAX_RECOVERY_ITEMS_PER_OPERATION: usize = 500;
const MAX_REPORT_OPERATIONS: usize = 50;
const MAX_RECOVERY_ISSUES: usize = 50;
const TRASH_TIME_TOLERANCE_MS: u128 = 15_000;
const MAX_RECOVERY_JOURNAL_BYTES: u64 = MAX_ACTION_JOURNAL_BYTES * 2;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActionRecoveryReport {
    checked_at_unix_ms: u128,
    journal_path: String,
    trash_lookup_supported: bool,
    trash_lookup_performed: bool,
    incomplete_operations: Vec<RecoveryOperation>,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryOperation {
    operation_id: String,
    started_at_unix_ms: u128,
    planned_count: usize,
    resolved: bool,
    audit_saved: bool,
    attention_count: usize,
    items: Vec<RecoveryItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryItem {
    path: String,
    logical_bytes: u64,
    status: RecoveryItemStatus,
    needs_attention: bool,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RecoveryItemStatus {
    NotStarted,
    OriginalPresent,
    RecordedMoved,
    FoundInTrash,
    RecordedFailed,
    OriginalAndTrash,
    Missing,
    TrashLookupUnavailable,
    AccessUnknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalRecord {
    operation_id: String,
    event: String,
    #[serde(default)]
    timestamp_unix_ms: u128,
    #[serde(default)]
    items: Vec<JournalPlannedItem>,
    path: Option<String>,
    logical_bytes: Option<u64>,
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalPlannedItem {
    path: String,
    logical_bytes: u64,
}

#[derive(Debug, Default)]
struct OperationState {
    operation_id: String,
    started_at_unix_ms: u128,
    items: BTreeMap<String, JournalPlannedItem>,
    item_events: HashMap<String, RecordedItemEvent>,
    completed: bool,
    reconciled: bool,
    seen_in_current_journal: bool,
}

#[derive(Debug, Clone)]
enum RecordedItemEvent {
    Moving {
        timestamp_unix_ms: u128,
        logical_bytes: u64,
    },
    Moved,
    Failed {
        timestamp_unix_ms: u128,
        moving_at_unix_ms: u128,
        logical_bytes: u64,
        message: String,
    },
}

struct LoadedOperations {
    operations: HashMap<String, OperationState>,
    journal_integrity_ok: bool,
}

struct ParsedJournalRecord {
    record: JournalRecord,
    is_current_journal: bool,
}

#[derive(Debug, Clone)]
struct TrashEvidence {
    deleted_at_unix_ms: u128,
    file_bytes: Option<u64>,
}

#[derive(Debug, Default)]
struct TrashLookup {
    supported: bool,
    performed: bool,
    error: Option<String>,
    by_original_path: HashMap<String, Vec<TrashEvidence>>,
}

#[tauri::command]
pub(crate) async fn get_action_recovery_status(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
) -> Result<ActionRecoveryReport, String> {
    let _cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let journal_path = action_journal_path(&app)?;

    tauri::async_runtime::spawn_blocking(move || inspect_action_recovery(journal_path))
        .await
        .map_err(|error| format!("이전 휴지통 작업 확인을 실행하지 못했습니다: {error}"))?
}

#[tauri::command]
pub(crate) fn open_system_trash() -> Result<(), String> {
    open_system_trash_impl()
}

fn inspect_action_recovery(journal_path: PathBuf) -> Result<ActionRecoveryReport, String> {
    let mut issues = Vec::new();
    let journal_files = [
        (journal_path.with_extension("previous.jsonl"), false),
        (journal_path.clone(), true),
    ];
    let loaded = load_operations(&journal_files, &mut issues);
    let journal_integrity_ok = loaded.journal_integrity_ok;
    let mut operations = loaded.operations;
    operations.retain(|_, operation| {
        !operation.completed
            && !operation.reconciled
            && operation.item_events.values().any(|event| {
                matches!(
                    event,
                    RecordedItemEvent::Moving { .. }
                        | RecordedItemEvent::Moved
                        | RecordedItemEvent::Failed { .. }
                )
            })
    });

    let ambiguous_paths: HashSet<String> = operations
        .values()
        .flat_map(|operation| {
            operation.item_events.iter().filter_map(|(key, event)| {
                matches!(
                    event,
                    RecordedItemEvent::Moving { .. } | RecordedItemEvent::Failed { .. }
                )
                .then(|| operation.items.get(key))
                .flatten()
                .map(|item| recovery_match_key(Path::new(&item.path)))
            })
        })
        .collect();
    let trash_lookup = collect_trash_lookup(&ambiguous_paths);
    if let Some(error) = trash_lookup.error.as_ref() {
        push_issue(
            &mut issues,
            format!("운영체제 휴지통 목록을 확인하지 못했습니다: {error}"),
        );
    }

    let mut report_operations: Vec<RecoveryOperation> = operations
        .values()
        .map(|operation| reconcile_operation(operation, &trash_lookup))
        .collect();
    report_operations.sort_unstable_by(|left, right| {
        right
            .started_at_unix_ms
            .cmp(&left.started_at_unix_ms)
            .then_with(|| right.operation_id.cmp(&left.operation_id))
    });

    let resolved_ids: HashSet<String> = report_operations
        .iter()
        .filter(|operation| operation.resolved)
        .map(|operation| operation.operation_id.clone())
        .collect();
    let mut audit_records = Vec::new();
    if journal_integrity_ok {
        for operation in operations.values().filter(|operation| {
            !operation.seen_in_current_journal && !resolved_ids.contains(&operation.operation_id)
        }) {
            audit_records.extend(checkpoint_records(operation));
        }
        audit_records.extend(resolved_ids.iter().map(|operation_id| {
            json!({
                "schemaVersion": 1,
                "timestampUnixMs": unix_time_ms(),
                "operationId": operation_id,
                "event": "reconciled",
                "method": "journalOriginalPathAndOsTrash",
            })
        }));
    }
    if !audit_records.is_empty() {
        match append_action_journal_records(journal_path.clone(), audit_records) {
            Ok(()) => {
                for operation in &mut report_operations {
                    if operation.resolved {
                        operation.audit_saved = true;
                    }
                }
            }
            Err(error) => push_issue(
                &mut issues,
                format!("자동 대조 결과를 작업 기록에 저장하지 못했습니다: {error}"),
            ),
        }
    } else if !resolved_ids.is_empty() && !journal_integrity_ok {
        push_issue(
            &mut issues,
            "손상되거나 제한을 넘은 작업 기록이 있어 자동 대조 결과를 덧쓰지 않았습니다".to_owned(),
        );
    }

    if report_operations.len() > MAX_REPORT_OPERATIONS {
        push_issue(
            &mut issues,
            format!("중단 작업이 많아 최근 {MAX_REPORT_OPERATIONS}건만 화면에 표시합니다"),
        );
        report_operations.truncate(MAX_REPORT_OPERATIONS);
    }

    Ok(ActionRecoveryReport {
        checked_at_unix_ms: unix_time_ms(),
        journal_path: journal_path.to_string_lossy().into_owned(),
        trash_lookup_supported: trash_lookup.supported,
        trash_lookup_performed: trash_lookup.performed,
        incomplete_operations: report_operations,
        issues,
    })
}

fn load_operations(
    journal_files: &[(PathBuf, bool)],
    issues: &mut Vec<String>,
) -> LoadedOperations {
    let mut parsed_records = Vec::new();
    let mut journal_integrity_ok = true;
    for (journal_file, is_current_journal) in journal_files {
        let file = match File::open(journal_file) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                journal_integrity_ok = false;
                push_issue(
                    issues,
                    format!(
                        "작업 기록을 열지 못했습니다 ({}): {error}",
                        journal_file.display()
                    ),
                );
                continue;
            }
        };
        match file.metadata() {
            Ok(metadata) if metadata.len() > MAX_RECOVERY_JOURNAL_BYTES => {
                journal_integrity_ok = false;
                push_issue(
                    issues,
                    format!(
                        "작업 기록이 안전한 읽기 한도 {}바이트를 넘었습니다 ({})",
                        MAX_RECOVERY_JOURNAL_BYTES,
                        journal_file.display()
                    ),
                );
                continue;
            }
            Ok(_) => {}
            Err(error) => {
                journal_integrity_ok = false;
                push_issue(
                    issues,
                    format!(
                        "작업 기록 크기를 확인하지 못했습니다 ({}): {error}",
                        journal_file.display()
                    ),
                );
                continue;
            }
        }
        for (line_index, line) in BufReader::new(file).lines().enumerate() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    journal_integrity_ok = false;
                    push_issue(
                        issues,
                        format!(
                            "작업 기록 {}의 {}번째 줄을 읽지 못했습니다: {error}",
                            journal_file.display(),
                            line_index + 1,
                        ),
                    );
                    continue;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            let record: JournalRecord = match serde_json::from_str(&line) {
                Ok(record) => record,
                Err(error) => {
                    journal_integrity_ok = false;
                    push_issue(
                        issues,
                        format!(
                            "작업 기록 {}의 {}번째 줄 형식이 손상됐습니다: {error}",
                            journal_file.display(),
                            line_index + 1,
                        ),
                    );
                    continue;
                }
            };
            parsed_records.push(ParsedJournalRecord {
                record,
                is_current_journal: *is_current_journal,
            });
        }
    }

    let terminal_operations: HashSet<&str> = parsed_records
        .iter()
        .filter(|parsed| matches!(parsed.record.event.as_str(), "completed" | "reconciled"))
        .map(|parsed| parsed.record.operation_id.as_str())
        .collect();
    let action_operations: HashSet<&str> = parsed_records
        .iter()
        .filter(|parsed| matches!(parsed.record.event.as_str(), "moving" | "moved" | "failed"))
        .map(|parsed| parsed.record.operation_id.as_str())
        .collect();
    let incomplete_count = action_operations
        .iter()
        .filter(|operation_id| !terminal_operations.contains(**operation_id))
        .count();
    if incomplete_count > MAX_RECOVERY_OPERATIONS {
        journal_integrity_ok = false;
        push_issue(
            issues,
            format!(
                "중단 작업이 분석 상한 {MAX_RECOVERY_OPERATIONS}건을 넘어 최근 작업만 확인합니다"
            ),
        );
    }

    let mut selected_operations = HashSet::new();
    for parsed in parsed_records.iter().rev() {
        let operation_id = parsed.record.operation_id.as_str();
        if terminal_operations.contains(operation_id)
            || !action_operations.contains(operation_id)
            || selected_operations.contains(operation_id)
        {
            continue;
        }
        if selected_operations.len() >= MAX_RECOVERY_OPERATIONS {
            break;
        }
        selected_operations.insert(operation_id.to_owned());
    }

    let mut operations = HashMap::new();
    for parsed in parsed_records {
        if selected_operations.contains(&parsed.record.operation_id) {
            apply_record(
                &mut operations,
                parsed.record,
                parsed.is_current_journal,
                issues,
            );
        }
    }
    LoadedOperations {
        operations,
        journal_integrity_ok,
    }
}

fn apply_record(
    operations: &mut HashMap<String, OperationState>,
    record: JournalRecord,
    is_current_journal: bool,
    issues: &mut Vec<String>,
) {
    let operation = operations
        .entry(record.operation_id.clone())
        .or_insert_with(|| OperationState {
            operation_id: record.operation_id.clone(),
            started_at_unix_ms: record.timestamp_unix_ms,
            ..OperationState::default()
        });
    operation.seen_in_current_journal |= is_current_journal;
    if operation.started_at_unix_ms == 0
        || (record.timestamp_unix_ms > 0 && record.timestamp_unix_ms < operation.started_at_unix_ms)
    {
        operation.started_at_unix_ms = record.timestamp_unix_ms;
    }

    match record.event.as_str() {
        "planned" => {
            let item_count = record.items.len();
            for item in record
                .items
                .into_iter()
                .take(MAX_RECOVERY_ITEMS_PER_OPERATION)
            {
                operation.items.insert(path_key(&item.path), item);
            }
            if item_count > MAX_RECOVERY_ITEMS_PER_OPERATION {
                push_issue(
                    issues,
                    format!(
                        "작업 {}의 항목이 분석 상한 {}개를 넘었습니다",
                        operation.operation_id, MAX_RECOVERY_ITEMS_PER_OPERATION
                    ),
                );
            }
        }
        "moving" | "moved" | "failed" => {
            let Some(path) = record.path else {
                push_issue(
                    issues,
                    format!(
                        "작업 {}의 {} 기록에 경로가 없습니다",
                        operation.operation_id, record.event
                    ),
                );
                return;
            };
            let key = path_key(&path);
            let logical_bytes = record.logical_bytes.unwrap_or_else(|| {
                operation
                    .items
                    .get(&key)
                    .map(|item| item.logical_bytes)
                    .unwrap_or_default()
            });
            operation
                .items
                .entry(key.clone())
                .or_insert(JournalPlannedItem {
                    path,
                    logical_bytes,
                });
            let event = match record.event.as_str() {
                "moving" => RecordedItemEvent::Moving {
                    timestamp_unix_ms: record.timestamp_unix_ms,
                    logical_bytes,
                },
                "moved" => RecordedItemEvent::Moved,
                _ => {
                    let moving_at_unix_ms = match operation.item_events.get(&key) {
                        Some(RecordedItemEvent::Moving {
                            timestamp_unix_ms, ..
                        }) => *timestamp_unix_ms,
                        Some(RecordedItemEvent::Failed {
                            moving_at_unix_ms, ..
                        }) => *moving_at_unix_ms,
                        _ => record.timestamp_unix_ms,
                    };
                    RecordedItemEvent::Failed {
                        timestamp_unix_ms: record.timestamp_unix_ms,
                        moving_at_unix_ms,
                        logical_bytes,
                        message: record
                            .message
                            .unwrap_or_else(|| "휴지통 이동 실패".to_owned()),
                    }
                }
            };
            operation.item_events.insert(key, event);
        }
        "completed" => operation.completed = true,
        "reconciled" => operation.reconciled = true,
        _ => {}
    }
}

fn checkpoint_records(operation: &OperationState) -> Vec<serde_json::Value> {
    let planned_items: Vec<serde_json::Value> = operation
        .items
        .values()
        .map(|item| {
            json!({
                "path": item.path,
                "logicalBytes": item.logical_bytes,
            })
        })
        .collect();
    let mut records = vec![json!({
        "schemaVersion": 1,
        "timestampUnixMs": operation.started_at_unix_ms,
        "operationId": operation.operation_id,
        "event": "planned",
        "recovery": "osTrash",
        "checkpoint": true,
        "items": planned_items,
    })];

    for (key, event) in &operation.item_events {
        let Some(item) = operation.items.get(key) else {
            continue;
        };
        match event {
            RecordedItemEvent::Moving {
                timestamp_unix_ms,
                logical_bytes,
            } => records.push(json!({
                "schemaVersion": 1,
                "timestampUnixMs": timestamp_unix_ms,
                "operationId": operation.operation_id,
                "event": "moving",
                "path": item.path,
                "logicalBytes": logical_bytes,
                "checkpoint": true,
            })),
            RecordedItemEvent::Moved => records.push(json!({
                "schemaVersion": 1,
                "timestampUnixMs": unix_time_ms(),
                "operationId": operation.operation_id,
                "event": "moved",
                "path": item.path,
                "logicalBytes": item.logical_bytes,
                "checkpoint": true,
            })),
            RecordedItemEvent::Failed {
                timestamp_unix_ms,
                moving_at_unix_ms,
                logical_bytes,
                message,
            } => {
                records.push(json!({
                    "schemaVersion": 1,
                    "timestampUnixMs": moving_at_unix_ms,
                    "operationId": operation.operation_id,
                    "event": "moving",
                    "path": item.path,
                    "logicalBytes": logical_bytes,
                    "checkpoint": true,
                }));
                records.push(json!({
                    "schemaVersion": 1,
                    "timestampUnixMs": timestamp_unix_ms,
                    "operationId": operation.operation_id,
                    "event": "failed",
                    "path": item.path,
                    "logicalBytes": logical_bytes,
                    "message": message,
                    "checkpoint": true,
                }));
            }
        }
    }
    records
}

fn reconcile_operation(operation: &OperationState, lookup: &TrashLookup) -> RecoveryOperation {
    let mut items: Vec<RecoveryItem> = operation
        .items
        .iter()
        .map(|(key, planned)| {
            let event = operation.item_events.get(key);
            reconcile_item(planned, event, lookup)
        })
        .collect();
    items.sort_unstable_by(|left, right| {
        right
            .needs_attention
            .cmp(&left.needs_attention)
            .then_with(|| left.path.cmp(&right.path))
    });
    let attention_count = items.iter().filter(|item| item.needs_attention).count();
    RecoveryOperation {
        operation_id: operation.operation_id.clone(),
        started_at_unix_ms: operation.started_at_unix_ms,
        planned_count: operation.items.len(),
        resolved: attention_count == 0,
        audit_saved: false,
        attention_count,
        items,
    }
}

fn reconcile_item(
    planned: &JournalPlannedItem,
    event: Option<&RecordedItemEvent>,
    lookup: &TrashLookup,
) -> RecoveryItem {
    let (status, needs_attention, message) = match event {
        None => (
            RecoveryItemStatus::NotStarted,
            false,
            "이 항목의 이동 시작 기록이 없어 원본을 변경하지 않았습니다".to_owned(),
        ),
        Some(RecordedItemEvent::Moved) => (
            RecoveryItemStatus::RecordedMoved,
            false,
            "휴지통 이동 완료 기록이 디스크에 남아 있습니다".to_owned(),
        ),
        Some(RecordedItemEvent::Failed {
            moving_at_unix_ms,
            logical_bytes,
            message,
            ..
        }) => reconcile_uncertain_item(
            Path::new(&planned.path),
            *moving_at_unix_ms,
            *logical_bytes,
            lookup,
            Some(message),
        ),
        Some(RecordedItemEvent::Moving {
            timestamp_unix_ms,
            logical_bytes,
        }) => reconcile_moving_item(
            Path::new(&planned.path),
            *timestamp_unix_ms,
            *logical_bytes,
            lookup,
        ),
    };
    RecoveryItem {
        path: planned.path.clone(),
        logical_bytes: planned.logical_bytes,
        status,
        needs_attention,
        message,
    }
}

fn reconcile_moving_item(
    path: &Path,
    moving_at_unix_ms: u128,
    logical_bytes: u64,
    lookup: &TrashLookup,
) -> (RecoveryItemStatus, bool, String) {
    reconcile_uncertain_item(path, moving_at_unix_ms, logical_bytes, lookup, None)
}

fn reconcile_uncertain_item(
    path: &Path,
    moving_at_unix_ms: u128,
    logical_bytes: u64,
    lookup: &TrashLookup,
    recorded_failure: Option<&str>,
) -> (RecoveryItemStatus, bool, String) {
    let original = match fs::symlink_metadata(path) {
        Ok(_) => OriginalPresence::Present,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => OriginalPresence::Missing,
        Err(error) => OriginalPresence::Unknown(error.to_string()),
    };
    let evidence = lookup
        .by_original_path
        .get(&recovery_match_key(path))
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.deleted_at_unix_ms.abs_diff(moving_at_unix_ms) <= TRASH_TIME_TOLERANCE_MS
                    && item
                        .file_bytes
                        .is_none_or(|file_bytes| file_bytes == logical_bytes)
            })
        });

    match (original, lookup.error.as_ref(), lookup.supported, evidence) {
        (OriginalPresence::Present, _, _, true) => (
            RecoveryItemStatus::OriginalAndTrash,
            true,
            "같은 원래 경로가 현재도 존재하고 휴지통에도 일치 항목이 있어 자동 확정할 수 없습니다".to_owned(),
        ),
        (OriginalPresence::Present, Some(error), _, false) => (
            RecoveryItemStatus::TrashLookupUnavailable,
            true,
            format!("원본은 존재하지만 휴지통 대조가 실패했습니다: {error}"),
        ),
        (OriginalPresence::Present, None, _, false) => (
            recorded_failure
                .map(|_| RecoveryItemStatus::RecordedFailed)
                .unwrap_or(RecoveryItemStatus::OriginalPresent),
            false,
            recorded_failure.map_or_else(
                || "원본 경로가 남아 있고 같은 시각의 휴지통 항목이 없어 이동되지 않은 것으로 확인했습니다".to_owned(),
                |message| format!("이동 실패 기록과 원본 경로를 확인했습니다: {message}"),
            ),
        ),
        (OriginalPresence::Missing, _, _, true) => (
            RecoveryItemStatus::FoundInTrash,
            false,
            "원본은 사라졌고 같은 경로·시각의 항목을 운영체제 휴지통에서 찾았습니다".to_owned(),
        ),
        (OriginalPresence::Missing, Some(error), _, false) => (
            RecoveryItemStatus::TrashLookupUnavailable,
            true,
            format!("원본이 없고 휴지통 대조도 실패했습니다: {error}"),
        ),
        (OriginalPresence::Missing, None, false, false) => (
            RecoveryItemStatus::TrashLookupUnavailable,
            true,
            "이 운영체제에서는 휴지통 목록을 조회할 수 없어 원본이 사라진 이유를 자동 확정할 수 없습니다".to_owned(),
        ),
        (OriginalPresence::Missing, None, true, false) => (
            RecoveryItemStatus::Missing,
            true,
            "원본 경로와 일치하는 휴지통 항목을 모두 찾지 못했습니다".to_owned(),
        ),
        (OriginalPresence::Unknown(error), _, _, _) => (
            RecoveryItemStatus::AccessUnknown,
            true,
            format!("원본 경로 상태를 확인하지 못했습니다: {error}"),
        ),
    }
}

enum OriginalPresence {
    Present,
    Missing,
    Unknown(String),
}

#[cfg(any(
    windows,
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    )
))]
fn collect_trash_lookup(paths: &HashSet<String>) -> TrashLookup {
    let mut lookup = TrashLookup {
        supported: true,
        performed: !paths.is_empty(),
        ..TrashLookup::default()
    };
    if paths.is_empty() {
        return lookup;
    }
    let items = match trash::os_limited::list() {
        Ok(items) => items,
        Err(error) => {
            lookup.error = Some(error.to_string());
            return lookup;
        }
    };
    for item in items {
        let original_path = item.original_path();
        let key = path_key(&original_path.to_string_lossy());
        if !paths.contains(&key) || item.time_deleted < 0 {
            continue;
        }
        let file_bytes = trash::os_limited::metadata(&item)
            .ok()
            .and_then(|metadata| metadata.size.size());
        lookup
            .by_original_path
            .entry(key)
            .or_default()
            .push(TrashEvidence {
                deleted_at_unix_ms: (item.time_deleted as u128).saturating_mul(1_000),
                file_bytes,
            });
    }
    lookup
}

#[cfg(not(any(
    windows,
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    )
)))]
fn collect_trash_lookup(paths: &HashSet<String>) -> TrashLookup {
    TrashLookup {
        supported: false,
        performed: !paths.is_empty(),
        ..TrashLookup::default()
    }
}

#[cfg(windows)]
fn path_key(path: &str) -> String {
    let normalized = path.replace('/', "\\");
    let normalized = normalized
        .strip_prefix(r"\\?\UNC\")
        .map(|path| format!(r"\\{path}"))
        .or_else(|| normalized.strip_prefix(r"\\?\").map(str::to_owned))
        .unwrap_or(normalized);
    normalized.trim_end_matches('\\').to_lowercase()
}

#[cfg(windows)]
fn recovery_match_key(path: &Path) -> String {
    let expanded = path
        .parent()
        .zip(path.file_name())
        .and_then(|(parent, name)| {
            fs::canonicalize(parent)
                .ok()
                .map(|parent| parent.join(name))
        });
    path_key(&expanded.as_deref().unwrap_or(path).to_string_lossy())
}

#[cfg(not(windows))]
fn path_key(path: &str) -> String {
    path.to_owned()
}

#[cfg(not(windows))]
fn recovery_match_key(path: &Path) -> String {
    path_key(&path.to_string_lossy())
}

#[cfg(windows)]
fn open_system_trash_impl() -> Result<(), String> {
    Command::new("explorer.exe")
        .arg("shell:RecycleBinFolder")
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Windows 휴지통을 열지 못했습니다: {error}"))
}

#[cfg(target_os = "macos")]
fn open_system_trash_impl() -> Result<(), String> {
    let trash_path = dirs::home_dir()
        .map(|home| home.join(".Trash"))
        .ok_or_else(|| "사용자 휴지통 경로를 찾지 못했습니다".to_owned())?;
    Command::new("open")
        .arg(trash_path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("macOS 휴지통을 열지 못했습니다: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_system_trash_impl() -> Result<(), String> {
    Command::new("gio")
        .args(["open", "trash:///"])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("시스템 휴지통을 열지 못했습니다: {error}"))
}

#[cfg(not(any(windows, unix)))]
fn open_system_trash_impl() -> Result<(), String> {
    Err("이 운영체제에서는 휴지통 열기를 지원하지 않습니다".to_owned())
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn push_issue(issues: &mut Vec<String>, message: String) {
    if issues.len() < MAX_RECOVERY_ISSUES && !issues.contains(&message) {
        issues.push(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn planned(path: &Path, logical_bytes: u64) -> JournalPlannedItem {
        JournalPlannedItem {
            path: path.to_string_lossy().into_owned(),
            logical_bytes,
        }
    }

    fn moving_operation(path: &Path, moving_at: u128) -> OperationState {
        let item = planned(path, 12);
        let key = path_key(&item.path);
        OperationState {
            operation_id: "operation-1".to_owned(),
            started_at_unix_ms: moving_at.saturating_sub(10),
            items: BTreeMap::from([(key.clone(), item)]),
            item_events: HashMap::from([(
                key,
                RecordedItemEvent::Moving {
                    timestamp_unix_ms: moving_at,
                    logical_bytes: 12,
                },
            )]),
            ..OperationState::default()
        }
    }

    #[test]
    fn moving_record_with_original_and_no_trash_is_resolved_as_not_moved() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("still-here.bin");
        fs::write(&path, b"still-here!!").expect("write original");
        let operation = moving_operation(&path, 100_000);
        let lookup = TrashLookup {
            supported: true,
            performed: true,
            ..TrashLookup::default()
        };

        let report = reconcile_operation(&operation, &lookup);

        assert!(report.resolved);
        assert_eq!(report.items[0].status, RecoveryItemStatus::OriginalPresent);
    }

    #[test]
    fn missing_original_with_matching_trash_evidence_is_resolved_as_moved() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("moved.bin");
        let operation = moving_operation(&path, 100_000);
        let lookup = TrashLookup {
            supported: true,
            performed: true,
            by_original_path: HashMap::from([(
                recovery_match_key(&path),
                vec![TrashEvidence {
                    deleted_at_unix_ms: 101_000,
                    file_bytes: Some(12),
                }],
            )]),
            ..TrashLookup::default()
        };

        let report = reconcile_operation(&operation, &lookup);

        assert!(report.resolved);
        assert_eq!(report.items[0].status, RecoveryItemStatus::FoundInTrash);
    }

    #[test]
    fn missing_original_without_trash_evidence_requires_attention() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("unknown.bin");
        let operation = moving_operation(&path, 100_000);
        let lookup = TrashLookup {
            supported: true,
            performed: true,
            ..TrashLookup::default()
        };

        let report = reconcile_operation(&operation, &lookup);

        assert!(!report.resolved);
        assert_eq!(report.attention_count, 1);
        assert_eq!(report.items[0].status, RecoveryItemStatus::Missing);
    }

    #[test]
    fn original_and_matching_trash_evidence_requires_manual_attention() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("recreated.bin");
        fs::write(&path, b"still-here!!").expect("write original");
        let operation = moving_operation(&path, 100_000);
        let lookup = TrashLookup {
            supported: true,
            performed: true,
            by_original_path: HashMap::from([(
                recovery_match_key(&path),
                vec![TrashEvidence {
                    deleted_at_unix_ms: 101_000,
                    file_bytes: Some(12),
                }],
            )]),
            ..TrashLookup::default()
        };

        let report = reconcile_operation(&operation, &lookup);

        assert!(!report.resolved);
        assert_eq!(report.items[0].status, RecoveryItemStatus::OriginalAndTrash);
    }

    #[test]
    fn recorded_failure_is_only_resolved_after_the_original_is_observed() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("failed.bin");
        fs::write(&path, b"still-here!!").expect("write original");
        let mut operation = moving_operation(&path, 100_000);
        let key = path_key(&path.to_string_lossy());
        operation.item_events.insert(
            key,
            RecordedItemEvent::Failed {
                timestamp_unix_ms: 101_000,
                moving_at_unix_ms: 100_000,
                logical_bytes: 12,
                message: "backend error".to_owned(),
            },
        );
        let lookup = TrashLookup {
            supported: true,
            performed: true,
            ..TrashLookup::default()
        };

        let report = reconcile_operation(&operation, &lookup);

        assert!(report.resolved);
        assert_eq!(report.items[0].status, RecoveryItemStatus::RecordedFailed);
    }

    #[test]
    fn trash_evidence_outside_the_crash_window_is_not_accepted() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("stale.bin");
        let operation = moving_operation(&path, 100_000);
        let lookup = TrashLookup {
            supported: true,
            performed: true,
            by_original_path: HashMap::from([(
                recovery_match_key(&path),
                vec![TrashEvidence {
                    deleted_at_unix_ms: 200_000,
                    file_bytes: Some(12),
                }],
            )]),
            ..TrashLookup::default()
        };

        let report = reconcile_operation(&operation, &lookup);

        assert!(!report.resolved);
        assert_eq!(report.items[0].status, RecoveryItemStatus::Missing);
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_keys_collapse_verbatim_drive_paths() {
        assert_eq!(
            path_key(r"\\?\C:\Temp\Example.bin"),
            path_key(r"c:\temp\example.bin")
        );
        assert_eq!(
            path_key(r"\\?\UNC\server\share\Example.bin"),
            path_key(r"\\server\share\example.bin")
        );
    }

    #[test]
    fn completed_operations_are_ignored_and_malformed_lines_are_reported() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let journal_path = temp.path().join("action-journal.jsonl");
        let mut journal = File::create(&journal_path).expect("create journal");
        writeln!(journal, "not-json").expect("write malformed line");
        writeln!(
            journal,
            "{}",
            json!({
                "operationId": "done",
                "event": "planned",
                "timestampUnixMs": 1,
                "items": [{ "path": temp.path().join("a").to_string_lossy(), "logicalBytes": 1 }]
            })
        )
        .expect("write plan");
        writeln!(
            journal,
            "{}",
            json!({
                "operationId": "done",
                "event": "completed",
                "timestampUnixMs": 2
            })
        )
        .expect("write completion");
        drop(journal);
        let mut issues = Vec::new();

        let loaded = load_operations(&[(journal_path, true)], &mut issues);

        assert!(loaded.operations.is_empty());
        assert!(!loaded.journal_integrity_ok);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn previous_journal_checkpoint_preserves_the_latest_item_state() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("checkpoint.bin");
        let operation = moving_operation(&path, 123_000);

        let records = checkpoint_records(&operation);
        let mut reloaded = HashMap::new();
        let mut issues = Vec::new();
        for value in records {
            let record = serde_json::from_value(value).expect("decode checkpoint record");
            apply_record(&mut reloaded, record, true, &mut issues);
        }

        let reloaded = &reloaded["operation-1"];
        assert!(reloaded.seen_in_current_journal);
        assert!(issues.is_empty());
        assert!(matches!(
            reloaded.item_events.values().next(),
            Some(RecordedItemEvent::Moving {
                timestamp_unix_ms: 123_000,
                logical_bytes: 12,
            })
        ));
    }

    #[test]
    fn operation_limit_keeps_the_most_recent_incomplete_records() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let journal_path = temp.path().join("action-journal.jsonl");
        let mut journal = File::create(&journal_path).expect("create journal");
        for index in 0..=MAX_RECOVERY_OPERATIONS {
            let operation_id = format!("operation-{index:03}");
            let path = temp.path().join(format!("item-{index:03}.bin"));
            writeln!(
                journal,
                "{}",
                json!({
                    "operationId": operation_id,
                    "event": "planned",
                    "timestampUnixMs": index,
                    "items": [{ "path": path.to_string_lossy(), "logicalBytes": 1 }]
                })
            )
            .expect("write plan");
            writeln!(
                journal,
                "{}",
                json!({
                    "operationId": operation_id,
                    "event": "moving",
                    "timestampUnixMs": index,
                    "path": path.to_string_lossy(),
                    "logicalBytes": 1
                })
            )
            .expect("write moving record");
        }
        drop(journal);
        let mut issues = Vec::new();

        let loaded = load_operations(&[(journal_path, true)], &mut issues);

        assert_eq!(loaded.operations.len(), MAX_RECOVERY_OPERATIONS);
        assert!(!loaded.journal_integrity_ok);
        assert!(loaded.operations.contains_key("operation-200"));
        assert!(!loaded.operations.contains_key("operation-000"));
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn oversized_journal_is_not_loaded_or_modified() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let journal_path = temp.path().join("action-journal.jsonl");
        let journal = File::create(&journal_path).expect("create journal");
        journal
            .set_len(MAX_RECOVERY_JOURNAL_BYTES + 1)
            .expect("extend sparse journal");
        drop(journal);
        let mut issues = Vec::new();

        let loaded = load_operations(&[(journal_path.clone(), true)], &mut issues);

        assert!(loaded.operations.is_empty());
        assert!(!loaded.journal_integrity_ok);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            fs::metadata(journal_path).expect("journal metadata").len(),
            MAX_RECOVERY_JOURNAL_BYTES + 1
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "temporarily moves and restores a tiny fixture through the Windows Recycle Bin"]
    fn real_windows_recovery_finds_and_restores_a_moving_fixture() {
        let stale_smoke_items: Vec<_> = trash::os_limited::list()
            .expect("list recycle bin for stale smoke cleanup")
            .into_iter()
            .filter(|item| {
                let name = item.name.to_string_lossy();
                name.starts_with("bloomsweepy-recovery-smoke-")
                    || name.starts_with("bloomsweepy-trash-smoke-")
            })
            .collect();
        if !stale_smoke_items.is_empty() {
            trash::os_limited::purge_all(&stale_smoke_items)
                .expect("purge stale recovery smoke fixtures");
        }
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp
            .path()
            .join(format!("bloomsweepy-recovery-smoke-{}.bin", unix_time_ms()));
        fs::write(&path, b"still-here!!").expect("write fixture");
        let journal_path = temp.path().join("action-journal.jsonl");
        let moving_at = unix_time_ms();
        let mut journal = File::create(&journal_path).expect("create journal");
        writeln!(
            journal,
            "{}",
            json!({
                "operationId": "real-windows-recovery",
                "event": "planned",
                "timestampUnixMs": moving_at,
                "items": [{ "path": path.to_string_lossy(), "logicalBytes": 12 }]
            })
        )
        .expect("write plan");
        writeln!(
            journal,
            "{}",
            json!({
                "operationId": "real-windows-recovery",
                "event": "moving",
                "timestampUnixMs": moving_at,
                "path": path.to_string_lossy(),
                "logicalBytes": 12
            })
        )
        .expect("write moving record");
        drop(journal);
        trash::delete(&path).expect("move fixture to recycle bin");

        let inspection = inspect_action_recovery(journal_path);
        let expected_name = path.file_name().expect("fixture name").to_owned();
        let matching_item = trash::os_limited::list()
            .expect("list recycle bin for cleanup")
            .into_iter()
            .find(|item| item.name == expected_name)
            .expect("find fixture for restore");
        trash::os_limited::restore_all([matching_item]).expect("restore fixture");
        let report = inspection.expect("inspect interrupted action");

        assert!(path.exists());
        assert_eq!(report.incomplete_operations.len(), 1);
        assert_eq!(
            report.incomplete_operations[0].items[0].status,
            RecoveryItemStatus::FoundInTrash
        );
        assert!(report.incomplete_operations[0].audit_saved);
    }
}
