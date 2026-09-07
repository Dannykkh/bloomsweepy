//! Explicit, app-only irreversible action. Never expose this through chat/MCP.
//! Delegate to the OS; do not enumerate Trash or fall back to recursive deletion.
use crate::{ScanCompletionGuard, ScanRuntime, StoredReports};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State, WebviewWindow};

const PLAN_TTL: Duration = Duration::from_secs(120);

#[derive(Default)]
pub(crate) struct EmptyTrashState(Mutex<PlanState>);

#[derive(Default)]
struct PlanState {
    pending: Option<(EmptyTrashPlan, Instant)>,
    // A timed-out native operation might still be running. Never automatically retry.
    unconfirmed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmptyTrashPlan {
    id: String,
    expires_at_unix_ms: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfirmEmptyTrashRequest {
    plan_id: String,
    irreversible_acknowledged: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EmptyTrashError {
    Unavailable,
    Unsupported,
    InvalidPlan,
    Expired,
    AcknowledgmentRequired,
    NeedsInspection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EmptyTrashOutcome {
    Requested,
    Cancelled,
    PermissionDenied,
    LaunchFailed,
    Unconfirmed,
}

impl EmptyTrashState {
    fn prepare(&self) -> Result<EmptyTrashPlan, EmptyTrashError> {
        let mut state = self.0.lock().map_err(|_| EmptyTrashError::Unavailable)?;
        if state.unconfirmed {
            return Err(EmptyTrashError::NeedsInspection);
        }
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| EmptyTrashError::Unavailable)?;
        let plan = EmptyTrashPlan {
            id: random.iter().map(|byte| format!("{byte:02x}")).collect(),
            expires_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
                + PLAN_TTL.as_millis() as u64,
        };
        state.pending = Some((plan.clone(), Instant::now() + PLAN_TTL));
        Ok(plan)
    }

    fn claim(&self, request: &ConfirmEmptyTrashRequest) -> Result<(), EmptyTrashError> {
        let mut state = self.0.lock().map_err(|_| EmptyTrashError::Unavailable)?;
        if state.unconfirmed {
            return Err(EmptyTrashError::NeedsInspection);
        }
        let (plan, deadline) = state.pending.as_ref().ok_or(EmptyTrashError::InvalidPlan)?;
        if plan.id != request.plan_id {
            return Err(EmptyTrashError::InvalidPlan);
        }
        if *deadline <= Instant::now() {
            state.pending = None;
            return Err(EmptyTrashError::Expired);
        }
        if !request.irreversible_acknowledged {
            return Err(EmptyTrashError::AcknowledgmentRequired);
        }
        state.pending = None;
        Ok(())
    }

    fn dismiss(&self, id: &str) -> Result<(), EmptyTrashError> {
        let mut state = self.0.lock().map_err(|_| EmptyTrashError::Unavailable)?;
        if state
            .pending
            .as_ref()
            .is_some_and(|(plan, _)| plan.id == id)
        {
            state.pending = None;
        }
        Ok(())
    }

    fn record(&self, outcome: EmptyTrashOutcome) -> Result<(), EmptyTrashError> {
        let mut state = self.0.lock().map_err(|_| EmptyTrashError::Unavailable)?;
        if outcome == EmptyTrashOutcome::Unconfirmed {
            state.unconfirmed = true;
            state.pending = None;
        }
        Ok(())
    }
}

fn require_main_window(window: &WebviewWindow) -> Result<(), EmptyTrashError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(EmptyTrashError::Unavailable)
    }
}

#[tauri::command]
pub(crate) fn prepare_empty_system_trash(
    window: WebviewWindow,
    plans: State<'_, EmptyTrashState>,
) -> Result<EmptyTrashPlan, EmptyTrashError> {
    require_main_window(&window)?;
    if !cfg!(any(target_os = "macos", windows)) {
        return Err(EmptyTrashError::Unsupported);
    }
    plans.prepare()
}

#[tauri::command]
pub(crate) fn dismiss_empty_system_trash(
    window: WebviewWindow,
    plans: State<'_, EmptyTrashState>,
    plan_id: String,
) -> Result<(), EmptyTrashError> {
    require_main_window(&window)?;
    plans.dismiss(&plan_id)
}

#[tauri::command]
pub(crate) async fn confirm_empty_system_trash(
    window: WebviewWindow,
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    plans: State<'_, EmptyTrashState>,
    request: ConfirmEmptyTrashRequest,
) -> Result<EmptyTrashOutcome, EmptyTrashError> {
    require_main_window(&window)?;
    runtime.begin().map_err(|_| EmptyTrashError::Unavailable)?;
    let _completion = ScanCompletionGuard::new(app.clone());
    plans.claim(&request)?;
    if !runtime
        .begin_commit()
        .map_err(|_| EmptyTrashError::Unavailable)?
    {
        return Ok(EmptyTrashOutcome::Cancelled);
    }
    // Keep the lease inside the worker even if the IPC future is dropped.
    let worker_completion = _completion.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let _completion = worker_completion;
        let outcome = native_empty_trash();
        // Persist the latch and invalidate maps even if the IPC caller goes away.
        let _ = app.state::<EmptyTrashState>().record(outcome);
        let _ = app.state::<StoredReports>().clear_all();
        outcome
    })
    .await
    .unwrap_or(EmptyTrashOutcome::Unconfirmed);
    plans.record(outcome)?;
    Ok(outcome)
}

#[cfg(target_os = "macos")]
const FINDER_SCRIPT: &str = r#"try
    with timeout of 120 seconds
        tell application id "com.apple.finder" to empty trash
    end timeout
    return "requested"
on error errorMessage number errorNumber
    if errorNumber is -128 then return "cancelled"
    if errorNumber is -1743 then return "permissionDenied"
    return "unconfirmed"
end try"#;

#[cfg(target_os = "macos")]
fn native_empty_trash() -> EmptyTrashOutcome {
    use std::io::Read;
    use std::process::{Command, Stdio};

    // Fixed executable and script; never interpolate a path, change Finder warning
    // preferences, ask for elevation, or use a shell/recursive-delete fallback.
    let Ok(mut child) = Command::new("/usr/bin/osascript")
        .args(["-e", FINDER_SCRIPT])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return EmptyTrashOutcome::LaunchFailed;
    };
    let deadline = Instant::now() + Duration::from_secs(130);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let mut output = String::new();
                if let Some(stdout) = child.stdout.take()
                    && stdout.take(64).read_to_string(&mut output).is_ok()
                {
                    return parse_finder_reply(&output);
                }
                return EmptyTrashOutcome::Unconfirmed;
            }
            Ok(Some(_)) => return EmptyTrashOutcome::Unconfirmed,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                // Killing our helper cannot cancel an Apple event already sent.
                return EmptyTrashOutcome::Unconfirmed;
            }
        }
    }
}

#[cfg(any(target_os = "macos", test))]
fn parse_finder_reply(output: &str) -> EmptyTrashOutcome {
    match output.trim() {
        "requested" => EmptyTrashOutcome::Requested,
        "cancelled" => EmptyTrashOutcome::Cancelled,
        "permissionDenied" => EmptyTrashOutcome::PermissionDenied,
        _ => EmptyTrashOutcome::Unconfirmed,
    }
}

#[cfg(windows)]
fn native_empty_trash() -> EmptyTrashOutcome {
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
    };
    use windows_sys::Win32::UI::Shell::SHEmptyRecycleBinW;
    // The worker owns this COM apartment; retain the OS confirmation/progress UI.
    let initialized = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    if initialized < 0 {
        return EmptyTrashOutcome::LaunchFailed;
    }
    let result = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), 0) };
    unsafe { CoUninitialize() };
    match result as u32 {
        0 => EmptyTrashOutcome::Requested,
        0x80004004 | 0x800704C7 => EmptyTrashOutcome::Cancelled,
        0x80070005 => EmptyTrashOutcome::PermissionDenied,
        _ => EmptyTrashOutcome::Unconfirmed,
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn native_empty_trash() -> EmptyTrashOutcome {
    EmptyTrashOutcome::LaunchFailed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(plan: &EmptyTrashPlan) -> ConfirmEmptyTrashRequest {
        ConfirmEmptyTrashRequest {
            plan_id: plan.id.clone(),
            irreversible_acknowledged: true,
        }
    }

    #[test]
    fn requires_explicit_acknowledgment_and_one_shot_plan() {
        let state = EmptyTrashState::default();
        let plan = state.prepare().unwrap();
        let mut request = request(&plan);
        request.irreversible_acknowledged = false;
        assert_eq!(
            state.claim(&request),
            Err(EmptyTrashError::AcknowledgmentRequired)
        );
        request.irreversible_acknowledged = true;
        assert_eq!(state.claim(&request), Ok(()));
        assert_eq!(state.claim(&request), Err(EmptyTrashError::InvalidPlan));
    }

    #[test]
    fn expiry_cancellation_and_replacement_invalidate_plans() {
        let state = EmptyTrashState::default();
        let old = state.prepare().unwrap();
        let new = state.prepare().unwrap();
        assert_ne!(old.id, new.id);
        assert_eq!(
            state.claim(&request(&old)),
            Err(EmptyTrashError::InvalidPlan)
        );
        state.dismiss(&old.id).unwrap();
        assert_eq!(state.claim(&request(&new)), Ok(()));
        let plan = state.prepare().unwrap();
        state.0.lock().unwrap().pending.as_mut().unwrap().1 = Instant::now();
        assert_eq!(state.claim(&request(&plan)), Err(EmptyTrashError::Expired));
        let plan = state.prepare().unwrap();
        state.dismiss(&plan.id).unwrap();
        assert_eq!(
            state.claim(&request(&plan)),
            Err(EmptyTrashError::InvalidPlan)
        );
    }

    #[test]
    fn strict_request_rejects_paths_commands_and_missing_acknowledgment() {
        for json in [
            r#"{"planId":"id"}"#,
            r#"{"planId":"id","irreversibleAcknowledged":true,"path":"/"}"#,
            r#"{"planId":"id","irreversibleAcknowledged":true,"command":"rm"}"#,
        ] {
            assert!(serde_json::from_str::<ConfirmEmptyTrashRequest>(json).is_err());
        }
    }

    #[test]
    fn ambiguous_os_result_prevents_further_requests_this_session() {
        let state = EmptyTrashState::default();
        let plan = state.prepare().unwrap();
        state.record(EmptyTrashOutcome::Unconfirmed).unwrap();
        assert_eq!(
            state.claim(&request(&plan)),
            Err(EmptyTrashError::NeedsInspection)
        );
        assert!(matches!(
            state.prepare(),
            Err(EmptyTrashError::NeedsInspection)
        ));
    }

    #[test]
    fn unknown_output_is_never_reported_as_success() {
        assert_eq!(
            parse_finder_reply("requested\n"),
            EmptyTrashOutcome::Requested
        );
        assert_eq!(
            parse_finder_reply("cancelled"),
            EmptyTrashOutcome::Cancelled
        );
        assert_eq!(
            parse_finder_reply("permissionDenied"),
            EmptyTrashOutcome::PermissionDenied
        );
        for output in ["", "unconfirmed", "error", "requested\nerror"] {
            assert_eq!(parse_finder_reply(output), EmptyTrashOutcome::Unconfirmed);
        }
    }

    #[test]
    fn repeated_submit_cannot_dispatch_a_mock_operation_twice() {
        let state = EmptyTrashState::default();
        let plan = state.prepare().unwrap();
        let mut mock_dispatches = 0;
        for _ in 0..2 {
            if state.claim(&request(&plan)).is_ok() {
                mock_dispatches += 1;
            }
        }
        assert_eq!(mock_dispatches, 1);
        // No test calls native_empty_trash or touches the user's actual Trash.
    }
}
