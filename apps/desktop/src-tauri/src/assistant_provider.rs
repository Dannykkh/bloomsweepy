use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    provider: AssistantProviderKind,
    model: Option<String>,
    message: String,
    history: Vec<AssistantChatTurn>,
    summary: AssistantFolderSummary,
    scope_kind: AssistantScopeKind,
    include_docker_status: bool,
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
) -> Result<AssistantChatResponse, String> {
    validate_request(&request)?;
    state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "이미 AI 응답을 기다리고 있습니다".to_owned())?;
    let _lease = ProviderLease { state: &state };
    state.cancellation.store(false, Ordering::Release);

    let provider = request.provider;
    let program = find_provider_program(provider).ok_or_else(|| {
        format!(
            "{}를 찾지 못했습니다. 먼저 설치하고 로그인하세요",
            provider.label()
        )
    })?;
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
    let prompt = build_prompt(&request, docker_context_json.as_deref())?;
    let cancellation = std::sync::Arc::clone(&state.cancellation);
    let response_model = request.model.clone();
    let run_model = response_model.clone();

    tauri::async_runtime::spawn_blocking(move || {
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
    .map_err(|error| format!("{} 실행 작업이 중단됐습니다: {error}", provider.label()))?
    .map(|message| AssistantChatResponse {
        provider,
        label: provider.label(),
        model: response_model,
        message,
        docker_context,
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
                AssistantChatRole::User => "사용자",
                AssistantChatRole::Assistant => "도우미",
            };
            format!("{role}: {}", turn.content.trim())
        })
        .collect::<Vec<_>>()
        .join("\n");

    let docker_context = docker_context_json.map_or_else(
        || "[Docker 사용량]\n요청하지 않음".to_owned(),
        |context| {
            format!(
                "[Docker 사용량]\n{context}\n\
                 이 값은 BroomSweepy가 Docker CLI로 읽은 제한된 요약입니다. Docker 정리를 요청받으면 범주와 이유만 설명하고, 앱의 'Docker 정리 검토'에서 최종 확인하라고 안내하세요. 앱은 개별 Docker 객체 목록이 아니라 범주별 참고 상한과 고정 명령만 보여줍니다. 명령을 제시하거나 실행했다고 말하지 마세요."
            )
        },
    );

    let scope_context = match request.scope_kind {
        AssistantScopeKind::Folder => format!(
            "아래 JSON은 BroomSweepy가 사용자가 선택한 폴더를 읽기 전용으로 검사해 만든 제한된 요약입니다.\n\
             전체 경로와 파일 내용은 전달되지 않았습니다.\n\n\
             [폴더 요약]\n{summary}"
        ),
        AssistantScopeKind::Docker => {
            "이 대화의 대상은 폴더가 아니라 이 컴퓨터의 Docker입니다. 폴더를 선택하거나 파일을 직접 읽었다고 말하지 마세요.\n\
             BroomSweepy가 제공한 Docker 범주별 요약만 사용하세요."
                .to_owned()
        }
    };

    Ok(format!(
        "당신은 BroomSweepy의 저장공간 분석 도우미입니다. 반드시 한국어로 간결하게 답하세요.\n\
         {scope_context}\n\
         당신은 디스크를 직접 읽지 않았으며, 셸이나 다른 도구를 사용하지 마세요. 요약에 없는 사실은 추측하지 말고 '추가 검사가 필요합니다'라고 말하세요.\n\
         삭제를 확정하거나 실행했다고 말하지 마세요. 삭제 후보가 있으면 이유와 앱에서 사용자가 확인할 다음 행동만 제안하세요.\n\
         응답은 앱에 일반 텍스트로 표시됩니다. Markdown 굵게, 제목, 코드 펜스, 백틱, 메타데이터 태그를 쓰지 말고 짧은 한국어 문장과 하이픈 목록만 사용하세요.\n\n\
         {docker_context}\n\n\
         [최근 대화]\n{history}\n\n\
         [사용자 질문]\n{}",
        request.message.trim()
    ))
}

fn run_provider(
    provider: AssistantProviderKind,
    program: ProviderProgram,
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
            command
                .arg("--print")
                .arg("--no-session-persistence")
                .arg("--safe-mode")
                .arg("--tools")
                .arg("")
                .arg("--permission-mode")
                .arg("dontAsk")
                .arg("--strict-mcp-config")
                .arg("--mcp-config")
                .arg(r#"{"mcpServers":{}}"#)
                .arg("--output-format")
                .arg("text")
                .stdout(Stdio::from(response_file));
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
        return Err(provider_failure_message(provider, &provider_error));
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
            "{} 실행 옵션이 현재 CLI 버전과 맞지 않습니다. AI CLI 상태를 다시 확인하거나 앱을 업데이트해 주세요",
            provider.label()
        )
    } else if lower.contains("login")
        || lower.contains("authentication")
        || lower.contains("unauthorized")
    {
        format!(
            "{} 로그인이 필요합니다. 해당 CLI에서 먼저 로그인하세요",
            provider.label()
        )
    } else if lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("quota")
        || lower.contains("overloaded")
        || lower.contains("usage limit")
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

#[derive(Clone, Debug)]
enum ProviderProgram {
    Direct(PathBuf),
    #[cfg(windows)]
    CommandScript(PathBuf),
}

impl ProviderProgram {
    fn command(&self) -> Command {
        match self {
            Self::Direct(path) => Command::new(path),
            #[cfg(windows)]
            Self::CommandScript(path) => {
                let mut command = Command::new("cmd.exe");
                command.arg("/D").arg("/C").arg(path);
                command
            }
        }
    }
}

fn find_provider_program(provider: AssistantProviderKind) -> Option<ProviderProgram> {
    let path = env::var_os("PATH")?;
    let executable_name = provider.executable_name();
    for directory in env::split_paths(&path) {
        #[cfg(windows)]
        {
            let executable = directory.join(format!("{executable_name}.exe"));
            if executable.is_file() {
                return Some(ProviderProgram::Direct(executable));
            }
            let command_script = directory.join(format!("{executable_name}.cmd"));
            if command_script.is_file() {
                return Some(ProviderProgram::CommandScript(command_script));
            }
        }
        #[cfg(not(windows))]
        {
            let executable = directory.join(executable_name);
            if executable.is_file() {
                return Some(ProviderProgram::Direct(executable));
            }
        }
    }
    None
}

fn provider_status(provider: AssistantProviderKind, busy: bool) -> AssistantProviderStatus {
    let program = find_provider_program(provider);
    let installed = program.is_some();
    let mut models = Vec::new();
    let mut provider_ready = installed;
    let authentication = match (provider, program.as_ref()) {
        (_, None) => AssistantAuthentication::Required,
        (AssistantProviderKind::Codex, Some(program)) => {
            authentication_command_succeeds(program, &["login", "status"])
        }
        (AssistantProviderKind::ClaudeCode, Some(program)) => {
            authentication_command_succeeds(program, &["auth", "status"])
        }
        (AssistantProviderKind::Grok, Some(program)) => {
            authentication_command_succeeds(program, &["models"])
        }
        (AssistantProviderKind::Antigravity, Some(program)) => {
            authentication_command_succeeds(program, &["models"])
        }
        (AssistantProviderKind::Ollama, Some(program)) => {
            match ollama_models(program) {
                Ok(installed_models) => models = installed_models,
                Err(_) => provider_ready = false,
            }
            AssistantAuthentication::NotRequired
        }
    };
    let available = installed
        && provider_ready
        && authentication != AssistantAuthentication::Required
        && (provider != AssistantProviderKind::Ollama || !models.is_empty());
    AssistantProviderStatus {
        provider,
        label: provider.label(),
        installed,
        authentication,
        available,
        busy,
        detail: match (installed, provider, authentication, models.len()) {
            (false, _, _, _) => format!("{}를 찾지 못했습니다", provider.label()),
            (true, AssistantProviderKind::Ollama, _, count) if count > 0 => {
                format!("Ollama에서 설치된 모델 {count}개를 확인했습니다")
            }
            (true, AssistantProviderKind::Ollama, _, _) => {
                "Ollama 모델 목록을 읽지 못했거나 설치된 모델이 없습니다".to_owned()
            }
            (true, _, AssistantAuthentication::Authenticated, _) => {
                format!("{} 로그인 상태를 확인했습니다", provider.label())
            }
            (true, _, AssistantAuthentication::Required, _) => {
                format!("{}는 있지만 로그인이 필요합니다", provider.label())
            }
            (true, _, AssistantAuthentication::NotRequired, _) => {
                format!("{}가 준비됐습니다", provider.label())
            }
        },
        models,
    }
}

fn provider_status_failed(provider: AssistantProviderKind, busy: bool) -> AssistantProviderStatus {
    AssistantProviderStatus {
        provider,
        label: provider.label(),
        installed: false,
        authentication: AssistantAuthentication::Required,
        available: false,
        busy,
        detail: format!("{} 상태 확인 작업을 완료하지 못했습니다", provider.label()),
        models: Vec::new(),
    }
}

fn authentication_command_succeeds(
    program: &ProviderProgram,
    arguments: &[&str],
) -> AssistantAuthentication {
    if status_command_output(program, arguments).is_ok() {
        AssistantAuthentication::Authenticated
    } else {
        AssistantAuthentication::Required
    }
}

fn ollama_models(program: &ProviderProgram) -> Result<Vec<AssistantProviderModel>, String> {
    let output = status_command_output(program, &["list"])
        .map_err(|_| "Ollama 모델 목록을 읽지 못했습니다".to_owned())?;
    Ok(parse_ollama_models(&output))
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

fn status_command_output(program: &ProviderProgram, arguments: &[&str]) -> Result<String, ()> {
    let mut command = program.command();
    let mut child = command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(());
                }
                let mut bytes = Vec::new();
                child
                    .stdout
                    .take()
                    .ok_or(())?
                    .take(MAX_STATUS_OUTPUT_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| ())?;
                if bytes.len() as u64 > MAX_STATUS_OUTPUT_BYTES {
                    return Err(());
                }
                return String::from_utf8(bytes).map_err(|_| ());
            }
            Ok(None) if started.elapsed() < Duration::from_secs(10) => {
                thread::sleep(Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> AssistantChatRequest {
        AssistantChatRequest {
            provider: AssistantProviderKind::Codex,
            model: None,
            message: "이 폴더에서 용량이 큰 부분을 알려줘".to_owned(),
            history: Vec::new(),
            scope_kind: AssistantScopeKind::Folder,
            include_docker_status: false,
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
        assert!(prompt.contains("디스크를 직접 읽지 않았으며"));
        assert!(!prompt.contains("C:\\Users"));
    }

    #[test]
    fn docker_context_remains_advisory_and_points_back_to_app_confirmation() {
        let prompt = build_prompt(
            &valid_request(),
            Some(r#"{"enabled":true,"available":true,"reclaimableBytes":21100000000}"#),
        )
        .expect("prompt");
        assert!(prompt.contains("Docker 정리 검토"));
        assert!(prompt.contains("21100000000"));
        assert!(prompt.contains("실행했다고 말하지 마세요"));
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
        assert!(prompt.contains("대상은 폴더가 아니라 이 컴퓨터의 Docker"));
        assert!(!prompt.contains("[폴더 요약]"));
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
