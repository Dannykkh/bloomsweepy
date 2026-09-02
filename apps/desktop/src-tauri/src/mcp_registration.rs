use crate::external_program::{ExternalProgram, find_external_program};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

const REGISTRATION_NAME: &str = "bloomsweepy";
const RECEIPT_SCHEMA_VERSION: u32 = 1;
const MAX_RECEIPT_BYTES: u64 = 16 * 1024;
const MAX_COMMAND_OUTPUT_BYTES: usize = 128 * 1024;
const CLI_TIMEOUT: Duration = Duration::from_secs(15);
const HELPER_VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const TERMINATION_HELPER_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(crate) struct McpRegistrationRuntime {
    gate: Arc<Mutex<()>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum McpClientKind {
    Codex,
    ClaudeCode,
}

impl McpClientKind {
    fn executable_name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
        }
    }

    fn scope(self) -> &'static str {
        match self {
            Self::Codex => "userConfig",
            Self::ClaudeCode => "user",
        }
    }

    fn receipt_file_name(self) -> &'static str {
        match self {
            Self::Codex => "mcp-registration-codex-v1.json",
            Self::ClaudeCode => "mcp-registration-claude-code-v1.json",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum McpRegistrationState {
    ClientMissing,
    HelperMissing,
    NotRegistered,
    RegisteredManaged,
    RegisteredOther,
    PathStale,
    CheckFailed,
    DebugBuild,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpClientRegistrationStatus {
    client: McpClientKind,
    label: &'static str,
    state: McpRegistrationState,
    detail: String,
    helper_path: String,
    helper_version: Option<String>,
    app_version: String,
    can_register: bool,
    can_unregister: bool,
    restart_required: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RegistrationReceipt {
    schema_version: u32,
    client: McpClientKind,
    name: String,
    scope: String,
    command: String,
    args: Vec<String>,
    app_version: String,
    helper_version: String,
    registered_at_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveRegistration {
    scope: String,
    command: String,
    args: Vec<String>,
}

struct RegistrationEnvironment {
    app_data_dir: PathBuf,
    helper_path: PathBuf,
    helper_state: HelperState,
    app_version: String,
}

enum HelperState {
    Ready(String),
    Missing,
    Invalid,
    VersionMismatch(String),
}

struct ClientInspection {
    program: Option<ExternalProgram>,
    active: Result<Option<ActiveRegistration>, ()>,
    receipt: Result<Option<RegistrationReceipt>, ()>,
}

struct CommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

#[tauri::command]
pub(crate) async fn get_mcp_registration_statuses(
    app: AppHandle,
    runtime: State<'_, McpRegistrationRuntime>,
) -> Result<Vec<McpClientRegistrationStatus>, String> {
    let gate = Arc::clone(&runtime.gate);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .map_err(|_| "AI 연결 상태 잠금을 사용할 수 없습니다".to_owned())?;
        let environment = registration_environment(&app)?;
        Ok([
            status_for_client(&environment, McpClientKind::Codex),
            status_for_client(&environment, McpClientKind::ClaudeCode),
        ]
        .into_iter()
        .collect())
    })
    .await
    .map_err(|error| format!("AI 연결 상태 확인 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn register_mcp_client(
    app: AppHandle,
    runtime: State<'_, McpRegistrationRuntime>,
    client: McpClientKind,
) -> Result<McpClientRegistrationStatus, String> {
    reject_debug_mutation()?;
    let gate = Arc::clone(&runtime.gate);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .map_err(|_| "AI 연결 변경 잠금을 사용할 수 없습니다".to_owned())?;
        register_client_at(&app, client)
    })
    .await
    .map_err(|error| format!("AI 연결 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn unregister_mcp_client(
    app: AppHandle,
    runtime: State<'_, McpRegistrationRuntime>,
    client: McpClientKind,
) -> Result<McpClientRegistrationStatus, String> {
    reject_debug_mutation()?;
    let gate = Arc::clone(&runtime.gate);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .map_err(|_| "AI 연결 변경 잠금을 사용할 수 없습니다".to_owned())?;
        unregister_client_at(&app, client)
    })
    .await
    .map_err(|error| format!("AI 연결 해제 작업이 중단됐습니다: {error}"))?
}

fn reject_debug_mutation() -> Result<(), String> {
    if cfg!(debug_assertions) {
        Err(
            "개발용 앱에서는 사용자 AI 설정을 변경하지 않습니다. 설치한 릴리스 앱에서 연결하세요"
                .to_owned(),
        )
    } else {
        Ok(())
    }
}

fn registration_environment(app: &AppHandle) -> Result<RegistrationEnvironment, String> {
    let app_version = app.package_info().version.to_string();
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("AI 연결 기록 폴더를 찾지 못했습니다: {error}"))?;
    let executable = std::env::current_exe()
        .map_err(|error| format!("앱 실행 위치를 확인하지 못했습니다: {error}"))?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| "앱 실행 폴더를 확인하지 못했습니다".to_owned())?;
    let helper_path = executable_dir.join(helper_file_name());
    let helper_state = inspect_helper(&helper_path, &app_version);
    Ok(RegistrationEnvironment {
        app_data_dir,
        helper_path,
        helper_state,
        app_version,
    })
}

fn helper_file_name() -> &'static str {
    if cfg!(windows) {
        "bloomsweepy-mcp.exe"
    } else {
        "bloomsweepy-mcp"
    }
}

fn inspect_helper(path: &Path, app_version: &str) -> HelperState {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return HelperState::Missing,
        Err(_) => return HelperState::Invalid,
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return HelperState::Invalid;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return HelperState::Invalid;
        }
    }

    let program = ExternalProgram::Direct(path.to_path_buf());
    let arguments = vec!["--version".to_owned()];
    let output = match run_external_program(&program, &arguments, HELPER_VERSION_TIMEOUT) {
        Ok(output) if output.success => output,
        _ => return HelperState::Invalid,
    };
    let Some(version) = parse_helper_version(&output.stdout) else {
        return HelperState::Invalid;
    };
    if version == app_version {
        HelperState::Ready(version)
    } else {
        HelperState::VersionMismatch(version)
    }
}

fn parse_helper_version(output: &str) -> Option<String> {
    let mut fields = output.split_whitespace();
    if fields.next()? != "bloomsweepy-mcp" {
        return None;
    }
    let version = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return None;
    }
    Some(version.to_owned())
}

fn status_for_client(
    environment: &RegistrationEnvironment,
    client: McpClientKind,
) -> McpClientRegistrationStatus {
    let inspection = inspect_client(environment, client);
    status_from_inspection(environment, client, &inspection, false)
}

fn inspect_client(
    environment: &RegistrationEnvironment,
    client: McpClientKind,
) -> ClientInspection {
    let receipt = load_receipt(&receipt_path(environment, client), client);
    let program = find_external_program(client.executable_name());
    let active = match program.as_ref() {
        Some(program) => query_registration(client, program),
        None => Ok(None),
    };
    ClientInspection {
        program,
        active,
        receipt,
    }
}

fn status_from_inspection(
    environment: &RegistrationEnvironment,
    client: McpClientKind,
    inspection: &ClientInspection,
    restart_required: bool,
) -> McpClientRegistrationStatus {
    let helper_version = match &environment.helper_state {
        HelperState::Ready(version) | HelperState::VersionMismatch(version) => {
            Some(version.clone())
        }
        HelperState::Missing | HelperState::Invalid => None,
    };
    let exact_owned = matches!(
        (&inspection.active, &inspection.receipt),
        (Ok(Some(active)), Ok(Some(receipt))) if receipt_matches_active(receipt, active)
    );
    let current_owned = matches!(
        (&inspection.active, &inspection.receipt),
        (Ok(Some(active)), Ok(Some(receipt)))
            if receipt_matches_active(receipt, active)
                && receipt_matches_environment(receipt, environment)
    );

    let (mut state, mut detail) = if matches!(&environment.helper_state, HelperState::Missing) {
        (
            McpRegistrationState::HelperMissing,
            "설치된 MCP 도구를 찾지 못했습니다".to_owned(),
        )
    } else if matches!(&environment.helper_state, HelperState::Invalid) {
        (
            McpRegistrationState::HelperMissing,
            "설치된 MCP 도구를 안전하게 실행할 수 없습니다".to_owned(),
        )
    } else if matches!(&environment.helper_state, HelperState::VersionMismatch(_)) {
        (
            McpRegistrationState::PathStale,
            "앱과 MCP 도구 버전이 달라 다시 설치해야 합니다".to_owned(),
        )
    } else if inspection.program.is_none() {
        (
            McpRegistrationState::ClientMissing,
            format!("{} 명령줄 도구를 찾지 못했습니다", client.label()),
        )
    } else if inspection.receipt.is_err() || inspection.active.is_err() {
        (
            McpRegistrationState::CheckFailed,
            format!(
                "{} 연결 상태를 안전하게 확인하지 못했습니다",
                client.label()
            ),
        )
    } else {
        match (&inspection.active, &inspection.receipt) {
            (Ok(None), _) => (
                McpRegistrationState::NotRegistered,
                format!("{}에 연결되어 있지 않습니다", client.label()),
            ),
            (Ok(Some(_)), Ok(None)) => (
                McpRegistrationState::RegisteredOther,
                "같은 이름의 다른 연결이 있어 자동으로 바꾸지 않습니다".to_owned(),
            ),
            (Ok(Some(_)), Ok(Some(_))) if current_owned => (
                McpRegistrationState::RegisteredManaged,
                format!("{}에 BroomSweepy가 연결되어 있습니다", client.label()),
            ),
            (Ok(Some(_)), Ok(Some(_))) if exact_owned => (
                McpRegistrationState::PathStale,
                "앱 위치나 버전이 달라 연결 복구가 필요합니다".to_owned(),
            ),
            (Ok(Some(_)), Ok(Some(_))) => (
                McpRegistrationState::RegisteredOther,
                "현재 연결이 앱의 연결 기록과 달라 자동으로 바꾸지 않습니다".to_owned(),
            ),
            _ => (
                McpRegistrationState::CheckFailed,
                "연결 상태를 안전하게 확인하지 못했습니다".to_owned(),
            ),
        }
    };

    let release_build = !cfg!(debug_assertions);
    let helper_ready = matches!(&environment.helper_state, HelperState::Ready(_));
    let can_register = release_build
        && helper_ready
        && inspection.program.is_some()
        && inspection.active.is_ok()
        && inspection.receipt.is_ok()
        && (matches!(&inspection.active, Ok(None)) || exact_owned)
        && !current_owned;
    let can_unregister = release_build && inspection.program.is_some() && exact_owned;

    if !release_build {
        state = McpRegistrationState::DebugBuild;
        detail = "개발용 앱은 상태만 확인하며 사용자 AI 설정을 바꾸지 않습니다".to_owned();
    }

    McpClientRegistrationStatus {
        client,
        label: client.label(),
        state,
        detail,
        helper_path: environment.helper_path.to_string_lossy().into_owned(),
        helper_version,
        app_version: environment.app_version.clone(),
        can_register,
        can_unregister,
        restart_required,
    }
}

fn register_client_at(
    app: &AppHandle,
    client: McpClientKind,
) -> Result<McpClientRegistrationStatus, String> {
    let environment = registration_environment(app)?;
    let helper_version = match &environment.helper_state {
        HelperState::Ready(version) => version.clone(),
        HelperState::Missing => return Err("설치된 MCP 도구를 찾지 못했습니다".to_owned()),
        HelperState::Invalid => {
            return Err("설치된 MCP 도구를 안전하게 실행할 수 없습니다".to_owned());
        }
        HelperState::VersionMismatch(_) => {
            return Err("앱과 MCP 도구 버전이 달라 다시 설치해야 합니다".to_owned());
        }
    };
    let helper_command = path_to_command(&environment.helper_path)?;
    let program = find_external_program(client.executable_name())
        .ok_or_else(|| format!("{} 명령줄 도구를 찾지 못했습니다", client.label()))?;
    let receipt_file = receipt_path(&environment, client);
    let receipt = load_receipt(&receipt_file, client)
        .map_err(|_| "앱의 AI 연결 기록을 안전하게 읽지 못했습니다".to_owned())?;
    let active = query_registration(client, &program).map_err(|_| {
        format!(
            "{} 연결 상태를 안전하게 확인하지 못했습니다",
            client.label()
        )
    })?;

    let previous = match (&active, &receipt) {
        (Some(active), Some(receipt)) if receipt_matches_active(receipt, active) => {
            Some(receipt.clone())
        }
        (Some(_), _) => {
            return Err("같은 이름의 다른 연결이 있어 자동으로 바꾸지 않습니다".to_owned());
        }
        (None, _) => None,
    };

    if let Some(previous) = previous.as_ref() {
        if receipt_matches_environment(previous, &environment) {
            let inspection = ClientInspection {
                program: Some(program),
                active: Ok(active),
                receipt: Ok(receipt),
            };
            return Ok(status_from_inspection(
                &environment,
                client,
                &inspection,
                false,
            ));
        }
        remove_and_verify_absent(client, &program)?;
    }

    let next = RegistrationReceipt {
        schema_version: RECEIPT_SCHEMA_VERSION,
        client,
        name: REGISTRATION_NAME.to_owned(),
        scope: client.scope().to_owned(),
        command: helper_command,
        args: vec!["mcp".to_owned()],
        app_version: environment.app_version.clone(),
        helper_version,
        registered_at_unix_ms: unix_time_ms(),
    };

    if let Err(error) = add_registration(client, &program, &next) {
        let rollback_ok = restore_after_failed_add(client, &program, &next, previous.as_ref());
        return Err(if rollback_ok {
            error
        } else {
            "AI 연결을 완료하지 못했고 이전 연결도 복구하지 못했습니다".to_owned()
        });
    }
    match query_registration(client, &program) {
        Ok(Some(active)) if receipt_matches_active(&next, &active) => {}
        _ => {
            let rollback_ok = restore_after_failed_add(client, &program, &next, previous.as_ref());
            return Err(if rollback_ok {
                "AI 연결을 확인하지 못해 변경을 되돌렸습니다".to_owned()
            } else {
                "AI 연결을 확인하지 못했고 이전 연결도 복구하지 못했습니다".to_owned()
            });
        }
    }
    if save_receipt(&receipt_file, &next).is_err() {
        let rollback_ok = restore_after_failed_add(client, &program, &next, previous.as_ref());
        return Err(if rollback_ok {
            "AI 연결 기록을 저장하지 못해 변경을 되돌렸습니다".to_owned()
        } else {
            "AI 연결 기록을 저장하지 못했고 이전 연결도 복구하지 못했습니다".to_owned()
        });
    }

    let inspection = inspect_client(&environment, client);
    Ok(status_from_inspection(
        &environment,
        client,
        &inspection,
        true,
    ))
}

fn restore_after_failed_add(
    client: McpClientKind,
    program: &ExternalProgram,
    attempted: &RegistrationReceipt,
    previous: Option<&RegistrationReceipt>,
) -> bool {
    remove_if_matches(client, program, attempted)
        && rollback_registration(client, program, previous)
}

fn unregister_client_at(
    app: &AppHandle,
    client: McpClientKind,
) -> Result<McpClientRegistrationStatus, String> {
    let environment = registration_environment(app)?;
    let program = find_external_program(client.executable_name())
        .ok_or_else(|| format!("{} 명령줄 도구를 찾지 못했습니다", client.label()))?;
    let receipt_file = receipt_path(&environment, client);
    let receipt = load_receipt(&receipt_file, client)
        .map_err(|_| "앱의 AI 연결 기록을 안전하게 읽지 못했습니다".to_owned())?
        .ok_or_else(|| "이 앱이 만든 연결 기록이 없어 자동으로 해제하지 않습니다".to_owned())?;
    let active = query_registration(client, &program).map_err(|_| {
        format!(
            "{} 연결 상태를 안전하게 확인하지 못했습니다",
            client.label()
        )
    })?;

    let removed_external = match active {
        Some(active) if receipt_matches_active(&receipt, &active) => {
            remove_and_verify_absent(client, &program)?;
            true
        }
        Some(_) => {
            return Err("현재 연결이 앱의 연결 기록과 달라 자동으로 해제하지 않습니다".to_owned());
        }
        None => false,
    };

    if remove_receipt(&receipt_file).is_err() {
        let rollback_ok = !removed_external
            || (add_registration(client, &program, &receipt).is_ok()
                && matches!(
                    query_registration(client, &program),
                    Ok(Some(active)) if receipt_matches_active(&receipt, &active)
                ));
        return Err(if rollback_ok {
            "연결 기록을 지우지 못해 연결 해제를 되돌렸습니다".to_owned()
        } else {
            "연결은 해제됐지만 앱의 연결 기록을 정리하지 못했습니다".to_owned()
        });
    }

    let inspection = inspect_client(&environment, client);
    Ok(status_from_inspection(
        &environment,
        client,
        &inspection,
        true,
    ))
}

fn rollback_registration(
    client: McpClientKind,
    program: &ExternalProgram,
    previous: Option<&RegistrationReceipt>,
) -> bool {
    match previous {
        Some(previous) => {
            add_registration(client, program, previous).is_ok()
                && matches!(
                    query_registration(client, program),
                    Ok(Some(active)) if receipt_matches_active(previous, &active)
                )
        }
        None => matches!(query_registration(client, program), Ok(None)),
    }
}

fn remove_if_matches(
    client: McpClientKind,
    program: &ExternalProgram,
    expected: &RegistrationReceipt,
) -> bool {
    match query_registration(client, program) {
        Ok(Some(active)) if receipt_matches_active(expected, &active) => {
            remove_and_verify_absent(client, program).is_ok()
        }
        Ok(None) => true,
        _ => false,
    }
}

fn add_registration(
    client: McpClientKind,
    program: &ExternalProgram,
    registration: &RegistrationReceipt,
) -> Result<(), String> {
    let arguments = add_arguments(client, registration);
    let output = run_external_program(program, &arguments, CLI_TIMEOUT)
        .map_err(|_| format!("{} 연결 명령을 완료하지 못했습니다", client.label()))?;
    if output.success {
        Ok(())
    } else {
        Err(format!("{} 연결 명령이 실패했습니다", client.label()))
    }
}

fn add_arguments(client: McpClientKind, registration: &RegistrationReceipt) -> Vec<String> {
    match client {
        McpClientKind::Codex => {
            let mut arguments = vec![
                "mcp".to_owned(),
                "add".to_owned(),
                REGISTRATION_NAME.to_owned(),
                "--".to_owned(),
                registration.command.clone(),
            ];
            arguments.extend(registration.args.iter().cloned());
            arguments
        }
        McpClientKind::ClaudeCode => {
            let definition = serde_json::json!({
                "type": "stdio",
                "command": &registration.command,
                "args": &registration.args,
                "env": {},
            });
            vec![
                "mcp".to_owned(),
                "add-json".to_owned(),
                "--scope".to_owned(),
                "user".to_owned(),
                REGISTRATION_NAME.to_owned(),
                definition.to_string(),
            ]
        }
    }
}

fn remove_and_verify_absent(
    client: McpClientKind,
    program: &ExternalProgram,
) -> Result<(), String> {
    let arguments = match client {
        McpClientKind::Codex => vec![
            "mcp".to_owned(),
            "remove".to_owned(),
            REGISTRATION_NAME.to_owned(),
        ],
        McpClientKind::ClaudeCode => vec![
            "mcp".to_owned(),
            "remove".to_owned(),
            "--scope".to_owned(),
            "user".to_owned(),
            REGISTRATION_NAME.to_owned(),
        ],
    };
    let output = run_external_program(program, &arguments, CLI_TIMEOUT)
        .map_err(|_| format!("{} 연결 해제 명령을 완료하지 못했습니다", client.label()))?;
    if !output.success {
        return Err(format!("{} 연결 해제 명령이 실패했습니다", client.label()));
    }
    if matches!(query_registration(client, program), Ok(None)) {
        Ok(())
    } else {
        Err(format!(
            "{} 연결 해제를 확인하지 못했습니다",
            client.label()
        ))
    }
}

fn query_registration(
    client: McpClientKind,
    program: &ExternalProgram,
) -> Result<Option<ActiveRegistration>, ()> {
    let arguments = match client {
        McpClientKind::Codex => vec![
            "mcp".to_owned(),
            "get".to_owned(),
            REGISTRATION_NAME.to_owned(),
            "--json".to_owned(),
        ],
        McpClientKind::ClaudeCode => vec![
            "mcp".to_owned(),
            "get".to_owned(),
            REGISTRATION_NAME.to_owned(),
        ],
    };
    let output = run_external_program(program, &arguments, CLI_TIMEOUT)?;
    if !output.success {
        return if output_indicates_missing(&output) {
            Ok(None)
        } else {
            Err(())
        };
    }
    match client {
        McpClientKind::Codex => parse_codex_registration(&output.stdout).map(Some),
        McpClientKind::ClaudeCode => parse_claude_registration(&output.stdout).map(Some),
    }
}

fn output_indicates_missing(output: &CommandOutput) -> bool {
    let combined = format!("{}\n{}", output.stdout, output.stderr).to_lowercase();
    [
        "no mcp server",
        "mcp server 'bloomsweepy' not found",
        "mcp server \"bloomsweepy\" not found",
        "unknown mcp server 'bloomsweepy'",
        "unknown mcp server \"bloomsweepy\"",
    ]
    .iter()
    .any(|needle| combined.contains(needle))
}

fn parse_codex_registration(output: &str) -> Result<ActiveRegistration, ()> {
    let value: Value = serde_json::from_str(output).map_err(|_| ())?;
    let transport = value.get("transport").unwrap_or(&value);
    if transport.get("type").and_then(Value::as_str) != Some("stdio") {
        return Err(());
    }
    let command = transport.get("command").and_then(Value::as_str).ok_or(())?;
    let args = transport
        .get("args")
        .and_then(Value::as_array)
        .ok_or(())?
        .iter()
        .map(|argument| argument.as_str().map(str::to_owned).ok_or(()))
        .collect::<Result<Vec<_>, _>>()?;
    active_registration("userConfig", command, args)
}

fn parse_claude_registration(output: &str) -> Result<ActiveRegistration, ()> {
    let mut scope = None;
    let mut transport_type = None;
    let mut command = None;
    let mut args = None;
    for line in output.lines() {
        let trimmed = line.trim();
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "scope" => {
                let value = value.trim();
                let normalized = if value.eq_ignore_ascii_case("user")
                    || value.eq_ignore_ascii_case("user config")
                {
                    "user"
                } else if value.eq_ignore_ascii_case("local")
                    || value.eq_ignore_ascii_case("local config")
                {
                    "local"
                } else if value.eq_ignore_ascii_case("project")
                    || value.eq_ignore_ascii_case("project config")
                {
                    "project"
                } else {
                    return Err(());
                };
                scope = Some(normalized.to_owned());
            }
            "type" => transport_type = Some(value.trim().to_ascii_lowercase()),
            "command" => command = Some(trim_outer_quotes(value.trim()).to_owned()),
            "args" => {
                let value = value.trim();
                args = Some(
                    if value.is_empty() || value.eq_ignore_ascii_case("(none)") {
                        Vec::new()
                    } else if value == "mcp" || value == "[\"mcp\"]" {
                        vec!["mcp".to_owned()]
                    } else {
                        return Err(());
                    },
                );
            }
            _ => {}
        }
    }
    if transport_type.as_deref() != Some("stdio") {
        return Err(());
    }
    active_registration(&scope.ok_or(())?, &command.ok_or(())?, args.ok_or(())?)
}

fn trim_outer_quotes(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn active_registration(
    scope: &str,
    command: &str,
    args: Vec<String>,
) -> Result<ActiveRegistration, ()> {
    if scope.is_empty()
        || scope.len() > 64
        || command.is_empty()
        || command.len() > 4096
        || args.len() > 16
        || args.iter().any(|argument| argument.len() > 1024)
    {
        return Err(());
    }
    Ok(ActiveRegistration {
        scope: scope.to_owned(),
        command: command.to_owned(),
        args,
    })
}

fn receipt_matches_active(receipt: &RegistrationReceipt, active: &ActiveRegistration) -> bool {
    receipt.scope == active.scope
        && same_command_path(&receipt.command, &active.command)
        && receipt.args == active.args
}

fn receipt_matches_environment(
    receipt: &RegistrationReceipt,
    environment: &RegistrationEnvironment,
) -> bool {
    let expected = environment.helper_path.to_string_lossy();
    same_command_path(&receipt.command, &expected)
        && receipt.app_version == environment.app_version
        && matches!(
            &environment.helper_state,
            HelperState::Ready(version) if receipt.helper_version == *version
        )
}

fn same_command_path(left: &str, right: &str) -> bool {
    #[cfg(windows)]
    {
        left.replace('/', "\\")
            .eq_ignore_ascii_case(&right.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn path_to_command(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "MCP 도구 경로를 AI 프로그램에 안전하게 전달할 수 없습니다".to_owned())
}

fn receipt_path(environment: &RegistrationEnvironment, client: McpClientKind) -> PathBuf {
    environment.app_data_dir.join(client.receipt_file_name())
}

fn load_receipt(
    path: &Path,
    expected_client: McpClientKind,
) -> Result<Option<RegistrationReceipt>, ()> {
    let backup = receipt_backup_path(path)?;
    let source = match fs::symlink_metadata(path) {
        Ok(_) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match fs::symlink_metadata(&backup) {
                Ok(_) => backup.as_path(),
                Err(backup_error) if backup_error.kind() == io::ErrorKind::NotFound => {
                    return Ok(None);
                }
                Err(_) => return Err(()),
            }
        }
        Err(_) => return Err(()),
    };
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(()),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_RECEIPT_BYTES
    {
        return Err(());
    }
    let mut bytes = Vec::new();
    File::open(source)
        .map_err(|_| ())?
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(());
    }
    let receipt: RegistrationReceipt = serde_json::from_slice(&bytes).map_err(|_| ())?;
    validate_receipt(&receipt, expected_client)?;
    Ok(Some(receipt))
}

fn validate_receipt(
    receipt: &RegistrationReceipt,
    expected_client: McpClientKind,
) -> Result<(), ()> {
    let expected_file = helper_file_name();
    let file_name_matches = Path::new(&receipt.command)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            if cfg!(windows) {
                name.eq_ignore_ascii_case(expected_file)
            } else {
                name == expected_file
            }
        });
    if receipt.schema_version != RECEIPT_SCHEMA_VERSION
        || receipt.client != expected_client
        || receipt.name != REGISTRATION_NAME
        || receipt.scope != expected_client.scope()
        || receipt.args.as_slice() != ["mcp"]
        || !file_name_matches
        || !Path::new(&receipt.command).is_absolute()
        || receipt.command.len() > 4096
        || receipt.app_version.is_empty()
        || receipt.app_version.len() > 64
        || receipt.helper_version.is_empty()
        || receipt.helper_version.len() > 64
    {
        return Err(());
    }
    Ok(())
}

fn save_receipt(path: &Path, receipt: &RegistrationReceipt) -> Result<(), ()> {
    validate_receipt(receipt, receipt.client)?;
    let parent = path.parent().ok_or(())?;
    fs::create_dir_all(parent).map_err(|_| ())?;
    if fs::symlink_metadata(parent)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(true)
    {
        return Err(());
    }
    reject_unsafe_receipt_path(path)?;
    let file_name = path.file_name().and_then(|name| name.to_str()).ok_or(())?;
    let backup = receipt_backup_path(path)?;
    reject_unsafe_receipt_path(&backup)?;
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        unix_time_ms()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let payload = serde_json::to_vec(receipt).map_err(|_| ())?;
    let result: Result<(), ()> = (|| {
        let mut file = options.open(&temporary).map_err(|_| ())?;
        file.write_all(&payload).map_err(|_| ())?;
        file.sync_all().map_err(|_| ())?;
        drop(file);
        let had_main = path.exists();
        if had_main {
            if backup.exists() {
                fs::remove_file(&backup).map_err(|_| ())?;
            }
            fs::rename(path, &backup).map_err(|_| ())?;
        }
        if fs::rename(&temporary, path).is_err() {
            if had_main {
                let _ = fs::rename(&backup, path);
            }
            return Err(());
        }
        if backup.exists() {
            let _ = fs::remove_file(&backup);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn remove_receipt(path: &Path) -> Result<(), ()> {
    let backup = receipt_backup_path(path)?;
    for candidate in [path, backup.as_path()] {
        reject_unsafe_receipt_path(candidate)?;
    }
    for candidate in [backup.as_path(), path] {
        match fs::symlink_metadata(candidate) {
            Ok(_) => fs::remove_file(candidate).map_err(|_| ())?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(()),
        }
    }
    Ok(())
}

fn receipt_backup_path(path: &Path) -> Result<PathBuf, ()> {
    let parent = path.parent().ok_or(())?;
    let file_name = path.file_name().and_then(|name| name.to_str()).ok_or(())?;
    Ok(parent.join(format!(".{file_name}.backup")))
}

fn reject_unsafe_receipt_path(path: &Path) -> Result<(), ()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(()),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(()),
    }
}

fn run_external_program(
    program: &ExternalProgram,
    arguments: &[String],
    timeout: Duration,
) -> Result<CommandOutput, ()> {
    if program.is_command_script()
        && (!safe_command_script_text(program.path().to_string_lossy().as_ref())
            || arguments
                .iter()
                .any(|argument| !safe_command_script_text(argument)))
    {
        return Err(());
    }

    let mut command = program.command();
    configure_external_command(&mut command);
    let mut child = command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ())?;
    let stdout = child.stdout.take().ok_or(())?;
    let stderr = child.stderr.take().ok_or(())?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, MAX_COMMAND_OUTPUT_BYTES));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, MAX_COMMAND_OUTPUT_BYTES));
    let started = Instant::now();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(25)),
            Ok(None) | Err(_) => {
                terminate_process_tree(&mut child);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(());
            }
        }
    };
    let (stdout, stdout_overflow) = stdout_reader.join().map_err(|_| ())?.map_err(|_| ())?;
    let (stderr, stderr_overflow) = stderr_reader.join().map_err(|_| ())?.map_err(|_| ())?;
    if stdout_overflow || stderr_overflow {
        return Err(());
    }
    Ok(CommandOutput {
        success,
        stdout: String::from_utf8(stdout).map_err(|_| ())?,
        stderr: String::from_utf8(stderr).map_err(|_| ())?,
    })
}

fn configure_external_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

fn terminate_process_tree(child: &mut Child) {
    let process_id = child.id();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let process_id = process_id.to_string();
        let mut command = Command::new("taskkill.exe");
        command
            .args(["/PID", process_id.as_str(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        run_termination_helper(&mut command);
    }
    #[cfg(unix)]
    {
        let process_group = format!("-{process_id}");
        let mut command = Command::new("/bin/kill");
        command
            .args(["-KILL", process_group.as_str()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        run_termination_helper(&mut command);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn run_termination_helper(command: &mut Command) {
    let Ok(mut helper) = command.spawn() else {
        return;
    };
    let started = Instant::now();
    loop {
        match helper.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if started.elapsed() < TERMINATION_HELPER_TIMEOUT => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) | Err(_) => {
                let _ = helper.kill();
                let _ = helper.wait();
                return;
            }
        }
    }
}

fn safe_command_script_text(value: &str) -> bool {
    !value.chars().any(|character| {
        matches!(
            character,
            '\r' | '\n' | '&' | '|' | '<' | '>' | '(' | ')' | '^' | '%' | '!' | '"'
        )
    })
}

fn read_bounded(mut reader: impl Read, maximum: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut kept = Vec::new();
    let mut overflow = false;
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let available = maximum.saturating_sub(kept.len());
        let keep = available.min(count);
        kept.extend_from_slice(&buffer[..keep]);
        overflow |= keep < count;
    }
    Ok((kept, overflow))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::tempdir;

    fn receipt(client: McpClientKind, command: &str) -> RegistrationReceipt {
        RegistrationReceipt {
            schema_version: RECEIPT_SCHEMA_VERSION,
            client,
            name: REGISTRATION_NAME.to_owned(),
            scope: client.scope().to_owned(),
            command: command.to_owned(),
            args: vec!["mcp".to_owned()],
            app_version: "1.2.0".to_owned(),
            helper_version: "1.2.0".to_owned(),
            registered_at_unix_ms: 1,
        }
    }

    #[test]
    fn frontend_wire_names_remain_stable() {
        assert_eq!(
            serde_json::to_string(&McpClientKind::ClaudeCode).unwrap(),
            "\"claudeCode\""
        );
        assert_eq!(
            serde_json::to_string(&McpRegistrationState::RegisteredManaged).unwrap(),
            "\"registeredManaged\""
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_builds_refuse_external_configuration_mutations() {
        assert!(reject_debug_mutation().is_err());
    }

    #[test]
    fn codex_json_keeps_only_stdio_command_and_arguments() {
        let active = parse_codex_registration(
            r#"{"name":"bloomsweepy","transport":{"type":"stdio","command":"C:\\Program Files\\BroomSweepy\\bloomsweepy-mcp.exe","args":["mcp"],"env":null}}"#,
        )
        .unwrap();
        assert_eq!(
            active,
            ActiveRegistration {
                scope: "userConfig".to_owned(),
                command: r#"C:\Program Files\BroomSweepy\bloomsweepy-mcp.exe"#.to_owned(),
                args: vec!["mcp".to_owned()],
            }
        );
        assert!(
            parse_codex_registration(
                r#"{"transport":{"type":"http","command":"tool","args":["mcp"]}}"#,
            )
            .is_err()
        );
    }

    #[test]
    fn claude_text_requires_explicit_command_and_known_argument_shape() {
        let active = parse_claude_registration(
            "bloomsweepy:\n  Scope: User config\n  Type: stdio\n  Command: C:\\Program Files\\BroomSweepy\\bloomsweepy-mcp.exe\n  Args: mcp\n",
        )
        .unwrap();
        assert_eq!(active.args, ["mcp"]);
        assert_eq!(active.scope, "user");
        assert!(parse_claude_registration("Status: connected\n").is_err());
        assert!(parse_claude_registration("Command: tool\nArgs: arbitrary --flag\n").is_err());
        assert_eq!(
            parse_claude_registration(
                "Scope: Local config\nType: stdio\nCommand: tool\nArgs: mcp\n"
            )
            .unwrap()
            .scope,
            "local"
        );
    }

    #[test]
    fn receipt_round_trip_is_client_scoped_and_bounded() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("receipt.json");
        let expected = receipt(
            McpClientKind::Codex,
            if cfg!(windows) {
                r#"C:\Program Files\BroomSweepy\bloomsweepy-mcp.exe"#
            } else {
                "/Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-mcp"
            },
        );
        save_receipt(&path, &expected).unwrap();
        assert_eq!(
            load_receipt(&path, McpClientKind::Codex).unwrap(),
            Some(expected)
        );
        assert!(load_receipt(&path, McpClientKind::ClaudeCode).is_err());
    }

    #[test]
    fn ownership_requires_exact_command_and_arguments() {
        let command = if cfg!(windows) {
            r#"C:\Program Files\BroomSweepy\bloomsweepy-mcp.exe"#
        } else {
            "/Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-mcp"
        };
        let receipt = receipt(McpClientKind::Codex, command);
        assert!(receipt_matches_active(
            &receipt,
            &ActiveRegistration {
                scope: "userConfig".to_owned(),
                command: command.to_owned(),
                args: vec!["mcp".to_owned()],
            }
        ));
        assert!(!receipt_matches_active(
            &receipt,
            &ActiveRegistration {
                scope: "userConfig".to_owned(),
                command: command.to_owned(),
                args: vec!["mcp".to_owned(), "unexpected".to_owned()],
            }
        ));
    }

    #[test]
    fn registration_arguments_keep_the_helper_path_as_structured_data() {
        let command = if cfg!(windows) {
            r#"C:\Program Files\BroomSweepy\bloomsweepy-mcp.exe"#
        } else {
            "/Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-mcp"
        };
        let registration = receipt(McpClientKind::Codex, command);
        assert_eq!(
            add_arguments(McpClientKind::Codex, &registration),
            ["mcp", "add", "bloomsweepy", "--", command, "mcp"]
        );

        let claude = add_arguments(McpClientKind::ClaudeCode, &registration);
        assert_eq!(
            &claude[..5],
            ["mcp", "add-json", "--scope", "user", "bloomsweepy"]
        );
        let definition: Value = serde_json::from_str(&claude[5]).unwrap();
        assert_eq!(definition["command"], command);
        assert_eq!(definition["args"], serde_json::json!(["mcp"]));
        assert_eq!(definition["env"], serde_json::json!({}));
    }

    #[test]
    fn receipt_backup_remains_readable_after_an_interrupted_replace() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("receipt.json");
        let backup = receipt_backup_path(&path).unwrap();
        let expected = receipt(
            McpClientKind::ClaudeCode,
            if cfg!(windows) {
                r#"C:\Program Files\BroomSweepy\bloomsweepy-mcp.exe"#
            } else {
                "/Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-mcp"
            },
        );
        fs::write(&backup, serde_json::to_vec(&expected).unwrap()).unwrap();
        assert_eq!(
            load_receipt(&path, McpClientKind::ClaudeCode).unwrap(),
            Some(expected)
        );
    }

    #[test]
    fn bounded_reader_drains_but_never_keeps_more_than_limit() {
        let input = vec![b'x'; 4096];
        let (kept, overflow) = read_bounded(Cursor::new(input), 128).unwrap();
        assert_eq!(kept.len(), 128);
        assert!(overflow);
    }

    #[test]
    fn helper_version_parser_rejects_unbounded_or_ambiguous_output() {
        assert_eq!(
            parse_helper_version("bloomsweepy-mcp 1.2.0\n"),
            Some("1.2.0".to_owned())
        );
        assert!(parse_helper_version("another-program 1.2.0").is_none());
        assert!(parse_helper_version("bloomsweepy-mcp 1.2.0 unexpected").is_none());
        assert!(parse_helper_version("bloomsweepy-mcp version/1.2.0").is_none());
    }

    #[test]
    fn missing_detection_does_not_confuse_a_broken_helper_with_an_absent_registration() {
        let absent = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "Error: MCP server 'bloomsweepy' not found.".to_owned(),
        };
        assert!(output_indicates_missing(&absent));

        let broken_command = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "The configured command was not found.".to_owned(),
        };
        assert!(!output_indicates_missing(&broken_command));
    }

    #[cfg(windows)]
    #[test]
    fn command_scripts_reject_cmd_metacharacters_in_dynamic_arguments() {
        assert!(safe_command_script_text(
            r#"C:\Program Files\BroomSweepy\tool.exe"#
        ));
        assert!(!safe_command_script_text(r#"C:\Unsafe&Path\tool.exe"#));
        assert!(!safe_command_script_text(r#"C:\Unsafe(Path)\tool.exe"#));
        assert!(!safe_command_script_text("%USERPROFILE%\\tool.exe"));
    }

    #[cfg(windows)]
    #[test]
    fn timed_out_process_is_killed_and_waited() {
        let program = ExternalProgram::Direct(PathBuf::from("ping.exe"));
        let arguments = vec!["-n".to_owned(), "6".to_owned(), "127.0.0.1".to_owned()];
        let started = Instant::now();
        assert!(run_external_program(&program, &arguments, Duration::from_millis(50)).is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[test]
    fn timed_out_process_is_killed_and_waited() {
        let program = ExternalProgram::Direct(PathBuf::from("/bin/sleep"));
        let arguments = vec!["2".to_owned()];
        let started = Instant::now();
        assert!(run_external_program(&program, &arguments, Duration::from_millis(50)).is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
