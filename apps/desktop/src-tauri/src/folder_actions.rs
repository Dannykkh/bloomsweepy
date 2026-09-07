//! App-only folder review. No external model or raw path can approve a plan.
use crate::{ScanCompletionGuard, ScanRuntime, StoredReports, trash_actions};
use bloomsweepy_core::{VerifiedTrashItem, validate_directory_trash_folder};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, atomic::Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};

const PLAN_TTL: Duration = Duration::from_secs(300);

#[derive(Default)]
pub(crate) struct FolderActionsState(Mutex<Option<StoredFolderPlan>>);

struct StoredFolderPlan {
    view: FolderReviewPlan,
    deadline: Instant,
    item: VerifiedTrashItem,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FolderReviewPlan {
    id: String,
    generation: u64,
    path: String,
    logical_bytes: u64,
    file_count: u64,
    directory_count: u64,
    expires_at_unix_ms: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrepareFolderRequest {
    generation: u64,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfirmFolderRequest {
    generation: u64,
    plan_id: String,
    nested_contents_acknowledged: bool,
}

impl FolderActionsState {
    fn replace(
        &self,
        generation: u64,
        item: VerifiedTrashItem,
    ) -> Result<FolderReviewPlan, String> {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random)
            .map_err(|_| "폴더 확인 번호를 만들지 못했습니다".to_owned())?;
        let (file_count, directory_count) = item
            .directory_counts()
            .ok_or_else(|| "폴더 확인 결과가 아닙니다".to_owned())?;
        let view = FolderReviewPlan {
            id: random.iter().map(|byte| format!("{byte:02x}")).collect(),
            generation,
            path: item.path().to_string_lossy().into_owned(),
            logical_bytes: item.logical_bytes(),
            file_count,
            directory_count,
            expires_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
                + PLAN_TTL.as_millis() as u64,
        };
        *self
            .0
            .lock()
            .map_err(|_| "폴더 확인 상태를 잠그지 못했습니다".to_owned())? =
            Some(StoredFolderPlan {
                view: view.clone(),
                deadline: Instant::now() + PLAN_TTL,
                item,
            });
        Ok(view)
    }

    fn claim(&self, request: &ConfirmFolderRequest) -> Result<VerifiedTrashItem, String> {
        let mut stored = self
            .0
            .lock()
            .map_err(|_| "폴더 확인 상태를 잠그지 못했습니다".to_owned())?;
        let plan = stored.as_ref().ok_or_else(|| {
            "폴더 확인 계획이 없거나 이미 사용했습니다. 다시 검토하세요".to_owned()
        })?;
        if plan.view.id != request.plan_id || plan.view.generation != request.generation {
            return Err("폴더 확인 대상이 변경됐습니다. 다시 검토하세요".to_owned());
        }
        if plan.deadline <= Instant::now() {
            *stored = None;
            return Err("폴더 확인 시간이 만료됐습니다. 다시 검토하세요".to_owned());
        }
        if !request.nested_contents_acknowledged {
            return Err("하위 항목 전체 이동 확인이 필요합니다".to_owned());
        }
        // Claim under the lock before any I/O; a duplicate submit cannot move twice.
        Ok(stored.take().expect("validated plan exists").item)
    }

    fn clear(&self, plan_id: Option<&str>) -> Result<(), String> {
        let mut stored = self
            .0
            .lock()
            .map_err(|_| "폴더 확인 상태를 잠그지 못했습니다".to_owned())?;
        if plan_id.is_none()
            || stored
                .as_ref()
                .is_some_and(|plan| Some(plan.view.id.as_str()) == plan_id)
        {
            *stored = None;
        }
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn prepare_directory_folder_plan(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
    plans: State<'_, FolderActionsState>,
    request: PrepareFolderRequest,
) -> Result<FolderReviewPlan, String> {
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app);
    plans.clear(None)?;
    let report = reports.directory_report(request.generation)?;
    let worker_cancellation = cancellation.clone();
    let item = tauri::async_runtime::spawn_blocking(move || {
        validate_directory_trash_folder(&report, &request.path, || {
            worker_cancellation.load(Ordering::Acquire)
        })
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("폴더 검토를 완료하지 못했습니다: {error}"))??;
    if cancellation.load(Ordering::Acquire) {
        return Err("폴더 검토가 취소되었습니다".to_owned());
    }
    // Replaced/cleared maps cannot publish an actionable result.
    reports.directory_report(request.generation)?;
    plans.replace(request.generation, item)
}

#[tauri::command]
pub(crate) fn dismiss_directory_folder_plan(
    plans: State<'_, FolderActionsState>,
    plan_id: String,
) -> Result<(), String> {
    plans.clear(Some(&plan_id))
}

#[tauri::command]
pub(crate) async fn confirm_directory_folder_plan(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
    plans: State<'_, FolderActionsState>,
    request: ConfirmFolderRequest,
) -> Result<trash_actions::TrashOperationResult, String> {
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    reports.directory_report(request.generation)?;
    let item = plans.claim(&request)?;
    let result = trash_actions::trash_verified_directory(app, item, cancellation).await;
    reports.clear_all()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_fixture(
        state: &FolderActionsState,
        generation: u64,
    ) -> (tempfile::TempDir, FolderReviewPlan) {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("download")).unwrap();
        std::fs::write(temp.path().join("download/file"), b"fixture").unwrap();
        let report = bloomsweepy_core::scan_directory_level(
            temp.path(),
            Default::default(),
            |_| {},
            || false,
        )
        .unwrap();
        let item =
            validate_directory_trash_folder(&report, &report.children[0].path, || false).unwrap();
        let view = state.replace(generation, item).unwrap();
        (temp, view)
    }

    fn request(plan: &FolderReviewPlan) -> ConfirmFolderRequest {
        ConfirmFolderRequest {
            generation: plan.generation,
            plan_id: plan.id.clone(),
            nested_contents_acknowledged: true,
        }
    }

    #[test]
    fn folder_plan_requires_exact_selection_acknowledgment_and_one_shot_claim() {
        let state = FolderActionsState::default();
        let (_temp, plan) = plan_fixture(&state, 7);
        let mut request = request(&plan);
        request.nested_contents_acknowledged = false;
        assert!(state.claim(&request).is_err());
        request.nested_contents_acknowledged = true;
        request.generation = 8;
        assert!(state.claim(&request).is_err());
        request.generation = 7;
        request.plan_id = "foreign-plan".into();
        assert!(state.claim(&request).is_err());
        request.plan_id = plan.id;
        assert!(state.claim(&request).is_ok());
        assert!(state.claim(&request).is_err());
    }

    #[test]
    fn folder_plan_expires_replaces_dismisses_and_is_not_restored() {
        let state = FolderActionsState::default();
        let (_temp, first) = plan_fixture(&state, 1);
        let (_other, second) = plan_fixture(&state, 2);
        assert!(state.claim(&request(&first)).is_err());
        state.clear(Some(&first.id)).unwrap();
        assert!(state.0.lock().unwrap().is_some());
        state.0.lock().unwrap().as_mut().unwrap().deadline = Instant::now();
        assert!(state.claim(&request(&second)).is_err());
        assert!(
            FolderActionsState::default()
                .claim(&request(&second))
                .is_err()
        );
        let (_last, last) = plan_fixture(&state, 3);
        state.clear(Some(&last.id)).unwrap();
        assert!(state.claim(&request(&last)).is_err());
    }

    #[test]
    fn folder_plan_concurrent_confirmation_claims_exactly_once() {
        let state = std::sync::Arc::new(FolderActionsState::default());
        let (_temp, plan) = plan_fixture(&state, 1);
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let state = state.clone();
                    let request = request(&plan);
                    scope.spawn(move || state.claim(&request).is_ok())
                })
                .collect();
            assert_eq!(
                handles
                    .into_iter()
                    .filter_map(|handle| handle.join().ok())
                    .filter(|claimed| *claimed)
                    .count(),
                1
            );
        });
    }
}
