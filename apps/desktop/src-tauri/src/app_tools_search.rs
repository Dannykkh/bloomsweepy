//! App-owned search/index/storage adapters. The model receives bounded evidence;
//! original paths and reports are reserved for local UI presentation.
use bloomsweepy_control::{
    AppToolRequest, AppToolResult, AppToolStatus, CleanupPlanReference, ControlCommand,
    ControlOperationState, DocumentSearchRequest, FileSearchRequest, IndexSource,
    OperationReference,
};
use bloomsweepy_core::{
    DocumentIndexConfig, FileCatalogConfig, FileCatalogSearchRequest as CoreFileRequest,
    ScanConfig, ScanPhase, ScanProgress, build_document_index, build_file_catalog,
    document_index_status, file_catalog_status, scan_cleanup_candidates,
    search_file_catalog_with_cancellation,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    ScanCompletionGuard, ScanRuntime, StoredReports, app_tools::ToolScope, control_server,
};

const SEARCH_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_ITEMS: usize = 24;
const MAX_NAME_CHARS: usize = 240;
const MAX_SNIPPET_CHARS: usize = 512;
const MAX_DATA_BYTES: usize = 15 * 1024;

pub(crate) async fn execute(
    app: &AppHandle,
    scope: &ToolScope,
    request: &AppToolRequest,
    cancellation: Arc<AtomicBool>,
) -> Result<Option<AppToolResult>, String> {
    request.validate().map_err(|error| error.to_string())?;
    if cancellation.load(Ordering::Acquire) {
        return Err("앱 조회를 취소했습니다".to_owned());
    }
    let id = request.capability_id();
    let result = match request {
        AppToolRequest::FileSearch { request } => {
            search_files(app, scope, request, cancellation).await?
        }
        AppToolRequest::DocumentSearch { request } => {
            search_documents(app, scope, request, cancellation).await?
        }
        AppToolRequest::IndexStatus { source } => index_status(app, scope, *source).await?,
        AppToolRequest::BuildIndex { source } => build_index(app, scope, *source)?,
        AppToolRequest::StorageScan {} => start_storage_scan(app, scope, cancellation)?,
        AppToolRequest::CleanupScan {} => start_cleanup_scan(app, scope)?,
        AppToolRequest::OperationStatus { operation_id } => {
            let raw = control_server::dispatch_operation_control(
                app,
                scope,
                ControlCommand::OperationStatus(
                    OperationReference::new(operation_id.clone())
                        .map_err(|error| error.to_string())?,
                ),
                cancellation,
            )?;
            operation_result(id, raw)
        }
        AppToolRequest::OperationCancel { operation_id } => {
            let raw = control_server::dispatch_operation_control(
                app,
                scope,
                ControlCommand::CancelOperation(
                    OperationReference::new(operation_id.clone())
                        .map_err(|error| error.to_string())?,
                ),
                cancellation,
            )?;
            operation_result(id, raw)
        }
        AppToolRequest::CleanupCandidates { request } => {
            if control_server::tool_cleanup_access(app).is_err() {
                return Ok(Some(permission(
                    id,
                    "앱에서 정리 검토 허용을 먼저 켜 주세요",
                )));
            }
            let raw = control_server::dispatch_tool_control(
                app,
                ControlCommand::CleanupCandidates(request.clone()),
                cancellation,
            )?;
            let mut result = AppToolResult::completed(id, raw.clone());
            result.truncated = raw["truncated"].as_bool().unwrap_or(false);
            result.presentation = Some(json!({"kind":"cleanupCandidates","report":raw}));
            result
        }
        AppToolRequest::CleanupReview { request } => {
            if control_server::tool_cleanup_access(app).is_err() {
                return Ok(Some(permission(
                    id,
                    "앱에서 정리 검토 허용을 먼저 켜 주세요",
                )));
            }
            let raw = control_server::dispatch_tool_control(
                app,
                ControlCommand::CreateCleanupPlan(request.clone()),
                cancellation,
            )?;
            AppToolResult::with_status(id, AppToolStatus::ReviewRequired, project_plan(&raw))
                .with_presentation(json!({"kind":"cleanupReview","planId":raw["planId"]}))
        }
        AppToolRequest::CleanupPlanStatus { plan_id } => {
            let raw = control_server::dispatch_tool_control(
                app,
                ControlCommand::CleanupPlanStatus(
                    CleanupPlanReference::new(plan_id.clone())
                        .map_err(|error| error.to_string())?,
                ),
                cancellation,
            )?;
            let status = cleanup_plan_tool_status(&raw);
            AppToolResult::with_status(id, status, project_plan(&raw))
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn permission(id: &str, message: &str) -> AppToolResult {
    AppToolResult::with_status(
        id,
        AppToolStatus::PermissionRequired,
        json!({"message":message,"permissionChanged":false}),
    )
}

fn native_root(scope: &ToolScope) -> Result<PathBuf, String> {
    let root = scope
        .root()
        .ok_or_else(|| "앱에서 대상 폴더를 선택한 대화가 필요합니다".to_owned())?;
    root.canonicalize()
        .map_err(|_| "선택한 대화 폴더를 다시 확인할 수 없습니다".to_owned())
}

fn index_root(app: &AppHandle, scope: &ToolScope, source: IndexSource) -> Result<PathBuf, String> {
    let documents = source == IndexSource::Documents;
    let allowed = if scope.is_external() || documents {
        Some(control_server::tool_search_root(app, documents)?)
    } else {
        None
    };
    select_index_root(
        scope.root(),
        scope.is_external(),
        documents,
        allowed.as_deref(),
    )
}

fn select_index_root(
    native: Option<&Path>,
    external: bool,
    documents: bool,
    allowed: Option<&Path>,
) -> Result<PathBuf, String> {
    if external {
        return allowed
            .map(Path::to_path_buf)
            .ok_or_else(|| "앱에서 외부 검색 범위를 먼저 허용해 주세요".to_owned());
    }
    let root = native
        .ok_or_else(|| "앱에서 대상 폴더를 선택한 대화가 필요합니다".to_owned())?
        .canonicalize()
        .map_err(|_| "선택한 대화 폴더를 확인할 수 없습니다".to_owned())?;
    if documents {
        let allowed = allowed.ok_or_else(|| "문서 내용 공개 허용이 필요합니다".to_owned())?;
        ensure_same_root(&root, allowed)?;
    }
    Ok(root)
}

fn ensure_same_root(expected: &Path, actual: &Path) -> Result<(), String> {
    let expected = expected
        .canonicalize()
        .map_err(|_| "선택한 검색 범위를 확인할 수 없습니다".to_owned())?;
    let actual = actual
        .canonicalize()
        .map_err(|_| "색인의 검색 범위를 확인할 수 없습니다".to_owned())?;
    if expected != actual {
        return Err(
            "선택한 대화 범위와 색인/공개 허용 범위가 다릅니다. 앱에서 범위를 다시 선택해 주세요"
                .to_owned(),
        );
    }
    Ok(())
}

async fn read_index(app: &AppHandle, source: IndexSource) -> Result<Option<Value>, String> {
    let path = if source == IndexSource::Documents {
        super::document_index_path(app)?
    } else {
        super::file_catalog_path(app)?
    };
    tauri::async_runtime::spawn_blocking(move || {
        if source == IndexSource::Documents {
            document_index_status(path)
                .map_err(|_| "문서 색인 상태를 읽지 못했습니다".to_owned())?
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| "문서 색인 상태를 표시하지 못했습니다".to_owned())
        } else {
            file_catalog_status(path)
                .map_err(|_| "파일 색인 상태를 읽지 못했습니다".to_owned())?
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| "파일 색인 상태를 표시하지 못했습니다".to_owned())
        }
    })
    .await
    .map_err(|_| "앱 색인 상태 조회가 중단됐습니다".to_owned())?
}

async fn index_status(
    app: &AppHandle,
    scope: &ToolScope,
    source: IndexSource,
) -> Result<AppToolResult, String> {
    let expected = match index_root(app, scope, source) {
        Ok(root) => root,
        Err(_) => {
            return Ok(permission(
                "index.status",
                if source == IndexSource::Documents {
                    "문서 내용 일부를 AI로 공개하려면 앱의 문서 검색 허용과 이 대화의 폴더 범위를 일치시켜 주세요"
                } else {
                    "앱에서 파일 검색 범위를 먼저 선택/허용해 주세요"
                },
            ));
        }
    };
    let status = read_index(app, source).await?;
    let current = match index_root(app, scope, source) {
        Ok(root) => root,
        Err(_) => {
            return Ok(permission(
                "index.status",
                "조회 중 검색/문서 공개 허용이 바뀌어 색인 정보를 전달하지 않았습니다",
            ));
        }
    };
    if ensure_same_root(&expected, &current).is_err() {
        return Ok(permission(
            "index.status",
            "조회 중 선택/허용한 색인 범위가 바뀌었습니다. 앱에서 범위를 다시 확인해 주세요",
        ));
    }
    if let Some(status) = &status {
        let actual = status["root"]
            .as_str()
            .ok_or_else(|| "색인 범위를 확인하지 못했습니다".to_owned())?;
        if ensure_same_root(&expected, Path::new(actual)).is_err() {
            return Ok(permission(
                "index.status",
                "선택한 폴더와 현재 색인의 범위가 다릅니다. 선택한 폴더의 색인을 앱에서 갱신해 주세요",
            ));
        }
    }
    let data = status
        .as_ref()
        .map(project_index)
        .unwrap_or_else(|| json!({"available":false,"buildRequired":true}));
    Ok(AppToolResult::completed(
        "index.status",
        json!({"source":source,"index":data,
        "freshness":"completed index snapshot; not a live filesystem view"}),
    )
    .with_presentation(json!({"kind":"indexStatus","source":source,"index":status})))
}

async fn search_files(
    app: &AppHandle,
    scope: &ToolScope,
    request: &FileSearchRequest,
    cancellation: Arc<AtomicBool>,
) -> Result<AppToolResult, String> {
    let expected = match index_root(app, scope, IndexSource::Files) {
        Ok(root) => root,
        Err(_) => {
            return Ok(permission(
                "files.search",
                "앱에서 파일 검색 범위를 먼저 선택/허용해 주세요",
            ));
        }
    };
    let index = read_index(app, IndexSource::Files).await?;
    let Some(index) = index else {
        return Ok(missing_index("files.search", IndexSource::Files));
    };
    ensure_same_root(
        &expected,
        Path::new(index["root"].as_str().unwrap_or_default()),
    )?;
    let raw = if scope.is_external() {
        let app = app.clone();
        let request = request.clone();
        tauri::async_runtime::spawn_blocking(move || {
            control_server::dispatch_tool_control(
                &app,
                ControlCommand::SearchFiles(request),
                cancellation,
            )
        })
        .await
        .map_err(|_| "앱 파일 검색이 중단됐습니다".to_owned())??
    } else {
        let request: CoreFileRequest = serde_json::from_value(
            serde_json::to_value(request)
                .map_err(|_| "파일 검색 조건을 읽지 못했습니다".to_owned())?,
        )
        .map_err(|_| "파일 검색 조건을 읽지 못했습니다".to_owned())?;
        let runtime_cancel = app.state::<ScanRuntime>().begin()?;
        let _completion = ScanCompletionGuard::new(app.clone());
        let path = super::file_catalog_path(app)?;
        tauri::async_runtime::spawn_blocking(move || {
            let deadline = Instant::now() + SEARCH_TIMEOUT;
            let report = search_file_catalog_with_cancellation(path, request, move || {
                cancellation.load(Ordering::Acquire)
                    || runtime_cancel.load(Ordering::Acquire)
                    || Instant::now() >= deadline
            })
            .map_err(|_| {
                "앱 파일 검색을 완료하지 못했습니다. 조건을 좁히거나 다시 시도해 주세요".to_owned()
            })?;
            serde_json::to_value(report)
                .map_err(|_| "파일 검색 결과를 표시하지 못했습니다".to_owned())
        })
        .await
        .map_err(|_| "앱 파일 검색이 중단됐습니다".to_owned())??
    };
    ensure_same_root(
        &expected,
        Path::new(raw["root"].as_str().unwrap_or_default()),
    )?;
    validate_result_paths(&raw)?;
    if scope.is_external() {
        ensure_same_root(&expected, &control_server::tool_search_root(app, false)?)?;
    }
    let data = project_search(&raw, &index, false);
    let truncated = data["truncated"].as_bool().unwrap_or(false);
    let mut result = AppToolResult::completed("files.search", data)
        .with_presentation(json!({"kind":"fileSearch","report":raw,"index":index}));
    result.truncated = truncated;
    Ok(result)
}

async fn search_documents(
    app: &AppHandle,
    scope: &ToolScope,
    request: &DocumentSearchRequest,
    cancellation: Arc<AtomicBool>,
) -> Result<AppToolResult, String> {
    let expected = match index_root(app, scope, IndexSource::Documents) {
        Ok(root) => root,
        Err(_) => {
            return Ok(permission(
                "documents.search",
                "문서 일치 내용 일부를 AI로 전달하려면 앱의 문서 검색 공개를 허용하고 대화 폴더와 범위를 일치시켜 주세요",
            ));
        }
    };
    let index = read_index(app, IndexSource::Documents).await?;
    let Some(index) = index else {
        return Ok(missing_index("documents.search", IndexSource::Documents));
    };
    ensure_same_root(
        &expected,
        Path::new(index["root"].as_str().unwrap_or_default()),
    )?;
    // Use the existing approved-document service for BOTH callers. The consent
    // gate is checked immediately before querying, not only at the prior status read.
    let app_for_search = app.clone();
    let request = request.clone();
    let raw = tauri::async_runtime::spawn_blocking(move || {
        control_server::dispatch_tool_control(
            &app_for_search,
            ControlCommand::SearchDocuments(request),
            cancellation,
        )
    })
    .await
    .map_err(|_| "앱 문서 검색이 중단됐습니다".to_owned())??;
    ensure_same_root(
        &expected,
        Path::new(raw["root"].as_str().unwrap_or_default()),
    )?;
    validate_result_paths(&raw)?;
    // Revocation during the query must not release excerpts to the model.
    let allowed = control_server::tool_search_root(app, true)?;
    ensure_same_root(&expected, &allowed)?;
    let data = project_search(&raw, &index, true);
    let truncated = data["truncated"].as_bool().unwrap_or(false);
    let mut result = AppToolResult::completed("documents.search", data)
        .with_presentation(json!({"kind":"documentSearch","report":raw,"index":index}));
    result.truncated = truncated;
    Ok(result)
}

fn build_index(
    app: &AppHandle,
    scope: &ToolScope,
    source: IndexSource,
) -> Result<AppToolResult, String> {
    let root = match index_root(app, scope, source) {
        Ok(root) => root,
        Err(_) => {
            return Ok(permission(
                "index.build",
                "앱에서 이 폴더의 검색/문서 공개 범위를 먼저 선택/허용해 주세요",
            ));
        }
    };
    let path = if source == IndexSource::Documents {
        super::document_index_path(app)?
    } else {
        super::file_catalog_path(app)?
    };
    let reservation = control_server::reserve_tool_operation(
        app,
        scope,
        if source == IndexSource::Documents {
            "documentIndex"
        } else {
            "fileIndex"
        },
        Some(&root),
    )?;
    let raw = serde_json::to_value(&reservation.operation)
        .map_err(|_| "색인 작업 상태를 표시하지 못했습니다".to_owned())?;
    let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = reservation.completion;
        let id = reservation.operation.operation_id;
        let cancel = reservation.cancellation;
        let progress_app = task_app.clone();
        let progress_id = id.clone();
        let result = if source == IndexSource::Documents {
            let cancel_for_build = Arc::clone(&cancel);
            build_document_index(
                root,
                path,
                DocumentIndexConfig::default(),
                move |progress| {
                    let _ = progress_app.emit("document-index-progress", &progress);
                    record_progress(
                        &progress_app,
                        &progress_id,
                        progress.message,
                        progress.scanned_files,
                        progress.processed_bytes,
                    );
                },
                move || cancel_for_build.load(Ordering::Acquire),
            )
            .map_err(|_| {
                "앱 문서 색인 갱신을 완료하지 못했습니다. 기존 색인은 보존됩니다".to_owned()
            })
            .and_then(|report| {
                serde_json::to_value(report)
                    .map_err(|_| "문서 색인 결과를 표시하지 못했습니다".to_owned())
            })
        } else {
            let cancel_for_build = Arc::clone(&cancel);
            build_file_catalog(
                root,
                path,
                FileCatalogConfig::default(),
                move |progress| {
                    let _ = progress_app.emit("file-catalog-progress", &progress);
                    record_progress(
                        &progress_app,
                        &progress_id,
                        progress.message,
                        progress.scanned_entries,
                        progress.processed_bytes,
                    );
                },
                move || cancel_for_build.load(Ordering::Acquire),
            )
            .map_err(|_| {
                "앱 파일 색인 갱신을 완료하지 못했습니다. 기존 색인은 보존됩니다".to_owned()
            })
            .and_then(|report| {
                serde_json::to_value(report)
                    .map_err(|_| "파일 색인 결과를 표시하지 못했습니다".to_owned())
            })
        };
        let (state, message) = completion_state(&result, cancel.load(Ordering::Acquire));
        if let Ok(index) = &result {
            let _ = task_app.emit(
                "app-tool-completed",
                json!({"operationId":id,
                "presentation":{"kind":"indexStatus","source":source,"index":index}}),
            );
        }
        control_server::finish_tool_operation(&task_app, &id, state, message, None, None);
    });
    Ok(operation_result("index.build", raw))
}

fn start_storage_scan(
    app: &AppHandle,
    scope: &ToolScope,
    cancellation: Arc<AtomicBool>,
) -> Result<AppToolResult, String> {
    if scope.is_external() {
        return match control_server::dispatch_tool_control(
            app,
            ControlCommand::StartStorageScan,
            cancellation,
        ) {
            Ok(raw) => Ok(operation_result("storage.scan", raw)),
            Err(_) => Ok(permission(
                "storage.scan",
                "앱에서 검사할 폴더를 먼저 선택해 주세요",
            )),
        };
    }
    let root = match native_root(scope) {
        Ok(root) => root,
        Err(_) => {
            return Ok(permission(
                "storage.scan",
                "앱에서 검사할 폴더를 선택한 대화가 필요합니다",
            ));
        }
    };
    let reservation =
        control_server::reserve_tool_operation(app, scope, "storageScan", Some(&root))?;
    if let Err(error) = app.state::<StoredReports>().clear_scan() {
        control_server::finish_tool_operation(
            app,
            &reservation.operation.operation_id,
            ControlOperationState::Failed,
            "앱 검사 결과를 준비하지 못했습니다".to_owned(),
            None,
            None,
        );
        return Err(error);
    }
    let raw = serde_json::to_value(&reservation.operation)
        .map_err(|_| "검사 작업 상태를 표시하지 못했습니다".to_owned())?;
    let task_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let id = reservation.operation.operation_id;
        let result = super::execute_reserved_scan(
            task_app.clone(),
            root,
            ScanConfig::default(),
            reservation.cancellation,
            reservation.completion.clone(),
            super::ScanProgressTarget::Control {
                operation_id: id.clone(),
            },
        )
        .await;
        let (state, message, generation, summary) = match result {
            Ok(snapshot) => (
                ControlOperationState::Completed,
                "앱의 폴더 검사를 완료했습니다".to_owned(),
                Some(snapshot.generation),
                Some(control_server::tool_storage_summary(&snapshot)),
            ),
            Err(error) if error.is_cancelled() => (
                ControlOperationState::Cancelled,
                "앱 검사를 취소했습니다".to_owned(),
                None,
                None,
            ),
            Err(_) => (
                ControlOperationState::Failed,
                "앱 폴더 검사를 완료하지 못했습니다".to_owned(),
                None,
                None,
            ),
        };
        control_server::finish_tool_operation(&task_app, &id, state, message, generation, summary);
        drop(reservation.completion);
    });
    Ok(operation_result("storage.scan", raw))
}

fn start_cleanup_scan(app: &AppHandle, scope: &ToolScope) -> Result<AppToolResult, String> {
    if control_server::tool_cleanup_access(app).is_err() {
        return Ok(permission(
            "cleanup.scan",
            "앱에서 시스템 정리 검토 허용을 먼저 켜 주세요",
        ));
    }
    let reservation = control_server::reserve_tool_operation(app, scope, "systemCleanup", None)?;
    if let Err(error) = app.state::<StoredReports>().clear_cleanup() {
        control_server::finish_tool_operation(
            app,
            &reservation.operation.operation_id,
            ControlOperationState::Failed,
            "앱 정리 결과를 준비하지 못했습니다".to_owned(),
            None,
            None,
        );
        return Err(error);
    }
    let raw = serde_json::to_value(&reservation.operation)
        .map_err(|_| "정리 검사 작업 상태를 표시하지 못했습니다".to_owned())?;
    let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = reservation.completion;
        let id = reservation.operation.operation_id;
        let cancel = reservation.cancellation;
        let result = (|| -> Result<(u64, Value), String> {
            let installed =
                super::installed_app_inventory_with_cancellation(|| cancel.load(Ordering::Acquire))
                    .map_err(|_| {
                        "정리 검사를 위한 설치 앱 확인을 완료하지 못했습니다".to_owned()
                    })?;
            let progress_app = task_app.clone();
            let progress_id = id.clone();
            let report = scan_cleanup_candidates(
                super::cleanup_scan_config(&installed),
                move |progress| {
                    let _ = progress_app.emit("cleanup-scan-progress", &progress);
                    record_progress(
                        &progress_app,
                        &progress_id,
                        progress.message,
                        progress.processed_entries,
                        progress.processed_bytes,
                    );
                },
                || cancel.load(Ordering::Acquire),
            )
            .map_err(|_| "앱의 시스템 정리 후보 검사를 완료하지 못했습니다".to_owned())?;
            let registry = super::registry_residue_inventory_with_cancellation(|| {
                cancel.load(Ordering::Acquire)
            })
            .map_err(|_| "정리 검사 부가 정보를 확인하지 못했습니다".to_owned())?;
            if !task_app.state::<ScanRuntime>().begin_control_commit(&id)? {
                return Err("시스템 정리 검사를 취소했습니다".to_owned());
            }
            let generation = task_app.state::<StoredReports>().replace_cleanup(&report)?;
            Ok((
                generation,
                json!({"report":report,"registryResidues":registry}),
            ))
        })();
        let (state, message) = completion_state(&result, cancel.load(Ordering::Acquire));
        let generation = result.as_ref().ok().map(|(generation, _)| *generation);
        if let Ok((_, report)) = &result {
            let _ = task_app.emit(
                "app-tool-completed",
                json!({"operationId":id,
                "presentation":{"kind":"cleanupScan","result":report}}),
            );
        }
        control_server::finish_tool_operation(&task_app, &id, state, message, generation, None);
    });
    Ok(operation_result("cleanup.scan", raw))
}

fn record_progress(app: &AppHandle, id: &str, message: String, items: u64, bytes: u64) {
    control_server::record_scan_progress(
        app,
        id,
        ScanProgress {
            phase: ScanPhase::Discovering,
            message,
            processed_files: items,
            processed_bytes: bytes,
            fraction: None,
        },
    );
}

fn completion_state<T>(
    result: &Result<T, String>,
    cancelled: bool,
) -> (ControlOperationState, String) {
    match result {
        Ok(_) => (
            ControlOperationState::Completed,
            "앱 조회 작업을 완료했습니다".to_owned(),
        ),
        Err(_) if cancelled => (
            ControlOperationState::Cancelled,
            "앱 조회 작업을 취소했습니다".to_owned(),
        ),
        Err(message) => (ControlOperationState::Failed, message.clone()),
    }
}

fn operation_result(id: &str, raw: Value) -> AppToolResult {
    let status = match raw["state"].as_str() {
        Some("running" | "queued") => AppToolStatus::Running,
        Some("failed") => AppToolStatus::Failed,
        _ => AppToolStatus::Completed,
    };
    AppToolResult::with_status(id, status, project_operation(&raw))
        .with_presentation(json!({"kind":"operation","operation":raw}))
}

fn project_index(raw: &Value) -> Value {
    let mut result = copy_fields(
        raw,
        &[
            "completedAtUnixMs",
            "durationMs",
            "indexedEntries",
            "indexedFiles",
            "indexedDirectories",
            "indexedSymlinks",
            "indexedBytes",
            "unreadableEntries",
            "entryLimitReached",
            "provider",
            "refreshMode",
            "indexedDocuments",
            "supportedExtensions",
            "documentLimitReached",
        ],
    );
    result["available"] = json!(true);
    result
}

fn missing_index(id: &str, source: IndexSource) -> AppToolResult {
    AppToolResult::completed(
        id,
        json!({"available":false,"buildRequired":true,"items":[],
        "searched":false,"scopeBasis":"selected session/index root"}),
    )
    .with_presentation(json!({"kind":"indexStatus","source":source,"index":null}))
}

fn validate_result_paths(raw: &Value) -> Result<(), String> {
    let root = Path::new(raw["root"].as_str().unwrap_or_default());
    if root.as_os_str().is_empty()
        || raw["results"].as_array().is_none_or(|items| {
            items.iter().any(|item| {
                item["path"]
                    .as_str()
                    .is_none_or(|path| !Path::new(path).starts_with(root))
            })
        })
    {
        return Err("색인 검색 결과가 선택한 폴더 범위를 벗어나 표시하지 않았습니다".to_owned());
    }
    Ok(())
}

fn project_search(raw: &Value, index: &Value, documents: bool) -> Value {
    let mut data = copy_fields(
        raw,
        &[
            "indexedEntries",
            "searchedDocuments",
            "totalMatches",
            "searchDurationMs",
        ],
    );
    data["available"] = json!(true);
    data["index"] = project_index(index);
    data["freshness"] = json!("completed index snapshot; not a live filesystem view");
    data["scopeBasis"] = json!("selected session/index root; not the currently browsed subfolder");
    data["contentExcerptsShared"] = json!(documents);
    let mut items = Vec::new();
    let source = raw["results"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    for (index, item) in source.iter().take(MAX_ITEMS).enumerate() {
        let mut row = copy_fields(
            item,
            &[
                "extension",
                "kind",
                "format",
                "logicalBytes",
                "modifiedAtUnixMs",
                "matchSource",
            ],
        );
        row["number"] = json!(index + 1);
        row["name"] = json!(
            item["name"]
                .as_str()
                .unwrap_or_default()
                .chars()
                .take(MAX_NAME_CHARS)
                .collect::<String>()
        );
        if item["kind"] == "directory" {
            row["logicalBytes"] = Value::Null;
            row["sizeMeasured"] = json!(false);
        }
        if documents {
            row["snippet"] = json!(
                item["snippet"]
                    .as_array()
                    .map(|parts| parts
                        .iter()
                        .filter_map(|part| part["text"].as_str())
                        .collect::<String>())
                    .unwrap_or_default()
                    .chars()
                    .take(MAX_SNIPPET_CHARS)
                    .collect::<String>()
            );
        }
        items.push(row);
        data["items"] = json!(&items);
        // Reserve the final count/truncation fields as well as JSON punctuation.
        if serde_json::to_vec(&data).is_ok_and(|encoded| encoded.len() > MAX_DATA_BYTES - 256) {
            items.pop();
            break;
        }
    }
    data["truncated"] =
        json!(raw["resultsTruncated"].as_bool().unwrap_or(false) || items.len() < source.len());
    data["returnedCount"] = json!(items.len());
    data["items"] = json!(items);
    data
}

fn project_operation(raw: &Value) -> Value {
    let mut result = copy_fields(
        raw,
        &[
            "operationId",
            "kind",
            "state",
            "cancellationRequested",
            "processedItems",
            "processedBytes",
            "startedAtUnixMs",
            "finishedAtUnixMs",
            "scanGeneration",
        ],
    );
    if !raw["summary"].is_null() {
        result["summary"] = copy_fields(
            &raw["summary"],
            &[
                "completedAtUnixMs",
                "durationMs",
                "totalFiles",
                "totalLogicalBytes",
                "largeFileCount",
                "duplicateGroupCount",
                "duplicateWasteBytes",
                "unreadableEntries",
                "issueCount",
                "candidateLimitReached",
                "hardLinkIdentityLimitReached",
            ],
        );
    }
    result
}

fn project_plan(raw: &Value) -> Value {
    let mut result = copy_fields(
        raw,
        &[
            "planId",
            "state",
            "source",
            "sourceGeneration",
            "itemCount",
            "totalBytes",
            "reviewCount",
            "expiresAtUnixMs",
        ],
    );
    result["executionAuthority"] = json!("final user confirmation in main app only");
    if !raw["result"].is_null() {
        result["result"] = copy_fields(
            &raw["result"],
            &["movedCount", "failedCount", "skippedCount", "movedBytes"],
        );
    }
    result
}

fn cleanup_plan_tool_status(raw: &Value) -> AppToolStatus {
    match raw["state"].as_str() {
        Some("awaitingApproval") => AppToolStatus::ReviewRequired,
        Some("executing") => AppToolStatus::Running,
        Some("failed") => AppToolStatus::Failed,
        _ => AppToolStatus::Completed,
    }
}

fn copy_fields(raw: &Value, fields: &[&str]) -> Value {
    let mut object = serde_json::Map::new();
    for field in fields {
        if let Some(value) = raw.get(*field) {
            object.insert((*field).to_owned(), value.clone());
        }
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projections_remove_paths_and_preserve_unknown_folder_sizes() {
        let raw = json!({"root":"/private/root","resultsTruncated":true,"indexedEntries":9,
            "results":[{"name":"folder","path":"/private/root/folder","parent":"/private/root",
                "kind":"directory","logicalBytes":0,"matchSource":"name"}]});
        let index = json!({"root":"/private/root","completedAtUnixMs":17,"entryLimitReached":true});
        let result = project_search(&raw, &index, false);
        assert_eq!(result["items"][0]["logicalBytes"], Value::Null);
        assert_eq!(result["items"][0]["sizeMeasured"], false);
        assert_eq!(result["index"]["completedAtUnixMs"], 17);
        assert_eq!(result["truncated"], true);
        assert!(!serde_json::to_string(&result).unwrap().contains("/private"));
    }

    #[test]
    fn document_projection_is_bounded_and_does_not_promote_excerpts_to_authority() {
        let raw = json!({"root":"/private/root","results":(0..30).map(|_| json!({
            "name":"한".repeat(400),"path":"/private/root/secret","format":"plainText",
            "snippet":[{"text":"문".repeat(2000),"highlighted":true}]})).collect::<Vec<_>>()});
        let result = project_search(&raw, &json!({"completedAtUnixMs":1}), true);
        assert!(result["items"].as_array().unwrap().len() <= MAX_ITEMS);
        assert!(serde_json::to_vec(&result).unwrap().len() <= MAX_DATA_BYTES);
        assert!(
            result["items"][0]["snippet"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= MAX_SNIPPET_CHARS
        );
        assert_eq!(result["contentExcerptsShared"], true);
        assert_eq!(result["truncated"], true);
        assert!(!serde_json::to_string(&result).unwrap().contains("/private"));
    }

    #[test]
    fn root_matching_rejects_siblings_and_document_consent_scope_changes() {
        let temp = tempfile::tempdir().unwrap();
        let left = temp.path().join("selected");
        let right = temp.path().join("selected-other");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        assert!(ensure_same_root(&left, &left).is_ok());
        assert!(ensure_same_root(&left, &right).is_err());
        assert!(ensure_same_root(&left, &temp.path().join("missing")).is_err());
        assert!(select_index_root(Some(&left), false, false, None).is_ok());
        assert!(select_index_root(Some(&left), false, true, None).is_err());
        assert!(select_index_root(Some(&left), false, true, Some(&left)).is_ok());
        assert!(select_index_root(Some(&left), false, true, Some(&right)).is_err());
        assert!(select_index_root(None, true, false, None).is_err());
        assert!(select_index_root(None, true, true, Some(&left)).is_ok());
    }

    #[test]
    fn operation_and_plan_projection_never_release_local_file_errors_or_results() {
        let raw = json!({"operationId":"x","state":"running","message":"/private/root failed",
            "summary":{"root":"/private/root","totalFiles":2,"candidateLimitReached":true}});
        assert!(
            !serde_json::to_string(&project_operation(&raw))
                .unwrap()
                .contains("/private")
        );
        let plan = project_plan(&json!({"planId":"x","state":"completed",
            "result":{"movedCount":1,"moved":["/private/root"]}}));
        assert_eq!(plan["result"]["movedCount"], 1);
        assert!(!serde_json::to_string(&plan).unwrap().contains("/private"));
    }

    #[test]
    fn permission_results_do_not_enable_any_access() {
        let result = permission("documents.search", "permission required");
        assert_eq!(result.status, AppToolStatus::PermissionRequired);
        assert_eq!(result.data["permissionChanged"], false);
        assert!(result.presentation.is_none());
    }

    #[test]
    fn missing_index_is_not_a_successful_empty_search_and_bad_snapshot_paths_are_rejected() {
        let missing = missing_index("documents.search", IndexSource::Documents);
        assert_eq!(missing.data["available"], false);
        assert_eq!(missing.data["searched"], false);
        assert_eq!(missing.data["buildRequired"], true);
        assert_eq!(missing.presentation.unwrap()["index"], Value::Null);
        assert!(
            validate_result_paths(
                &json!({"root":"/selected","results":[{"path":"/selected/file"}]})
            )
            .is_ok()
        );
        assert!(
            validate_result_paths(
                &json!({"root":"/selected","results":[{"path":"/selected-other/file"}]})
            )
            .is_err()
        );
        assert!(validate_result_paths(&json!({"root":"/selected","results":[{}]})).is_err());
    }

    #[test]
    fn executing_and_failed_cleanup_plans_are_not_reported_completed() {
        assert_eq!(
            cleanup_plan_tool_status(&json!({"state":"awaitingApproval"})),
            AppToolStatus::ReviewRequired
        );
        assert_eq!(
            cleanup_plan_tool_status(&json!({"state":"executing"})),
            AppToolStatus::Running
        );
        assert_eq!(
            cleanup_plan_tool_status(&json!({"state":"failed"})),
            AppToolStatus::Failed
        );
        assert_eq!(
            cleanup_plan_tool_status(&json!({"state":"completed"})),
            AppToolStatus::Completed
        );
    }
}
