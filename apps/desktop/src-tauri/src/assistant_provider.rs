use crate::external_program::{ExternalProgram, find_external_programs};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

const MAX_MESSAGE_CHARS: usize = 2_000;
const MAX_HISTORY_TURNS: usize = 20;
const MAX_HISTORY_CHARS: usize = 24_000;
const MAX_CHILDREN: usize = 24;
const MAX_NAME_CHARS: usize = 240;
const MAX_MODEL_NAME_CHARS: usize = 160;
const MAX_PROVIDER_MODELS: usize = 64;
const MAX_STATUS_OUTPUT_BYTES: u64 = 64 * 1024;
const MAX_PROVIDER_OUTPUT_BYTES: u64 = 64 * 1024;
const MAX_PROVIDER_ERROR_BYTES: u64 = 1024 * 1024;
const DEFAULT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(120);
const OLLAMA_PROVIDER_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Default)]
pub(crate) struct AssistantProviderState {
    running: AtomicBool,
    cancellation: std::sync::Arc<AtomicBool>,
    next_request_id: AtomicU64,
}

struct ProviderLease<'a> {
    state: &'a AssistantProviderState,
}

impl Drop for ProviderLease<'_> {
    fn drop(&mut self) {
        self.state.running.store(false, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantProviderStatus {
    provider: AssistantProviderKind,
    label: &'static str,
    installed: bool,
    authentication: AssistantAuthentication,
    available: bool,
    busy: bool,
    detail: String,
    models: Vec<AssistantProviderModel>,
    state: AssistantCliState,
    executable_path: Option<String>,
    version: Option<String>,
    #[serde(skip)]
    passed_launch_checks: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AssistantCliState {
    NotInstalled,
    Broken,
    Incompatible,
    LoginRequired,
    CheckFailed,
    ServiceUnavailable,
    NoModels,
    Ready,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AssistantProviderKind {
    Codex,
    ClaudeCode,
    Grok,
    Antigravity,
    Ollama,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AssistantScopeKind {
    Folder,
    Docker,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub(crate) enum AssistantResponseLanguage {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl AssistantResponseLanguage {
    fn prompt_instruction(self) -> &'static str {
        match self {
            Self::English => "Reply in concise English.",
            Self::Korean => "Reply in concise Korean.",
            Self::Japanese => "Reply in concise, natural Japanese.",
            Self::SimplifiedChinese => "Reply in concise Simplified Chinese.",
        }
    }
}

impl AssistantProviderKind {
    fn executable_name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude",
            Self::Grok => "grok",
            Self::Antigravity => "agy",
            Self::Ollama => "ollama",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Grok => "Grok",
            Self::Antigravity => "Antigravity",
            Self::Ollama => "Ollama",
        }
    }

    fn response_timeout(self) -> Duration {
        match self {
            Self::Ollama => OLLAMA_PROVIDER_TIMEOUT,
            _ => DEFAULT_PROVIDER_TIMEOUT,
        }
    }

    fn response_timeout_label(self) -> &'static str {
        match self {
            Self::Ollama => "10분",
            _ => "2분",
        }
    }
}

const ASSISTANT_PROVIDERS: [AssistantProviderKind; 5] = [
    AssistantProviderKind::Codex,
    AssistantProviderKind::ClaudeCode,
    AssistantProviderKind::Grok,
    AssistantProviderKind::Antigravity,
    AssistantProviderKind::Ollama,
];

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AssistantAuthentication {
    Unknown,
    Authenticated,
    Required,
    NotRequired,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantProviderModel {
    id: String,
    label: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantChatRequest {
    #[serde(default)]
    session_id: Option<String>,
    provider: AssistantProviderKind,
    model: Option<String>,
    message: String,
    history: Vec<AssistantChatTurn>,
    summary: AssistantFolderSummary,
    scope_kind: AssistantScopeKind,
    include_docker_status: bool,
    response_language: AssistantResponseLanguage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantChatTurn {
    pub(crate) role: AssistantChatRole,
    pub(crate) content: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AssistantChatRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantFolderSummary {
    pub(crate) scope_name: String,
    pub(crate) completed_at_unix_ms: u64,
    pub(crate) total_logical_bytes: u64,
    pub(crate) total_files: u64,
    pub(crate) total_directories: u64,
    pub(crate) unreadable_entries: u64,
    pub(crate) empty_directory_count: u64,
    pub(crate) children_truncated: bool,
    pub(crate) children: Vec<AssistantFolderChild>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantFolderChild {
    pub(crate) name: String,
    pub(crate) kind: AssistantFolderChildKind,
    pub(crate) logical_bytes: u64,
    pub(crate) file_count: u64,
    pub(crate) directory_count: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AssistantFolderChildKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantChatResponse {
    provider: AssistantProviderKind,
    label: &'static str,
    model: Option<String>,
    message: String,
    docker_context: Option<super::docker_tools::DockerAssistantContext>,
    empty_workspace: Option<super::assistant_tools::EmptyWorkspaceView>,
    tool_action: Option<&'static str>,
}

#[tauri::command]
pub(crate) async fn get_assistant_provider_status(
    app: AppHandle,
) -> Result<Vec<AssistantProviderStatus>, String> {
    let busy = app
        .try_state::<AssistantProviderState>()
        .ok_or_else(|| "AI CLI 상태 저장소를 찾지 못했습니다".to_owned())?
        .running
        .load(Ordering::Acquire);
    let statuses = tauri::async_runtime::spawn_blocking(move || {
        let checks = ASSISTANT_PROVIDERS
            .map(|provider| thread::spawn(move || provider_status(provider, busy)));
        ASSISTANT_PROVIDERS
            .into_iter()
            .zip(checks)
            .map(|(provider, check)| {
                check
                    .join()
                    .unwrap_or_else(|_| provider_status_failed(provider, busy))
            })
            .collect()
    })
    .await
    .map_err(|error| format!("AI CLI 상태 확인 작업이 중단됐습니다: {error}"))?;
    Ok(statuses)
}

#[tauri::command]
pub(crate) fn cancel_assistant(state: State<'_, AssistantProviderState>) -> bool {
    if !state.running.load(Ordering::Acquire) {
        return false;
    }
    state.cancellation.store(true, Ordering::Release);
    true
}

pub(crate) fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<AssistantProviderState>() {
        state.cancellation.store(true, Ordering::Release);
    }
}

#[tauri::command]
pub(crate) async fn ask_assistant(
    app: AppHandle,
    state: State<'_, AssistantProviderState>,
    request: AssistantChatRequest,
) -> Result<AssistantChatResponse, AssistantChatError> {
    ask_assistant_inner(app, state, request)
        .await
        .map_err(AssistantChatError::from)
}

const REAUTHENTICATION_PREFIX: &str = "reauth-required:";

#[derive(Debug, Serialize)]
pub(crate) struct AssistantChatError {
    kind: &'static str,
    message: String,
}

impl From<String> for AssistantChatError {
    fn from(message: String) -> Self {
        match message.strip_prefix(REAUTHENTICATION_PREFIX) {
            Some(detail) => Self {
                kind: "authentication",
                message: detail.to_owned(),
            },
            None => Self {
                kind: "other",
                message,
            },
        }
    }
}

async fn ask_assistant_inner(
    app: AppHandle,
    state: State<'_, AssistantProviderState>,
    mut request: AssistantChatRequest,
) -> Result<AssistantChatResponse, String> {
    if let Some(session_id) = &request.session_id {
        let session =
            super::assistant_sessions::get_assistant_session(app.clone(), session_id.clone())
                .await?;
        if session.session.scope_kind != request.scope_kind {
            return Err("대화 범위가 변경되었습니다".to_owned());
        }
        request.summary = session.folder_summary;
    }
    validate_request(&request)?;
    state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "이미 AI 응답을 기다리고 있습니다".to_owned())?;
    let _lease = ProviderLease { state: &state };
    state.cancellation.store(false, Ordering::Release);

    let provider = request.provider;
    // Revalidate the very same candidate-selection policy used by the status UI.
    // Never execute the first PATH hit without health and capability checks.
    let preflight_cancellation = std::sync::Arc::clone(&state.cancellation);
    let (status, program) = tauri::async_runtime::spawn_blocking(move || {
        resolve_candidates_cancellable(
            provider,
            true,
            find_external_programs(provider.executable_name()),
            Some(&preflight_cancellation),
        )
    })
    .await
    .map_err(|_| "AI CLI 실행 준비를 확인하지 못했습니다".to_owned())?;
    if state.cancellation.load(Ordering::Acquire) {
        return Err(format!("{} 응답을 취소했습니다", provider.label()));
    }
    if !status.available {
        return Err(status.detail);
    }
    let program = program.ok_or_else(|| "AI CLI 실행 경로를 확인하지 못했습니다".to_owned())?;
    let request_id = state.next_request_id.fetch_add(1, Ordering::AcqRel);
    let workspace = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("대화 작업 폴더를 찾지 못했습니다: {error}"))?
        .join("assistant-workspace");
    let docker_context = if request.include_docker_status {
        Some(super::docker_tools::assistant_context(&app).await?)
    } else {
        None
    };
    let docker_context_json = docker_context
        .as_ref()
        .map(serde_json::to_string_pretty)
        .transpose()
        .map_err(|error| format!("Docker 사용량 요약을 준비하지 못했습니다: {error}"))?;
    let mut prompt = build_prompt(&request, docker_context_json.as_deref())?;
    let tool_session = request
        .session_id
        .clone()
        .filter(|_| request.scope_kind == AssistantScopeKind::Folder);
    if let Some(session_id) = &tool_session {
        let context = app
            .state::<super::assistant_tools::AssistantToolsState>()
            .prompt_context(session_id)?;
        prompt.push_str(super::assistant_tools::TOOL_CONTRACT);
        prompt.push_str("\n[Current app tool state]\n");
        prompt.push_str(&context);
    }
    let cancellation = std::sync::Arc::clone(&state.cancellation);
    let response_model = request.model.clone();
    let run_model = response_model.clone();

    let raw_message = tauri::async_runtime::spawn_blocking(move || {
        run_provider(
            provider,
            program,
            run_model,
            workspace,
            request_id,
            prompt,
            cancellation,
        )
    })
    .await
    .map_err(|error| format!("{} 실행 작업이 중단됐습니다: {error}", provider.label()))??;
    let mut message = raw_message;
    let mut empty_workspace = None;
    let mut tool_action = None;
    if let Some(session_id) = tool_session {
        let envelope = super::assistant_tools::parse_envelope(&message)?;
        message = envelope.message;
        if let Some(action) = envelope.action {
            if state.cancellation.load(Ordering::Acquire) {
                return Err("대화 작업이 취소되었습니다".to_owned());
            }
            tool_action = Some(match &action {
                super::assistant_tools::AssistantAction::ScanEmptyDirectories {} => "scan",
                super::assistant_tools::AssistantAction::ListEmptyDirectories { .. } => "list",
                super::assistant_tools::AssistantAction::UpdateEmptySelection { .. } => "selection",
            });
            empty_workspace = Some(
                super::assistant_tools::dispatch(
                    app.clone(),
                    session_id,
                    action,
                    std::sync::Arc::clone(&state.cancellation),
                )
                .await?,
            );
            // The UI renders a localized factual app result, never an unverified model success claim.
            message =
                "앱에서 후보 검토 상태를 갱신했습니다. 아직 휴지통으로 이동한 항목은 없습니다."
                    .to_owned();
        }
    }
    Ok(AssistantChatResponse {
        provider,
        label: provider.label(),
        model: response_model,
        message,
        docker_context,
        empty_workspace,
        tool_action,
    })
}

fn validate_request(request: &AssistantChatRequest) -> Result<(), String> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err("질문을 입력해 주세요".to_owned());
    }
    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(format!("질문은 {MAX_MESSAGE_CHARS}자 이하여야 합니다"));
    }
    if request.history.len() > MAX_HISTORY_TURNS {
        return Err(format!(
            "최근 대화는 {MAX_HISTORY_TURNS}개까지만 보낼 수 있습니다"
        ));
    }
    let history_chars = request
        .history
        .iter()
        .map(|turn| turn.content.chars().count())
        .sum::<usize>();
    if history_chars > MAX_HISTORY_CHARS {
        return Err("최근 대화가 너무 깁니다. 새 대화를 시작해 주세요".to_owned());
    }
    if request.summary.scope_name.trim().is_empty()
        || request.summary.scope_name.chars().count() > MAX_NAME_CHARS
    {
        return Err("선택한 폴더 이름이 올바르지 않습니다".to_owned());
    }
    if request.summary.children.len() > MAX_CHILDREN {
        return Err(format!("폴더 요약은 {MAX_CHILDREN}개 항목 이하여야 합니다"));
    }
    if request
        .summary
        .children
        .iter()
        .any(|child| child.name.trim().is_empty() || child.name.chars().count() > MAX_NAME_CHARS)
    {
        return Err("폴더 항목 이름이 올바르지 않습니다".to_owned());
    }
    match request.scope_kind {
        AssistantScopeKind::Folder => {}
        AssistantScopeKind::Docker => {
            if !request.include_docker_status
                || request.summary.scope_name != "Docker"
                || !request.summary.children.is_empty()
            {
                return Err("Docker 대화 범위가 올바르지 않습니다".to_owned());
            }
        }
    }
    match (request.provider, request.model.as_deref()) {
        (AssistantProviderKind::Ollama, Some(model))
            if !model.trim().is_empty()
                && model.chars().count() <= MAX_MODEL_NAME_CHARS
                && !model.chars().any(char::is_control) => {}
        (AssistantProviderKind::Ollama, _) => {
            return Err("Ollama에서 사용할 모델을 선택해 주세요".to_owned());
        }
        (_, Some(_)) => {
            return Err("선택한 AI CLI에는 별도 모델 값을 보낼 수 없습니다".to_owned());
        }
        (_, None) => {}
    }
    Ok(())
}

fn build_prompt(
    request: &AssistantChatRequest,
    docker_context_json: Option<&str>,
) -> Result<String, String> {
    let summary = serde_json::to_string_pretty(&request.summary)
        .map_err(|error| format!("대화 범위 요약을 준비하지 못했습니다: {error}"))?;
    let history = request
        .history
        .iter()
        .map(|turn| {
            let role = match turn.role {
                AssistantChatRole::User => "User",
                AssistantChatRole::Assistant => "Assistant",
            };
            format!("{role}: {}", turn.content.trim())
        })
        .collect::<Vec<_>>()
        .join("\n");

    let docker_context = docker_context_json.map_or_else(
        || "[Docker usage]\nNot requested".to_owned(),
        |context| {
            format!(
                "[Docker usage]\n{context}\n\
                 This is a limited summary read by BroomSweepy through Docker CLI. If asked to clean Docker, explain only the category and reason, then direct the user to the app's Docker cleanup review for final confirmation. The app shows category-level estimates and fixed actions, not individual Docker object lists. Do not provide commands or claim that you executed anything."
            )
        },
    );

    let scope_context = match request.scope_kind {
        AssistantScopeKind::Folder => format!(
            "The JSON below is a limited summary produced by BroomSweepy after a read-only scan of the folder selected by the user.\n\
             This app-generated summary omits full paths and file contents; the user may still type these in questions or history.\n\n\
             [Folder summary]\n{summary}"
        ),
        AssistantScopeKind::Docker => {
            "The subject of this chat is Docker on this computer, not a folder. Do not claim that you selected a folder or read files directly.\n\
             Use only the category-level Docker summary supplied by BroomSweepy."
                .to_owned()
        }
    };

    Ok(format!(
        "You are BroomSweepy's storage analysis assistant. {}\n\
         {scope_context}\n\
         You did not read the disk directly. Do not use a shell or any other tool. Do not guess facts absent from the summary; say that an additional scan is required.\n\
         Never claim that deletion was approved or performed. For possible cleanup candidates, explain the reason and suggest only the next review action inside the app.\n\
         The response is displayed as plain text. Do not use Markdown emphasis, headings, code fences, backticks, or metadata tags. Use short sentences and hyphen lists only.\n\n\
         {docker_context}\n\n\
         [Recent conversation]\n{history}\n\n\
         [User question]\n{}",
        request.response_language.prompt_instruction(),
        request.message.trim()
    ))
}

fn run_provider(
    provider: AssistantProviderKind,
    program: ExternalProgram,
    model: Option<String>,
    workspace: PathBuf,
    request_id: u64,
    prompt: String,
    cancellation: std::sync::Arc<AtomicBool>,
) -> Result<String, String> {
    fs::create_dir_all(&workspace)
        .map_err(|error| format!("대화 작업 폴더를 만들지 못했습니다: {error}"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let base = format!("response-{}-{nonce}-{request_id}", std::process::id());
    let response_path = workspace.join(format!("{base}.txt"));
    let error_path = workspace.join(format!("{base}.log"));
    let error_file = File::create(&error_path).map_err(|error| {
        format!(
            "{} 오류 기록을 준비하지 못했습니다: {error}",
            provider.label()
        )
    })?;

    let selected_ollama_model = if provider == AssistantProviderKind::Ollama {
        let requested = model
            .as_deref()
            .ok_or_else(|| "Ollama에서 사용할 모델을 선택해 주세요".to_owned())?;
        let installed_models = ollama_models(&program)?;
        Some(
            installed_models
                .iter()
                .find(|candidate| candidate.id == requested)
                .map(|candidate| candidate.id.clone())
                .ok_or_else(|| "선택한 Ollama 모델이 현재 로컬 목록에 없습니다".to_owned())?,
        )
    } else {
        None
    };

    let mut command = program.command();
    let mut prompt_via_stdin = true;
    match provider {
        AssistantProviderKind::Codex => {
            command
                .arg("exec")
                .arg("--ephemeral")
                .arg("--ignore-user-config")
                .arg("--ignore-rules")
                .arg("--skip-git-repo-check")
                .arg("--sandbox")
                .arg("read-only")
                .arg("--config")
                .arg("approval_policy=\"never\"")
                .arg("--color")
                .arg("never")
                .arg("--cd")
                .arg(&workspace)
                .arg("--output-last-message")
                .arg(&response_path)
                .arg("-")
                .stdout(Stdio::null());
        }
        AssistantProviderKind::ClaudeCode => {
            let response_file = File::create(&response_path)
                .map_err(|error| format!("Claude Code 응답 파일을 준비하지 못했습니다: {error}"))?;
            configure_claude_chat(&mut command);
            command.stdout(Stdio::from(response_file));
        }
        AssistantProviderKind::Grok => {
            let response_file = File::create(&response_path)
                .map_err(|error| format!("Grok 응답 파일을 준비하지 못했습니다: {error}"))?;
            command
                .arg("--single")
                .arg(&prompt)
                .arg("--permission-mode")
                .arg("dontAsk")
                .arg("--tools")
                .arg("")
                .arg("--no-subagents")
                .arg("--disable-web-search")
                .arg("--cwd")
                .arg(&workspace)
                .arg("--output-format")
                .arg("plain")
                .stdout(Stdio::from(response_file));
            prompt_via_stdin = false;
        }
        AssistantProviderKind::Antigravity => {
            let response_file = File::create(&response_path)
                .map_err(|error| format!("Antigravity 응답 파일을 준비하지 못했습니다: {error}"))?;
            command
                .arg("--print")
                .arg(&prompt)
                .arg("--sandbox")
                .stdout(Stdio::from(response_file));
            prompt_via_stdin = false;
        }
        AssistantProviderKind::Ollama => {
            let response_file = File::create(&response_path)
                .map_err(|error| format!("Ollama 응답 파일을 준비하지 못했습니다: {error}"))?;
            command
                .arg("run")
                .arg(
                    selected_ollama_model
                        .as_deref()
                        .expect("validated Ollama model"),
                )
                .arg("--hidethinking")
                .arg("--nowordwrap")
                .env("OLLAMA_NOHISTORY", "1")
                .stdout(Stdio::from(response_file));
        }
    }
    command
        .current_dir(&workspace)
        .stdin(if prompt_via_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::from(error_file));

    let mut child = command
        .spawn()
        .map_err(|error| format!("{}를 시작하지 못했습니다: {error}", provider.label()))?;
    if prompt_via_stdin {
        let write_result = child
            .stdin
            .take()
            .ok_or_else(|| format!("{} 입력 연결을 열지 못했습니다", provider.label()))?
            .write_all(prompt.as_bytes());
        if let Err(error) = write_result {
            let _ = child.kill();
            let _ = child.wait();
            let _ = remove_private_file(&response_path);
            let _ = remove_private_file(&error_path);
            return Err(format!(
                "{}에 질문을 전달하지 못했습니다: {error}",
                provider.label()
            ));
        }
    }

    let started = Instant::now();
    let response_timeout = provider.response_timeout();
    let status = loop {
        if cancellation.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = remove_private_file(&response_path);
            let _ = remove_private_file(&error_path);
            return Err(format!("{} 응답을 취소했습니다", provider.label()));
        }
        if response_path
            .metadata()
            .is_ok_and(|metadata| metadata.len() > MAX_PROVIDER_OUTPUT_BYTES)
            || error_path
                .metadata()
                .is_ok_and(|metadata| metadata.len() > MAX_PROVIDER_ERROR_BYTES)
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = remove_private_file(&response_path);
            let _ = remove_private_file(&error_path);
            return Err("AI 응답이 안전한 표시 한도를 넘었습니다".to_owned());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < response_timeout => {
                thread::sleep(Duration::from_millis(100));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = remove_private_file(&response_path);
                let _ = remove_private_file(&error_path);
                return Err(format!(
                    "{} 응답 시간이 {}을 넘었습니다. 더 작은 모델로 다시 시도하거나 응답을 취소해 주세요",
                    provider.label(),
                    provider.response_timeout_label(),
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = remove_private_file(&response_path);
                let _ = remove_private_file(&error_path);
                return Err(format!(
                    "{} 실행 상태를 확인하지 못했습니다: {error}",
                    provider.label()
                ));
            }
        }
    };

    let response = read_bounded_text(&response_path);
    let provider_error = read_bounded_error_tail(&error_path).unwrap_or_default();
    let _ = remove_private_file(&response_path);
    let _ = remove_private_file(&error_path);

    if !status.success() {
        // Server errors can arrive on stdout. Classify both bounded streams,
        // but never return raw provider output or account details on failure.
        return Err(provider_failure_message(
            provider,
            &format!(
                "{provider_error}\n{}",
                response.as_deref().unwrap_or_default()
            ),
        ));
    }
    let response = response?.trim().to_owned();
    if response.is_empty() {
        return Err(format!("{}가 빈 응답을 반환했습니다", provider.label()));
    }
    Ok(response)
}

fn read_bounded_text(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > MAX_PROVIDER_OUTPUT_BYTES {
        return Err("AI 응답이 안전한 표시 한도를 넘었습니다".to_owned());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PROVIDER_OUTPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_PROVIDER_OUTPUT_BYTES {
        return Err("AI 응답이 안전한 표시 한도를 넘었습니다".to_owned());
    }
    String::from_utf8(bytes).map_err(|_| "AI 응답이 UTF-8 텍스트가 아닙니다".to_owned())
}

fn read_bounded_error_tail(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let length = file.metadata().map_err(|error| error.to_string())?.len();
    if length > MAX_PROVIDER_ERROR_BYTES {
        return Err("AI CLI 오류 기록이 안전한 한도를 넘었습니다".to_owned());
    }
    let start = length.saturating_sub(MAX_STATUS_OUTPUT_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity((length - start) as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn remove_private_file(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn provider_failure_message(provider: AssistantProviderKind, stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("invalid mcp configuration")
        || lower.contains("unknown option")
        || lower.contains("unrecognized option")
        || lower.contains("invalid argument")
    {
        format!(
            "{} 실행 옵션 호환성 오류입니다. CLI 상태에서 경로와 버전을 다시 확인해 주세요. BroomSweepy 연동 코드가 지원하지 않는 옵션을 전달했을 수도 있습니다",
            provider.label()
        )
    } else if lower.contains("login")
        || lower.contains("authentication")
        || lower.contains("authenticate")
        || lower.contains("oauth access token has expired")
        || lower.contains("invalid api key")
        || lower.contains("api error: 401")
        || lower.contains("unauthorized")
    {
        format!(
            "{REAUTHENTICATION_PREFIX}{} 로그인 정보가 만료되었거나 서버에서 거부되었습니다. 터미널에서 {}로 다시 로그인한 뒤, 앱의 CLI 상태를 다시 확인해 주세요",
            provider.label(),
            match provider {
                AssistantProviderKind::ClaudeCode => "claude auth login",
                AssistantProviderKind::Codex => "codex login",
                _ => provider.executable_name(),
            }
        )
    } else if lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("quota")
        || lower.contains("overloaded")
        || lower.contains("usage limit")
        || lower.contains("credit balance")
    {
        format!(
            "{} 사용량 제한 또는 서비스 혼잡으로 응답하지 못했습니다. 잠시 후 다시 시도해 주세요",
            provider.label()
        )
    } else if provider == AssistantProviderKind::Ollama
        && (lower.contains("connection") || lower.contains("refused") || lower.contains("server"))
    {
        "Ollama 서비스가 실행 중인지 확인한 뒤 다시 시도해 주세요".to_owned()
    } else if lower.contains("network") || lower.contains("connection") {
        format!(
            "{} 서비스에 연결하지 못했습니다. 인터넷 연결을 확인해 주세요",
            provider.label()
        )
    } else {
        format!(
            "{}가 응답을 완료하지 못했습니다. 터미널에서 상태를 확인해 주세요",
            provider.label()
        )
    }
}

fn provider_status(provider: AssistantProviderKind, busy: bool) -> AssistantProviderStatus {
    resolve_provider(provider, busy).0
}

fn empty_provider_status(provider: AssistantProviderKind, busy: bool) -> AssistantProviderStatus {
    AssistantProviderStatus {
        provider,
        label: provider.label(),
        installed: false,
        authentication: AssistantAuthentication::Unknown,
        available: false,
        busy,
        detail: format!(
            "{} CLI를 찾지 못했습니다. 공식 CLI를 별도로 설치한 뒤 다시 확인해 주세요. 데스크톱 앱 설치만으로 CLI 설치가 보장되지는 않습니다",
            provider.label()
        ),
        models: Vec::new(),
        state: AssistantCliState::NotInstalled,
        executable_path: None,
        version: None,
        passed_launch_checks: false,
    }
}

fn provider_status_failed(provider: AssistantProviderKind, busy: bool) -> AssistantProviderStatus {
    let mut status = empty_provider_status(provider, busy);
    status.state = AssistantCliState::CheckFailed;
    status.detail = format!(
        "{} 상태 확인에 실패했습니다. 설치나 로그인 여부는 아직 확인되지 않았습니다. 다시 확인해 주세요",
        provider.label()
    );
    status
}

fn is_app_bundled_program(program: &ExternalProgram) -> bool {
    let path = program
        .path()
        .canonicalize()
        .unwrap_or_else(|_| program.path().to_path_buf());
    let components = path.components().collect::<Vec<_>>();
    components.windows(2).any(|pair| {
        pair[0].as_os_str().to_string_lossy().ends_with(".app") && pair[1].as_os_str() == "Contents"
    })
}

fn resolve_provider(
    provider: AssistantProviderKind,
    busy: bool,
) -> (AssistantProviderStatus, Option<ExternalProgram>) {
    resolve_candidates(
        provider,
        busy,
        find_external_programs(provider.executable_name()),
    )
}

fn resolve_candidates(
    provider: AssistantProviderKind,
    busy: bool,
    programs: Vec<ExternalProgram>,
) -> (AssistantProviderStatus, Option<ExternalProgram>) {
    resolve_candidates_cancellable(provider, busy, programs, None)
}

fn resolve_candidates_cancellable(
    provider: AssistantProviderKind,
    busy: bool,
    programs: Vec<ExternalProgram>,
    cancellation: Option<&AtomicBool>,
) -> (AssistantProviderStatus, Option<ExternalProgram>) {
    let mut first_failure = None;
    let mut skipped = 0;
    // App-private binaries are not a supported standalone CLI installation.
    // Do not silently borrow them merely because an app injected them into PATH.
    for program in programs
        .into_iter()
        .filter(|program| !is_app_bundled_program(program))
        .take(8)
    {
        let mut status = inspect_candidate(provider, busy, &program, cancellation);
        if matches!(
            status.state,
            AssistantCliState::Broken | AssistantCliState::Incompatible
        ) || (status.state == AssistantCliState::CheckFailed && !status.passed_launch_checks)
        {
            skipped += 1;
            first_failure.get_or_insert(status);
            continue;
        }
        if skipped > 0 {
            status.detail.push_str(&format!(" 앞선 CLI 후보 {skipped}개의 실행 또는 호환성 검사 실패로 다른 설치본을 선택했습니다."));
        }
        // A healthy but signed-out CLI is not skipped: that could switch accounts.
        return (status, Some(program));
    }
    (
        first_failure.unwrap_or_else(|| empty_provider_status(provider, busy)),
        None,
    )
}

const CLAUDE_CHAT_ARGS: &[&str] = &[
    "--print",
    "--no-session-persistence",
    "--tools",
    "",
    "--permission-mode",
    "dontAsk",
    "--strict-mcp-config",
    "--mcp-config",
    r#"{"mcpServers":{}}"#,
    "--setting-sources",
    "",
    "--settings",
    r#"{"disableAllHooks":true}"#,
    "--disable-slash-commands",
    "--no-chrome",
    "--output-format",
    "text",
];

fn configure_claude_chat(command: &mut std::process::Command) {
    command
        .args(CLAUDE_CHAT_ARGS)
        .env("ENABLE_CLAUDEAI_MCP_SERVERS", "false")
        .env("DISABLE_AUTOUPDATER", "1");
}

fn missing_chat_options(provider: AssistantProviderKind, help: &str) -> Vec<&'static str> {
    let required: Vec<&str> = match provider {
        AssistantProviderKind::Codex => vec![
            "--ephemeral",
            "--ignore-user-config",
            "--ignore-rules",
            "--skip-git-repo-check",
            "--sandbox",
            "--config",
            "--color",
            "--cd",
            "--output-last-message",
        ],
        AssistantProviderKind::ClaudeCode => CLAUDE_CHAT_ARGS
            .iter()
            .copied()
            .filter(|arg| arg.starts_with("--"))
            .collect(),
        _ => Vec::new(),
    };
    let tokens = help
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .collect::<Vec<_>>();
    let mut missing: Vec<_> = required
        .into_iter()
        .filter(|option| !tokens.contains(option))
        .collect();
    // Old Claude CLIs may treat unknown subcommands as a prompt. Do not run
    // `auth status` unless the CLI advertises its authentication command group.
    if provider == AssistantProviderKind::ClaudeCode
        && !help.split_once("Commands:").is_some_and(|(_, commands)| {
            commands
                .lines()
                .any(|line| line.split_whitespace().next() == Some("auth"))
        })
    {
        missing.push("auth status");
    }
    missing
}

fn inspect_candidate(
    provider: AssistantProviderKind,
    busy: bool,
    program: &ExternalProgram,
    cancellation: Option<&AtomicBool>,
) -> AssistantProviderStatus {
    let probe = |arguments: &[&str]| {
        status_probe_cancellable(program, arguments, Duration::from_secs(10), cancellation)
    };
    let mut status = empty_provider_status(provider, busy);
    status.installed = true;
    status.executable_path = Some(program.path().to_string_lossy().into_owned());
    let version = match probe(&["--version"]) {
        Ok(output) if output.success => output,
        Ok(_) | Err(ProbeError::Launch) => {
            status.state = AssistantCliState::Broken;
            status.detail = format!(
                "{} CLI 실행에 실패했습니다. 설치 파일 누락·손상 또는 실행 권한을 확인하고 필요하면 CLI를 재설치해 주세요. 로그인 문제로 판정한 것은 아닙니다",
                provider.label()
            );
            return status;
        }
        Err(_) => {
            status.state = AssistantCliState::CheckFailed;
            status.detail = "CLI 버전 확인이 시간 초과되었거나 결과를 읽지 못했습니다. 다시 확인해 주세요. 설치 손상이나 로그아웃으로 단정할 수 없습니다".to_owned();
            return status;
        }
    };
    status.version = version
        .stdout
        .split_whitespace()
        .chain(version.stderr.split_whitespace())
        .find(|word| {
            word.len() <= 60
                && word
                    .trim_start_matches('v')
                    .split('.')
                    .take(3)
                    .filter(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
                    .count()
                    == 3
        })
        .map(str::to_owned);
    if matches!(
        provider,
        AssistantProviderKind::Codex | AssistantProviderKind::ClaudeCode
    ) {
        let arguments: &[&str] = if provider == AssistantProviderKind::Codex {
            &["exec", "--help"]
        } else {
            &["--help"]
        };
        match probe(arguments) {
            Ok(output) if output.success => {
                let missing = missing_chat_options(provider, &output.stdout);
                if !missing.is_empty() {
                    status.state = AssistantCliState::Incompatible;
                    status.detail = format!(
                        "{} CLI에서 필요한 옵션 지원을 확인하지 못했습니다: {}. 구버전 또는 앱과의 호환성 문제일 수 있습니다. CLI 업데이트 후 다시 확인하고, 계속되면 BroomSweepy 연동을 점검해 주세요",
                        provider.label(),
                        missing.join(", ")
                    );
                    return status;
                }
            }
            _ => {
                status.state = AssistantCliState::CheckFailed;
                status.detail = "CLI 실행은 확인했지만 지원 옵션 검사를 완료하지 못했습니다. 다시 확인해 주세요".to_owned();
                return status;
            }
        }
    }
    status.passed_launch_checks = true;
    if provider == AssistantProviderKind::Ollama {
        status.authentication = AssistantAuthentication::NotRequired;
        match probe(&["list"]).and_then(|output| {
            if output.success {
                Ok(parse_ollama_models(&output.stdout))
            } else {
                Err(ProbeError::Read)
            }
        }) {
            Ok(models) => {
                status.models = models;
                status.available = !status.models.is_empty();
                status.state = if status.available {
                    AssistantCliState::Ready
                } else {
                    AssistantCliState::NoModels
                };
                status.detail = if status.available {
                    format!("Ollama 모델 {}개를 확인했습니다", status.models.len())
                } else {
                    "Ollama는 실행되지만 대화용 모델이 없습니다. 모델을 설치한 뒤 다시 확인해 주세요".to_owned()
                };
            }
            Err(_) => {
                status.state = AssistantCliState::ServiceUnavailable;
                status.detail = "Ollama CLI는 있지만 서비스에 연결하지 못했습니다. Ollama를 실행한 뒤 다시 확인해 주세요".to_owned();
            }
        }
        return status;
    }
    let arguments: &[&str] = match provider {
        AssistantProviderKind::Codex => &["login", "status"],
        AssistantProviderKind::ClaudeCode => &["auth", "status"],
        _ => &["models"],
    };
    status.authentication = probe(arguments)
        .map(|output| authentication_from_output(provider, &output))
        .unwrap_or(AssistantAuthentication::Unknown);
    (status.state, status.detail) = match status.authentication {
        AssistantAuthentication::Authenticated => (AssistantCliState::Ready, format!("{} CLI 실행과 저장된 로그인 정보를 확인했습니다. 서버의 인증 유효성은 질문을 보낼 때 확인됩니다", provider.label())),
        AssistantAuthentication::Required => (AssistantCliState::LoginRequired, format!("{} CLI는 실행되지만 로그인이 필요합니다. 터미널에서 {}로 로그인한 뒤 다시 확인해 주세요", provider.label(), if provider == AssistantProviderKind::ClaudeCode { "claude auth login" } else if provider == AssistantProviderKind::Codex { "codex login" } else { provider.executable_name() })),
        _ => (AssistantCliState::CheckFailed, "CLI 실행은 확인했지만 인증 상태를 확인하지 못했습니다. 네트워크·CLI 오류일 수 있으므로 다시 확인해 주세요. 로그인 필요로 판정하지 않았습니다".to_owned()),
    };
    status.available = status.state == AssistantCliState::Ready;
    status
}

fn authentication_from_output(
    provider: AssistantProviderKind,
    output: &ProbeOutput,
) -> AssistantAuthentication {
    if provider == AssistantProviderKind::ClaudeCode
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&output.stdout)
    {
        return match value.get("loggedIn").and_then(serde_json::Value::as_bool) {
            Some(true) if output.success => AssistantAuthentication::Authenticated,
            Some(false) => AssistantAuthentication::Required,
            _ => AssistantAuthentication::Unknown,
        };
    }
    let text = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    if text.contains("not logged in")
        || text.contains("not authenticated")
        || text.contains("please log in")
        || text.contains("please login")
        || text.contains("authentication required")
    {
        return AssistantAuthentication::Required;
    }
    if output.success
        && provider != AssistantProviderKind::ClaudeCode
        && (provider != AssistantProviderKind::Codex || text.contains("logged in"))
    {
        return AssistantAuthentication::Authenticated;
    }
    AssistantAuthentication::Unknown
}

fn ollama_models(program: &ExternalProgram) -> Result<Vec<AssistantProviderModel>, String> {
    let output = status_probe(program, &["list"])
        .map_err(|_| "Ollama 모델 목록을 읽지 못했습니다".to_owned())?;
    if !output.success {
        return Err("Ollama 서비스에 연결하지 못했습니다".to_owned());
    }
    Ok(parse_ollama_models(&output.stdout))
}

fn parse_ollama_models(output: &str) -> Vec<AssistantProviderModel> {
    output
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| !name.eq_ignore_ascii_case("name"))
        .filter(|name| {
            !name.is_empty()
                && name.chars().count() <= MAX_MODEL_NAME_CHARS
                && !name.chars().any(char::is_control)
        })
        .take(MAX_PROVIDER_MODELS)
        .map(|name| AssistantProviderModel {
            id: name.to_owned(),
            label: name.to_owned(),
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum ProbeError {
    Launch,
    Timeout,
    OutputLimit,
    Read,
    Cancelled,
}

struct ProbeOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn status_probe(program: &ExternalProgram, arguments: &[&str]) -> Result<ProbeOutput, ProbeError> {
    status_probe_with_timeout(program, arguments, Duration::from_secs(10))
}

fn status_probe_with_timeout(
    program: &ExternalProgram,
    arguments: &[&str],
    timeout: Duration,
) -> Result<ProbeOutput, ProbeError> {
    status_probe_cancellable(program, arguments, timeout, None)
}

fn status_probe_cancellable(
    program: &ExternalProgram,
    arguments: &[&str],
    timeout: Duration,
    cancellation: Option<&AtomicBool>,
) -> Result<ProbeOutput, ProbeError> {
    if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        return Err(ProbeError::Cancelled);
    }
    // Anonymous files avoid pipe-buffer deadlock. Never surface raw auth output
    // (which can contain account details) in diagnostics or logs.
    let mut stdout = tempfile::tempfile().map_err(|_| ProbeError::Read)?;
    let mut stderr = tempfile::tempfile().map_err(|_| ProbeError::Read)?;
    let mut command = program.command();
    let mut child = command
        .args(arguments)
        .current_dir(std::env::temp_dir())
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            stdout.try_clone().map_err(|_| ProbeError::Read)?,
        ))
        .stderr(Stdio::from(
            stderr.try_clone().map_err(|_| ProbeError::Read)?,
        ))
        .spawn()
        .map_err(|_| ProbeError::Launch)?;
    let started = Instant::now();
    loop {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProbeError::Cancelled);
        }
        let oversized = [&stdout, &stderr].iter().any(|file| {
            file.metadata()
                .map_or(true, |metadata| metadata.len() > MAX_STATUS_OUTPUT_BYTES)
        });
        if oversized {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProbeError::OutputLimit);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(ProbeOutput {
                    success: status.success(),
                    stdout: read_probe_file(&mut stdout)?,
                    stderr: read_probe_file(&mut stderr)?,
                });
            }
            Ok(None) if started.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProbeError::Timeout);
            }
        }
    }
}

fn read_probe_file(file: &mut File) -> Result<String, ProbeError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ProbeError::Read)?;
    let mut bytes = Vec::new();
    file.take(MAX_STATUS_OUTPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ProbeError::Read)?;
    if bytes.len() as u64 > MAX_STATUS_OUTPUT_BYTES {
        return Err(ProbeError::OutputLimit);
    }
    String::from_utf8(bytes).map_err(|_| ProbeError::Read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_oauth_is_an_actionable_structured_error_without_raw_output() {
        let error = AssistantChatError::from(provider_failure_message(
            AssistantProviderKind::ClaudeCode,
            "Failed to authenticate. API Error: 401 OAuth access token has expired. Re-authenticate to continue. PRIVATE_TOKEN",
        ));
        assert_eq!(error.kind, "authentication");
        assert!(error.message.contains("claude auth login"));
        assert!(!error.message.contains("PRIVATE_TOKEN"));
        assert!(!error.message.contains(REAUTHENTICATION_PREFIX));
    }

    #[cfg(unix)]
    #[test]
    fn server_error_on_stdout_is_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        let program = fake_cli(
            dir.path(),
            "claude",
            "printf 'API Error: 401 OAuth access token has expired'; exit 1",
        );
        let result = run_provider(
            AssistantProviderKind::ClaudeCode,
            program,
            None,
            dir.path().join("workspace"),
            0,
            "synthetic test".into(),
            std::sync::Arc::new(AtomicBool::new(false)),
        );
        let error = AssistantChatError::from(result.unwrap_err());
        assert_eq!(error.kind, "authentication");
        assert!(error.message.contains("다시 로그인"));
    }

    fn probe_output(success: bool, stdout: &str, stderr: &str) -> ProbeOutput {
        ProbeOutput {
            success,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    #[test]
    fn auth_failures_require_positive_evidence_of_missing_login() {
        let claude = AssistantProviderKind::ClaudeCode;
        let codex = AssistantProviderKind::Codex;
        assert_eq!(
            authentication_from_output(claude, &probe_output(false, r#"{"loggedIn":false}"#, "")),
            AssistantAuthentication::Required
        );
        assert_eq!(
            authentication_from_output(claude, &probe_output(true, r#"{"loggedIn":true}"#, "")),
            AssistantAuthentication::Authenticated
        );
        assert_eq!(
            authentication_from_output(codex, &probe_output(false, "", "Not logged in")),
            AssistantAuthentication::Required
        );
        assert_eq!(
            authentication_from_output(codex, &probe_output(true, "", "Logged in using ChatGPT")),
            AssistantAuthentication::Authenticated
        );
        for error in [
            "spawn codex ENOENT",
            "network timeout",
            "unknown option",
            "",
        ] {
            assert_eq!(
                authentication_from_output(codex, &probe_output(false, "", error)),
                AssistantAuthentication::Unknown
            );
        }
        assert_eq!(
            authentication_from_output(
                claude,
                &probe_output(true, "unrecognized status format", "")
            ),
            AssistantAuthentication::Unknown
        );
    }

    #[test]
    fn missing_cli_and_failed_check_do_not_claim_login_required() {
        let (missing, program) = resolve_candidates(AssistantProviderKind::Codex, false, vec![]);
        assert!(program.is_none());
        assert_eq!(missing.state, AssistantCliState::NotInstalled);
        assert_eq!(missing.authentication, AssistantAuthentication::Unknown);
        let failed = provider_status_failed(AssistantProviderKind::Codex, false);
        assert_eq!(failed.state, AssistantCliState::CheckFailed);
        assert_eq!(failed.authentication, AssistantAuthentication::Unknown);
    }

    #[test]
    fn app_bundle_is_not_a_standalone_installation() {
        let program = ExternalProgram::Direct(PathBuf::from(
            "/Applications/Example.app/Contents/Resources/codex",
        ));
        let (status, selected) =
            resolve_candidates(AssistantProviderKind::Codex, false, vec![program]);
        assert_eq!(status.state, AssistantCliState::NotInstalled);
        assert!(selected.is_none());
    }

    #[test]
    fn claude_supported_arguments_keep_isolation_without_safe_mode() {
        let mut command = std::process::Command::new("claude");
        configure_claude_chat(&mut command);
        let args = command
            .get_args()
            .map(|arg| arg.to_str().unwrap())
            .collect::<Vec<_>>();
        assert!(!args.contains(&"--safe-mode"));
        for pair in [
            ["--tools", ""],
            ["--permission-mode", "dontAsk"],
            ["--setting-sources", ""],
            ["--settings", r#"{"disableAllHooks":true}"#],
            ["--mcp-config", r#"{"mcpServers":{}}"#],
        ] {
            assert!(args.windows(2).any(|candidate| candidate == pair));
        }
        for flag in [
            "--no-session-persistence",
            "--strict-mcp-config",
            "--disable-slash-commands",
            "--no-chrome",
        ] {
            assert!(args.contains(&flag));
        }
        assert!(
            command
                .get_envs()
                .any(|(key, value)| key == "ENABLE_CLAUDEAI_MCP_SERVERS"
                    && value == Some(std::ffi::OsStr::new("false")))
        );
        assert!(
            missing_chat_options(
                AssistantProviderKind::ClaudeCode,
                &format!(
                    "{}\nCommands:\n  auth Manage authentication",
                    CLAUDE_CHAT_ARGS.join(" ")
                )
            )
            .is_empty()
        );
        assert!(
            !missing_chat_options(AssistantProviderKind::ClaudeCode, "--tools-extra --print")
                .is_empty()
        );
    }

    #[cfg(unix)]
    fn fake_cli(directory: &Path, name: &str, script: &str) -> ExternalProgram {
        use std::os::unix::fs::PermissionsExt;
        let path = directory.join(name);
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        ExternalProgram::Direct(path)
    }

    #[cfg(unix)]
    fn fake_claude(
        directory: &Path,
        name: &str,
        help: &str,
        authenticated: bool,
    ) -> ExternalProgram {
        fake_cli(
            directory,
            name,
            &format!(
                "case \"$1\" in\n--version) printf '%s\\n' '2.1.147 (Claude Code)' ;;\n--help) printf '%s\\n' '{help}' ;;\nauth) printf '%s\\n' '{{\"loggedIn\":{authenticated}}}'; {} ;;\n*) exit 2 ;;\nesac",
                if authenticated { "exit 0" } else { "exit 1" }
            ),
        )
    }

    #[cfg(unix)]
    #[test]
    fn broken_and_incompatible_candidates_fall_back_but_signed_out_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let broken = fake_cli(dir.path(), "broken", "printf 'spawn ENOENT' >&2; exit 1");
        let old = fake_claude(dir.path(), "old", "--print", true);
        let help = CLAUDE_CHAT_ARGS
            .iter()
            .copied()
            .filter(|arg| arg.starts_with("--"))
            .collect::<Vec<_>>()
            .join(" ")
            + "\nCommands:\n  auth Manage authentication";
        let good = fake_claude(dir.path(), "good", &help, true);
        let signed_out = fake_claude(dir.path(), "signed-out", &help, false);
        let (status, selected) = resolve_candidates(
            AssistantProviderKind::ClaudeCode,
            false,
            vec![broken.clone(), old.clone(), good.clone()],
        );
        assert_eq!(status.state, AssistantCliState::Ready);
        assert_eq!(status.version.as_deref(), Some("2.1.147"));
        assert_eq!(
            selected.as_ref().map(ExternalProgram::path),
            Some(good.path())
        );
        assert!(status.detail.contains("2개"));
        let (status, _) =
            resolve_candidates(AssistantProviderKind::ClaudeCode, false, vec![broken]);
        assert_eq!(status.state, AssistantCliState::Broken);
        assert_eq!(status.authentication, AssistantAuthentication::Unknown);
        let (status, _) = resolve_candidates(AssistantProviderKind::ClaudeCode, false, vec![old]);
        assert_eq!(status.state, AssistantCliState::Incompatible);
        let (status, selected) = resolve_candidates(
            AssistantProviderKind::ClaudeCode,
            false,
            vec![signed_out.clone(), good],
        );
        assert_eq!(status.state, AssistantCliState::LoginRequired);
        assert_eq!(
            selected.as_ref().map(ExternalProgram::path),
            Some(signed_out.path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn old_claude_without_auth_subcommand_is_rejected_before_auth_probe() {
        let dir = tempfile::tempdir().unwrap();
        let help = CLAUDE_CHAT_ARGS
            .iter()
            .copied()
            .filter(|arg| arg.starts_with("--"))
            .collect::<Vec<_>>()
            .join(" ");
        let program = fake_claude(dir.path(), "old-claude", &help, false);
        let (status, selected) =
            resolve_candidates(AssistantProviderKind::ClaudeCode, false, vec![program]);
        assert_eq!(status.state, AssistantCliState::Incompatible);
        assert!(status.detail.contains("auth status"));
        assert_eq!(status.authentication, AssistantAuthentication::Unknown);
        assert!(selected.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn probes_bound_time_and_both_output_streams_without_pipe_deadlock() {
        let dir = tempfile::tempdir().unwrap();
        let timeout = fake_cli(dir.path(), "timeout", "exec /bin/sleep 2");
        assert!(matches!(
            status_probe_with_timeout(&timeout, &[], Duration::from_millis(100)),
            Err(ProbeError::Timeout)
        ));
        let noisy = fake_cli(dir.path(), "noisy", "head -c 70000 /dev/zero >&2");
        assert!(matches!(
            status_probe(&noisy, &[]),
            Err(ProbeError::OutputLimit)
        ));
        let stderr = fake_cli(dir.path(), "stderr", "printf 'Not logged in' >&2; exit 1");
        let output = status_probe(&stderr, &[]).unwrap();
        assert!(!output.success);
        assert_eq!(output.stderr, "Not logged in");
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_interrupts_cli_preflight() {
        let dir = tempfile::tempdir().unwrap();
        let program = fake_cli(dir.path(), "slow", "exec /bin/sleep 5");
        let cancelled = AtomicBool::new(true);
        assert!(matches!(
            status_probe_cancellable(&program, &[], Duration::from_secs(10), Some(&cancelled)),
            Err(ProbeError::Cancelled)
        ));
        cancelled.store(false, Ordering::Release);
        thread::scope(|scope| {
            scope.spawn(|| {
                thread::sleep(Duration::from_millis(100));
                cancelled.store(true, Ordering::Release);
            });
            assert!(matches!(
                status_probe_cancellable(&program, &[], Duration::from_secs(10), Some(&cancelled)),
                Err(ProbeError::Cancelled)
            ));
        });
    }

    #[test]
    #[ignore = "Opt-in read-only diagnostics for installed provider CLIs; no prompts or login changes"]
    fn installed_cli_diagnostics() {
        for provider in ASSISTANT_PROVIDERS {
            println!(
                "{}",
                serde_json::to_string(&provider_status(provider, false)).unwrap()
            );
        }
    }

    fn valid_request() -> AssistantChatRequest {
        AssistantChatRequest {
            session_id: None,
            provider: AssistantProviderKind::Codex,
            model: None,
            message: "이 폴더에서 용량이 큰 부분을 알려줘".to_owned(),
            history: Vec::new(),
            scope_kind: AssistantScopeKind::Folder,
            include_docker_status: false,
            response_language: AssistantResponseLanguage::English,
            summary: AssistantFolderSummary {
                scope_name: ".codex".to_owned(),
                completed_at_unix_ms: 1,
                total_logical_bytes: 10,
                total_files: 2,
                total_directories: 1,
                unreadable_entries: 0,
                empty_directory_count: 0,
                children_truncated: false,
                children: vec![AssistantFolderChild {
                    name: "sessions".to_owned(),
                    kind: AssistantFolderChildKind::Directory,
                    logical_bytes: 10,
                    file_count: 2,
                    directory_count: 0,
                }],
            },
        }
    }

    #[test]
    fn prompt_contains_only_the_bounded_summary_contract() {
        let prompt = build_prompt(&valid_request(), None).expect("prompt");
        assert!(prompt.contains(".codex"));
        assert!(prompt.contains("sessions"));
        assert!(prompt.contains("You did not read the disk directly"));
        assert!(prompt.contains("Reply in concise English"));
        assert!(!prompt.contains("C:\\Users"));
    }

    #[test]
    fn docker_context_remains_advisory_and_points_back_to_app_confirmation() {
        let prompt = build_prompt(
            &valid_request(),
            Some(r#"{"enabled":true,"available":true,"reclaimableBytes":21100000000}"#),
        )
        .expect("prompt");
        assert!(prompt.contains("Docker cleanup review"));
        assert!(prompt.contains("21100000000"));
        assert!(prompt.contains("Do not provide commands or claim that you executed anything"));
    }

    #[test]
    fn docker_scope_uses_docker_summary_without_a_folder_claim() {
        let mut request = valid_request();
        request.scope_kind = AssistantScopeKind::Docker;
        request.include_docker_status = true;
        request.summary.scope_name = "Docker".to_owned();
        request.summary.children.clear();
        validate_request(&request).expect("valid Docker request");
        let prompt = build_prompt(
            &request,
            Some(r#"{"enabled":true,"available":true,"totalSizeBytes":10}"#),
        )
        .expect("Docker prompt");
        assert!(prompt.contains("subject of this chat is Docker on this computer"));
        assert!(!prompt.contains("[Folder summary]"));
    }

    #[test]
    fn request_rejects_oversized_children() {
        let mut request = valid_request();
        request.summary.children = (0..=MAX_CHILDREN)
            .map(|index| AssistantFolderChild {
                name: format!("child-{index}"),
                kind: AssistantFolderChildKind::File,
                logical_bytes: 1,
                file_count: 1,
                directory_count: 0,
            })
            .collect();
        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn provider_kinds_use_stable_camel_case_wire_names() {
        assert_eq!(
            serde_json::to_string(&AssistantProviderKind::Codex).expect("codex"),
            "\"codex\""
        );
        assert_eq!(
            serde_json::to_string(&AssistantProviderKind::ClaudeCode).expect("claude"),
            "\"claudeCode\""
        );
        assert_eq!(
            serde_json::to_string(&AssistantProviderKind::Grok).expect("grok"),
            "\"grok\""
        );
        assert_eq!(
            serde_json::to_string(&AssistantProviderKind::Antigravity).expect("antigravity"),
            "\"antigravity\""
        );
        assert_eq!(
            serde_json::to_string(&AssistantProviderKind::Ollama).expect("ollama"),
            "\"ollama\""
        );
    }

    #[test]
    fn response_languages_use_stable_ui_wire_names() {
        assert_eq!(
            serde_json::from_str::<AssistantResponseLanguage>("\"en\"").expect("English"),
            AssistantResponseLanguage::English
        );
        assert_eq!(
            serde_json::from_str::<AssistantResponseLanguage>("\"ko\"").expect("Korean"),
            AssistantResponseLanguage::Korean
        );
        assert_eq!(
            serde_json::from_str::<AssistantResponseLanguage>("\"ja\"").expect("Japanese"),
            AssistantResponseLanguage::Japanese
        );
        assert_eq!(
            serde_json::from_str::<AssistantResponseLanguage>("\"zh-CN\"")
                .expect("Simplified Chinese"),
            AssistantResponseLanguage::SimplifiedChinese
        );
    }

    #[test]
    fn ollama_model_list_is_bounded_and_skips_the_header() {
        let models = parse_ollama_models(
            "NAME ID SIZE MODIFIED\nqwen3-coder:30b abc 18 GB 2 months ago\nbge-m3:latest def 1 GB 3 months ago\n",
        );
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["qwen3-coder:30b", "bge-m3:latest"]
        );
    }

    #[test]
    fn ollama_requires_an_explicit_installed_model_name() {
        let mut request = valid_request();
        request.provider = AssistantProviderKind::Ollama;
        assert!(validate_request(&request).is_err());
        request.model = Some("qwen3-coder:30b".to_owned());
        assert!(validate_request(&request).is_ok());
    }

    #[test]
    fn ollama_gets_a_longer_bounded_response_window() {
        assert_eq!(
            AssistantProviderKind::Codex.response_timeout(),
            Duration::from_secs(120)
        );
        assert_eq!(
            AssistantProviderKind::Ollama.response_timeout(),
            Duration::from_secs(600)
        );
    }

    #[test]
    fn provider_failures_distinguish_cli_option_and_local_service_errors() {
        assert!(
            provider_failure_message(
                AssistantProviderKind::ClaudeCode,
                "Error: Invalid MCP configuration"
            )
            .contains("실행 옵션")
        );
        assert!(
            provider_failure_message(AssistantProviderKind::Ollama, "connection refused")
                .contains("Ollama 서비스")
        );
    }
}
