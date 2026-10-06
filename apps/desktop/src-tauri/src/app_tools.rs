//! Single composition point for app-owned inspection and review, never final execution.
use bloomsweepy_control::{AppToolRequest, AppToolResult, AppToolStatus};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub(crate) struct RequestHistory(std::sync::Mutex<std::collections::VecDeque<String>>);

/// Read-only queries can repeat; starts/reviews/cancellation with the same wire
/// request ID cannot silently replace a review or start another expensive job.
pub(crate) fn reject_replayed_mutation(
    app: &AppHandle,
    request_id: &str,
    request: &AppToolRequest,
) -> Result<(), String> {
    let state = app.state::<RequestHistory>();
    let mut history = state
        .0
        .lock()
        .map_err(|_| "요청 기록을 확인하지 못했습니다".to_owned())?;
    record_mutating_request(&mut history, request_id, request)
}

fn record_mutating_request(
    history: &mut std::collections::VecDeque<String>,
    request_id: &str,
    request: &AppToolRequest,
) -> Result<(), String> {
    if !matches!(
        request,
        AppToolRequest::BuildIndex { .. }
            | AppToolRequest::StorageScan {}
            | AppToolRequest::CleanupScan {}
            | AppToolRequest::ApplicationReview { .. }
            | AppToolRequest::ProcessReview { .. }
            | AppToolRequest::MemoryReview {}
            | AppToolRequest::DockerReview { .. }
            | AppToolRequest::CleanupReview { .. }
            | AppToolRequest::OperationCancel { .. }
    ) && !matches!(request, AppToolRequest::FileWorkspace { operation }
            if !matches!(operation, bloomsweepy_control::FileWorkspaceAction::Status {}
                | bloomsweepy_control::FileWorkspaceAction::Page { .. }))
    {
        return Ok(());
    }
    if history.iter().any(|id| id == request_id) {
        return Err("이미 처리한 검토·시작 요청입니다. 최신 상태를 조회하거나 새 요청으로 다시 검토해 주세요".to_owned());
    }
    if history.len() >= 128 {
        history.pop_front();
    }
    history.push_back(request_id.to_owned());
    Ok(())
}

pub(crate) enum ToolScope {
    Native {
        session_id: String,
        root: Option<PathBuf>,
    },
    External,
}

impl ToolScope {
    pub(crate) fn root(&self) -> Option<&Path> {
        match self {
            Self::Native { root, .. } => root.as_deref(),
            Self::External => None,
        }
    }
    pub(crate) fn is_external(&self) -> bool {
        matches!(self, Self::External)
    }
}

pub(crate) async fn execute(
    app: &AppHandle,
    scope: &ToolScope,
    request: &AppToolRequest,
    cancellation: Arc<AtomicBool>,
) -> Result<AppToolResult, String> {
    request.validate().map_err(|error| error.to_string())?;
    if cancellation.load(Ordering::Acquire) {
        return Err("조회가 취소되었습니다".to_owned());
    }
    if let ToolScope::Native { session_id, root } = scope {
        let session =
            crate::assistant_sessions::get_assistant_session(app.clone(), session_id.clone())
                .await?;
        let expected = (session.session.scope_kind
            == crate::assistant_provider::AssistantScopeKind::Folder)
            .then(|| PathBuf::from(session.session.scope_root));
        if expected.as_ref() != root.as_ref() {
            return Err("대화 조회 범위가 변경되었습니다".to_owned());
        }
    }
    if scope.is_external()
        && request.requires_inspection_access()
        && !crate::control_server::inspection_access_allowed(app)?
    {
        return Ok(AppToolResult::with_status(
            request.capability_id(),
            AppToolStatus::PermissionRequired,
            json!({"reason":"설정에서 외부 AI의 시스템·앱 조회를 허용해 주세요", "permission":"inspection", "performed":false}),
        ));
    }
    let mut result = if let AppToolRequest::FileWorkspace { operation } = request {
        crate::assistant_files::execute_tool(app, scope, operation, Arc::clone(&cancellation))
            .await?
    } else if let Some(result) =
        crate::app_tools_system::execute(app, scope, request, Arc::clone(&cancellation)).await?
    {
        result
    } else if let Some(result) =
        crate::app_tools_search::execute(app, scope, request, Arc::clone(&cancellation)).await?
    {
        result
    } else {
        match request {
            AppToolRequest::Capabilities {} => AppToolResult::completed(
                request.capability_id(),
                json!({"catalog":bloomsweepy_control::discovery_index(),"platform":std::env::consts::OS,
                    "externalInspectionAllowed":crate::control_server::inspection_access_allowed(app)?}),
            ),
            AppToolRequest::CapabilityDetails { capability_id } => AppToolResult::completed(
                request.capability_id(),
                bloomsweepy_control::capability_details(capability_id)
                    .map_err(|error| error.to_string())?,
            ),
            AppToolRequest::StorageOverview {} => {
                let overview = tauri::async_runtime::spawn_blocking(crate::collect_system_overview)
                    .await
                    .map_err(|error| format!("드라이브 조회가 중단됐습니다: {error}"))?;
                // Mount paths stay on the computer; only measured labels/capacity leave it.
                let local = serde_json::to_value(overview).map_err(|error| error.to_string())?;
                let mut data = local.clone();
                if let Some(disks) = data.get_mut("volumes").and_then(Value::as_array_mut) {
                    for disk in disks {
                        if let Some(disk) = disk.as_object_mut() {
                            disk.remove("mountPoint");
                            disk.remove("path");
                        }
                    }
                }
                AppToolResult::completed(request.capability_id(), data)
                    .with_presentation(json!({"view":"dashboard","overview":local}))
            }
            AppToolRequest::DockerStatus {} => {
                let context = crate::docker_tools::assistant_context(app).await?;
                AppToolResult::completed(
                    request.capability_id(),
                    serde_json::to_value(context).map_err(|error| error.to_string())?,
                )
                .with_presentation(json!({"view":"docker"}))
            }
            AppToolRequest::DockerReview { actions } => {
                let preview = crate::docker_tools::create_docker_cleanup_preview(
                    app.clone(),
                    app.state::<crate::docker_tools::DockerManagerState>(),
                )
                .await?;
                AppToolResult::with_status(request.capability_id(), AppToolStatus::ReviewRequired,
                    json!({"actions":actions,"executed":false,"volumesExcluded":true,"approval":"local confirmation only"}))
                    .with_presentation(json!({"view":"docker","reviewKind":"docker","preview":preview,"actions":actions}))
            }
            AppToolRequest::View { view } => AppToolResult::completed(
                request.capability_id(),
                json!({"view":view,"performed":false}),
            )
            .with_presentation(json!({"view":view,"navigationRequested":true})),
            _ => AppToolResult::with_status(
                request.capability_id(),
                AppToolStatus::Unsupported,
                json!({"reason":"이 플랫폼에서 지원하지 않는 앱 기능입니다","performed":false}),
            ),
        }
    };
    // Consent can be withdrawn while inventory/Docker collection is awaiting I/O.
    if scope.is_external()
        && request.requires_inspection_access()
        && !crate::control_server::inspection_access_allowed(app)?
    {
        return Ok(AppToolResult::with_status(
            request.capability_id(),
            AppToolStatus::PermissionRequired,
            json!({"reason":"조회 공개 허용이 해제되어 결과를 전달하지 않았습니다","performed":false}),
        ));
    }
    // A per-result cap also applies to external callers; presentation is local-only.
    bound_model_result(&mut result)?;
    if scope.is_external() && result.status == AppToolStatus::ReviewRequired {
        app.emit("app-tool-review", &result)
            .map_err(|error| error.to_string())?;
    }
    Ok(result)
}

fn bound_model_result(result: &mut AppToolResult) -> Result<(), String> {
    if model_context(result)?.len() > 16 * 1024 {
        result.truncated = true;
        result.data = json!({"reason":"조회 결과가 전송 상한을 넘었습니다. 검색어나 페이지를 좁혀 주세요", "itemsOmitted":true});
    }
    Ok(())
}

pub(crate) fn model_context(result: &AppToolResult) -> Result<String, String> {
    let mut safe = result.clone();
    safe.presentation = None;
    serde_json::to_string(&safe).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_receives_evidence_not_local_paths_or_execution_ids() {
        let result = AppToolResult::completed(
            "applications.list",
            json!({"items":[{"name":"Editor","id":"opaque"}]}),
        )
        .with_presentation(json!({"planId":"local-secret","path":"/Users/private/app"}));
        let context = model_context(&result).unwrap();
        assert!(context.contains("Editor"));
        assert!(!context.contains("presentation"));
        assert!(!context.contains("local-secret"));
        assert!(!context.contains("/Users"));
    }

    #[test]
    fn wire_replays_cannot_restart_jobs_but_queries_repeat_with_bounded_history() {
        let mut history = std::collections::VecDeque::new();
        let mutation = AppToolRequest::StorageScan {};
        record_mutating_request(&mut history, "one", &mutation).unwrap();
        assert!(record_mutating_request(&mut history, "one", &mutation).is_err());
        let query = AppToolRequest::StorageOverview {};
        for _ in 0..3 {
            record_mutating_request(&mut history, "query", &query).unwrap();
        }
        assert_eq!(history.len(), 1);
        for index in 0..200 {
            record_mutating_request(&mut history, &format!("r{index}"), &mutation).unwrap();
        }
        assert_eq!(history.len(), 128);
    }

    #[test]
    fn discovery_and_every_detail_survive_the_same_model_result_budget() {
        let catalog = bloomsweepy_control::discovery_index();
        let mut discovery = AppToolResult::completed(
            "capabilities",
            json!({
                "catalog":catalog,"platform":"macos","externalInspectionAllowed":false,
            }),
        );
        bound_model_result(&mut discovery).unwrap();
        assert!(!discovery.truncated);
        let items = discovery.data["catalog"]["capabilities"]
            .as_array()
            .unwrap();
        assert!(!items.is_empty());
        for item in items {
            let mut detail = AppToolResult::completed(
                "capabilities.details",
                bloomsweepy_control::capability_details(item["id"].as_str().unwrap()).unwrap(),
            );
            bound_model_result(&mut detail).unwrap();
            assert!(!detail.truncated, "{}", item["id"]);
            assert_eq!(detail.data["capability"]["id"], item["id"]);
        }
        let mut oversized =
            AppToolResult::completed("test", json!({"items":"x".repeat(17 * 1024)}));
        bound_model_result(&mut oversized).unwrap();
        assert!(oversized.truncated);
        assert_eq!(oversized.data["itemsOmitted"], true);
    }

    #[test]
    fn file_workspace_wire_replay_guard_keeps_status_and_pages_repeatable() {
        use bloomsweepy_control::FileWorkspaceAction;
        let mut history = std::collections::VecDeque::new();
        let scan = AppToolRequest::FileWorkspace {
            operation: FileWorkspaceAction::Scan {},
        };
        record_mutating_request(&mut history, "file-scan", &scan).unwrap();
        assert!(record_mutating_request(&mut history, "file-scan", &scan).is_err());
        for _ in 0..3 {
            record_mutating_request(
                &mut history,
                "status",
                &AppToolRequest::FileWorkspace {
                    operation: FileWorkspaceAction::Status {},
                },
            )
            .unwrap();
        }
        assert_eq!(history.len(), 1);
    }
}
