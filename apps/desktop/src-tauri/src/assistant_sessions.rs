use crate::assistant_provider::{
    AssistantChatRole, AssistantFolderSummary, AssistantProviderKind, AssistantScopeKind,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const DATABASE_FILE_NAME: &str = "assistant-sessions-v1.sqlite3";
const SCHEMA_VERSION: i64 = 2;
const MAX_SESSIONS: i64 = 200;
const MAX_MESSAGES_PER_SESSION: i64 = 200;
const MAX_SESSION_ID_CHARS: usize = 64;
const MAX_SCOPE_ROOT_CHARS: usize = 32_768;
const MAX_SCOPE_NAME_CHARS: usize = 240;
const MAX_FOLDER_CHILDREN: usize = 24;
const MAX_FOLDER_SUMMARY_BYTES: usize = 128 * 1024;
const MAX_USER_MESSAGE_CHARS: usize = 2_000;
const MAX_ASSISTANT_MESSAGE_CHARS: usize = 65_536;
const MAX_MODEL_NAME_CHARS: usize = 160;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantSessionSummary {
    id: String,
    scope_kind: AssistantScopeKind,
    scope_root: String,
    scope_name: String,
    created_at_unix_ms: u64,
    updated_at_unix_ms: u64,
    message_count: u64,
    last_provider: Option<AssistantProviderKind>,
    last_model: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantSessionDetail {
    session: AssistantSessionSummary,
    folder_summary: AssistantFolderSummary,
    messages: Vec<AssistantStoredMessage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantStoredMessage {
    sequence: u64,
    role: AssistantChatRole,
    content: String,
    provider: Option<AssistantProviderKind>,
    provider_label: Option<String>,
    model: Option<String>,
    created_at_unix_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantMessageMutation {
    session: AssistantSessionSummary,
    message: AssistantStoredMessage,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateAssistantSessionRequest {
    scope_kind: AssistantScopeKind,
    scope_root: String,
    folder_summary: AssistantFolderSummary,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppendAssistantMessageRequest {
    session_id: String,
    role: AssistantChatRole,
    content: String,
    provider: Option<AssistantProviderKind>,
    model: Option<String>,
}

#[tauri::command]
pub(crate) async fn list_assistant_sessions(
    app: AppHandle,
) -> Result<Vec<AssistantSessionSummary>, String> {
    let path = database_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || list_sessions_at(&path))
        .await
        .map_err(|error| format!("대화 기록 조회 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn create_assistant_session(
    app: AppHandle,
    request: CreateAssistantSessionRequest,
) -> Result<AssistantSessionDetail, String> {
    let path = database_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || create_session_at(&path, request))
        .await
        .map_err(|error| format!("새 대화 저장 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn get_assistant_session(
    app: AppHandle,
    session_id: String,
) -> Result<AssistantSessionDetail, String> {
    validate_session_id(&session_id)?;
    let path = database_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || get_session_at(&path, &session_id))
        .await
        .map_err(|error| format!("대화 불러오기 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn append_assistant_message(
    app: AppHandle,
    request: AppendAssistantMessageRequest,
) -> Result<AssistantMessageMutation, String> {
    let path = database_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || append_message_at(&path, request))
        .await
        .map_err(|error| format!("대화 저장 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn delete_assistant_session(
    app: AppHandle,
    session_id: String,
) -> Result<bool, String> {
    validate_session_id(&session_id)?;
    let path = database_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || delete_session_at(&path, &session_id))
        .await
        .map_err(|error| format!("대화 삭제 작업이 중단됐습니다: {error}"))?
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(DATABASE_FILE_NAME))
        .map_err(|error| format!("대화 기록 저장 위치를 찾지 못했습니다: {error}"))
}

fn list_sessions_at(path: &Path) -> Result<Vec<AssistantSessionSummary>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let connection = open_database(path)?;
    let mut statement = connection
        .prepare(
            "SELECT s.id, s.scope_root, s.scope_name, s.created_at_unix_ms,
                    s.updated_at_unix_ms, s.last_provider, s.last_model,
                    (SELECT COUNT(*) FROM assistant_messages m WHERE m.session_id = s.id),
                    s.scope_kind
             FROM assistant_sessions s
             ORDER BY s.updated_at_unix_ms DESC, s.id DESC
             LIMIT ?1",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![MAX_SESSIONS], map_session_summary)
        .map_err(database_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
}

fn create_session_at(
    path: &Path,
    request: CreateAssistantSessionRequest,
) -> Result<AssistantSessionDetail, String> {
    validate_new_session(&request)?;
    let folder_summary_json = serde_json::to_string(&request.folder_summary)
        .map_err(|error| format!("폴더 요약을 저장할 수 없습니다: {error}"))?;
    if folder_summary_json.len() > MAX_FOLDER_SUMMARY_BYTES {
        return Err("폴더 요약이 너무 커서 대화를 저장할 수 없습니다".to_owned());
    }

    let mut connection = open_database(path)?;
    let timestamp = unix_time_ms()?;
    let session_id = generate_session_id()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let session_count = transaction
        .query_row("SELECT COUNT(*) FROM assistant_sessions", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_error)?;
    if session_count >= MAX_SESSIONS {
        return Err(format!(
            "저장된 대화가 {MAX_SESSIONS}개입니다. 필요 없는 대화를 지운 뒤 새 대화를 시작해 주세요"
        ));
    }
    transaction
        .execute(
            "INSERT INTO assistant_sessions (
                id, scope_kind, scope_root, scope_name, folder_summary_json,
                created_at_unix_ms, updated_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                session_id,
                scope_wire_name(request.scope_kind),
                request.scope_root,
                request.folder_summary.scope_name,
                folder_summary_json,
                timestamp
            ],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    get_session_with_connection(&connection, &session_id)
}

fn get_session_at(path: &Path, session_id: &str) -> Result<AssistantSessionDetail, String> {
    if !path.exists() {
        return Err("저장된 대화를 찾지 못했습니다".to_owned());
    }
    let connection = open_database(path)?;
    get_session_with_connection(&connection, session_id)
}

fn get_session_with_connection(
    connection: &Connection,
    session_id: &str,
) -> Result<AssistantSessionDetail, String> {
    let row = connection
        .query_row(
            "SELECT s.id, s.scope_root, s.scope_name, s.created_at_unix_ms,
                    s.updated_at_unix_ms, s.last_provider, s.last_model,
                    (SELECT COUNT(*) FROM assistant_messages m WHERE m.session_id = s.id),
                    s.scope_kind, s.folder_summary_json
             FROM assistant_sessions s
             WHERE s.id = ?1",
            params![session_id],
            |row| {
                let session = map_session_summary(row)?;
                let folder_summary_json = row.get::<_, String>(9)?;
                Ok((session, folder_summary_json))
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| "저장된 대화를 찾지 못했습니다".to_owned())?;
    let folder_summary = serde_json::from_str::<AssistantFolderSummary>(&row.1)
        .map_err(|error| format!("저장된 폴더 요약이 손상됐습니다: {error}"))?;

    let mut statement = connection
        .prepare(
            "SELECT sequence, role, content, provider, model, created_at_unix_ms
             FROM assistant_messages
             WHERE session_id = ?1
             ORDER BY sequence ASC
             LIMIT ?2",
        )
        .map_err(database_error)?;
    let messages = statement
        .query_map(params![session_id, MAX_MESSAGES_PER_SESSION], map_message)
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    Ok(AssistantSessionDetail {
        session: row.0,
        folder_summary,
        messages,
    })
}

fn append_message_at(
    path: &Path,
    mut request: AppendAssistantMessageRequest,
) -> Result<AssistantMessageMutation, String> {
    validate_message(&request)?;
    request.content = request.content.trim().to_owned();
    let mut connection = open_database(path)?;
    let timestamp = unix_time_ms()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let session_exists = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM assistant_sessions WHERE id = ?1)",
            params![request.session_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !session_exists {
        return Err("저장할 대화를 찾지 못했습니다".to_owned());
    }
    let current_count = transaction
        .query_row(
            "SELECT COUNT(*) FROM assistant_messages WHERE session_id = ?1",
            params![request.session_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    if current_count >= MAX_MESSAGES_PER_SESSION {
        return Err(format!(
            "이 대화에는 메시지가 {MAX_MESSAGES_PER_SESSION}개 있습니다. 새 대화를 시작해 주세요"
        ));
    }
    let sequence = current_count + 1;
    let provider_name = request.provider.map(provider_wire_name);
    let role_name = role_wire_name(request.role);
    transaction
        .execute(
            "INSERT INTO assistant_messages (
                session_id, sequence, role, content, provider, model, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                request.session_id,
                sequence,
                role_name,
                request.content,
                provider_name,
                request.model,
                timestamp
            ],
        )
        .map_err(database_error)?;
    if request.role == AssistantChatRole::Assistant {
        transaction
            .execute(
                "UPDATE assistant_sessions
                 SET updated_at_unix_ms = ?2, last_provider = ?3, last_model = ?4
                 WHERE id = ?1",
                params![request.session_id, timestamp, provider_name, request.model],
            )
            .map_err(database_error)?;
    } else {
        transaction
            .execute(
                "UPDATE assistant_sessions SET updated_at_unix_ms = ?2 WHERE id = ?1",
                params![request.session_id, timestamp],
            )
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)?;

    let session = load_session_summary(&connection, &request.session_id)?
        .ok_or_else(|| "저장한 대화를 다시 찾지 못했습니다".to_owned())?;
    let message = load_message(&connection, &request.session_id, sequence)?
        .ok_or_else(|| "저장한 메시지를 다시 찾지 못했습니다".to_owned())?;
    Ok(AssistantMessageMutation { session, message })
}

fn delete_session_at(path: &Path, session_id: &str) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let connection = open_database(path)?;
    let deleted = connection
        .execute(
            "DELETE FROM assistant_sessions WHERE id = ?1",
            params![session_id],
        )
        .map_err(database_error)?
        > 0;
    if deleted {
        // Deleted pages are overwritten by secure_delete. Compacting is best-effort because
        // another short-lived reader may temporarily keep the WAL busy.
        let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;");
    }
    Ok(deleted)
}

fn load_session_summary(
    connection: &Connection,
    session_id: &str,
) -> Result<Option<AssistantSessionSummary>, String> {
    connection
        .query_row(
            "SELECT s.id, s.scope_root, s.scope_name, s.created_at_unix_ms,
                    s.updated_at_unix_ms, s.last_provider, s.last_model,
                    (SELECT COUNT(*) FROM assistant_messages m WHERE m.session_id = s.id),
                    s.scope_kind
             FROM assistant_sessions s
             WHERE s.id = ?1",
            params![session_id],
            map_session_summary,
        )
        .optional()
        .map_err(database_error)
}

fn load_message(
    connection: &Connection,
    session_id: &str,
    sequence: i64,
) -> Result<Option<AssistantStoredMessage>, String> {
    connection
        .query_row(
            "SELECT sequence, role, content, provider, model, created_at_unix_ms
             FROM assistant_messages
             WHERE session_id = ?1 AND sequence = ?2",
            params![session_id, sequence],
            map_message,
        )
        .optional()
        .map_err(database_error)
}

fn map_session_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssistantSessionSummary> {
    let created_at = row.get::<_, i64>(3)?;
    let updated_at = row.get::<_, i64>(4)?;
    let last_provider = row
        .get::<_, Option<String>>(5)?
        .map(|value| parse_provider(&value))
        .transpose()
        .map_err(|error| conversion_error(5, error))?;
    Ok(AssistantSessionSummary {
        id: row.get(0)?,
        scope_kind: parse_scope(&row.get::<_, String>(8)?)
            .map_err(|error| conversion_error(8, error))?,
        scope_root: row.get(1)?,
        scope_name: row.get(2)?,
        created_at_unix_ms: non_negative_u64(created_at, 3)?,
        updated_at_unix_ms: non_negative_u64(updated_at, 4)?,
        message_count: non_negative_u64(row.get::<_, i64>(7)?, 7)?,
        last_provider,
        last_model: row.get(6)?,
    })
}

fn map_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssistantStoredMessage> {
    let role_name = row.get::<_, String>(1)?;
    let role = parse_role(&role_name).map_err(|error| conversion_error(1, error))?;
    let provider = row
        .get::<_, Option<String>>(3)?
        .map(|value| parse_provider(&value))
        .transpose()
        .map_err(|error| conversion_error(3, error))?;
    let created_at = row.get::<_, i64>(5)?;
    Ok(AssistantStoredMessage {
        sequence: non_negative_u64(row.get::<_, i64>(0)?, 0)?,
        role,
        content: row.get(2)?,
        provider,
        provider_label: provider.map(|value| value.label().to_owned()),
        model: row.get(4)?,
        created_at_unix_ms: non_negative_u64(created_at, 5)?,
    })
}

fn open_database(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("대화 기록 폴더를 만들지 못했습니다: {error}"))?;
    }
    let connection = Connection::open(path).map_err(database_error)?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(database_error)?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA secure_delete = ON;
             PRAGMA temp_store = MEMORY;",
        )
        .map_err(database_error)?;
    initialize_schema(&connection)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(database_error)?;
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    if version == 1 {
        connection
            .execute_batch(
                "BEGIN IMMEDIATE;
                 ALTER TABLE assistant_sessions
                    ADD COLUMN scope_kind TEXT NOT NULL DEFAULT 'folder'
                    CHECK(scope_kind IN ('folder', 'docker'));
                 PRAGMA user_version = 2;
                 COMMIT;",
            )
            .map_err(database_error)?;
        return Ok(());
    }
    if version != 0 {
        return Err(format!(
            "이 앱에서 읽을 수 없는 대화 기록 형식입니다 (형식 {version})"
        ));
    }
    connection
        .execute_batch(
            "BEGIN IMMEDIATE;
             CREATE TABLE assistant_sessions (
                id TEXT PRIMARY KEY,
                scope_kind TEXT NOT NULL CHECK(scope_kind IN ('folder', 'docker')),
                scope_root TEXT NOT NULL,
                scope_name TEXT NOT NULL,
                folder_summary_json TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL,
                last_provider TEXT,
                last_model TEXT
             );
             CREATE TABLE assistant_messages (
                session_id TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                content TEXT NOT NULL,
                provider TEXT,
                model TEXT,
                created_at_unix_ms INTEGER NOT NULL,
                PRIMARY KEY(session_id, sequence),
                FOREIGN KEY(session_id) REFERENCES assistant_sessions(id) ON DELETE CASCADE
             );
             CREATE INDEX assistant_sessions_updated_idx
                ON assistant_sessions(updated_at_unix_ms DESC);
             PRAGMA user_version = 2;
             COMMIT;",
        )
        .map_err(database_error)
}

fn validate_new_session(request: &CreateAssistantSessionRequest) -> Result<(), String> {
    let root = request.scope_root.trim();
    if root.is_empty() || root.chars().count() > MAX_SCOPE_ROOT_CHARS {
        return Err("대화할 폴더 경로가 올바르지 않습니다".to_owned());
    }
    let summary = &request.folder_summary;
    if summary.scope_name.trim().is_empty()
        || summary.scope_name.chars().count() > MAX_SCOPE_NAME_CHARS
    {
        return Err("대화할 폴더 이름이 올바르지 않습니다".to_owned());
    }
    if summary.children.len() > MAX_FOLDER_CHILDREN {
        return Err(format!(
            "저장할 폴더 요약은 {MAX_FOLDER_CHILDREN}개 항목 이하여야 합니다"
        ));
    }
    if summary.children.iter().any(|child| {
        child.name.trim().is_empty() || child.name.chars().count() > MAX_SCOPE_NAME_CHARS
    }) {
        return Err("폴더 요약의 항목 이름이 올바르지 않습니다".to_owned());
    }
    match request.scope_kind {
        AssistantScopeKind::Folder => {
            if root == "docker://local" {
                return Err("폴더 대화 경로가 올바르지 않습니다".to_owned());
            }
        }
        AssistantScopeKind::Docker => {
            if root != "docker://local"
                || summary.scope_name != "Docker"
                || !summary.children.is_empty()
            {
                return Err("Docker 대화 범위가 올바르지 않습니다".to_owned());
            }
        }
    }
    Ok(())
}

fn validate_message(request: &AppendAssistantMessageRequest) -> Result<(), String> {
    validate_session_id(&request.session_id)?;
    let content = request.content.trim();
    let max_chars = match request.role {
        AssistantChatRole::User => MAX_USER_MESSAGE_CHARS,
        AssistantChatRole::Assistant => MAX_ASSISTANT_MESSAGE_CHARS,
    };
    if content.is_empty() || content.chars().count() > max_chars {
        return Err(format!(
            "저장할 메시지는 1자 이상 {max_chars}자 이하여야 합니다"
        ));
    }
    match (request.role, request.provider) {
        (AssistantChatRole::User, None) => {
            if request.model.is_some() {
                return Err("사용자 메시지에는 AI 모델을 저장할 수 없습니다".to_owned());
            }
        }
        (AssistantChatRole::Assistant, Some(_)) => {}
        (AssistantChatRole::User, Some(_)) => {
            return Err("사용자 메시지에는 AI 공급자를 저장할 수 없습니다".to_owned());
        }
        (AssistantChatRole::Assistant, None) => {
            return Err("AI 응답에는 사용한 공급자가 필요합니다".to_owned());
        }
    }
    if request.model.as_ref().is_some_and(|model| {
        model.trim().is_empty()
            || model.chars().count() > MAX_MODEL_NAME_CHARS
            || model.chars().any(char::is_control)
    }) {
        return Err("AI 모델 이름이 올바르지 않습니다".to_owned());
    }
    Ok(())
}

fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty()
        || session_id.chars().count() > MAX_SESSION_ID_CHARS
        || !session_id
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-')
    {
        return Err("대화 번호가 올바르지 않습니다".to_owned());
    }
    Ok(())
}

fn generate_session_id() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("새 대화 번호를 만들지 못했습니다: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn unix_time_ms() -> Result<i64, String> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("현재 시각을 읽지 못했습니다: {error}"))?
        .as_millis();
    i64::try_from(milliseconds).map_err(|_| "현재 시각 값이 너무 큽니다".to_owned())
}

fn provider_wire_name(provider: AssistantProviderKind) -> &'static str {
    match provider {
        AssistantProviderKind::Codex => "codex",
        AssistantProviderKind::ClaudeCode => "claudeCode",
        AssistantProviderKind::Grok => "grok",
        AssistantProviderKind::Antigravity => "antigravity",
        AssistantProviderKind::Ollama => "ollama",
    }
}

fn scope_wire_name(scope: AssistantScopeKind) -> &'static str {
    match scope {
        AssistantScopeKind::Folder => "folder",
        AssistantScopeKind::Docker => "docker",
    }
}

fn parse_scope(value: &str) -> Result<AssistantScopeKind, String> {
    match value {
        "folder" => Ok(AssistantScopeKind::Folder),
        "docker" => Ok(AssistantScopeKind::Docker),
        _ => Err(format!("알 수 없는 대화 범위입니다: {value}")),
    }
}

fn parse_provider(value: &str) -> Result<AssistantProviderKind, String> {
    match value {
        "codex" => Ok(AssistantProviderKind::Codex),
        "claudeCode" => Ok(AssistantProviderKind::ClaudeCode),
        "grok" => Ok(AssistantProviderKind::Grok),
        "antigravity" => Ok(AssistantProviderKind::Antigravity),
        "ollama" => Ok(AssistantProviderKind::Ollama),
        _ => Err(format!("알 수 없는 AI 공급자입니다: {value}")),
    }
}

fn role_wire_name(role: AssistantChatRole) -> &'static str {
    match role {
        AssistantChatRole::User => "user",
        AssistantChatRole::Assistant => "assistant",
    }
}

fn parse_role(value: &str) -> Result<AssistantChatRole, String> {
    match value {
        "user" => Ok(AssistantChatRole::User),
        "assistant" => Ok(AssistantChatRole::Assistant),
        _ => Err(format!("알 수 없는 대화 역할입니다: {value}")),
    }
}

fn non_negative_u64(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| conversion_error(column, "음수 값".to_owned()))
}

fn conversion_error(column: usize, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Integer,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}

fn database_error(error: rusqlite::Error) -> String {
    format!("대화 기록 데이터베이스 오류: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant_provider::{AssistantFolderChild, AssistantFolderChildKind};

    fn folder_summary(name: &str) -> AssistantFolderSummary {
        AssistantFolderSummary {
            scope_name: name.to_owned(),
            completed_at_unix_ms: 1_780_000_000_000,
            total_logical_bytes: 12_345,
            total_files: 3,
            total_directories: 1,
            unreadable_entries: 0,
            empty_directory_count: 0,
            children_truncated: false,
            children: vec![AssistantFolderChild {
                name: "src".to_owned(),
                kind: AssistantFolderChildKind::Directory,
                logical_bytes: 12_345,
                file_count: 3,
                directory_count: 1,
            }],
        }
    }

    #[test]
    fn session_history_survives_provider_changes_and_delete_cascades() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let database = temporary.path().join("assistant.sqlite3");
        let scope = temporary.path().join("scope");
        fs::create_dir_all(&scope).expect("scope directory");
        let user_file = scope.join("keep.txt");
        fs::write(&user_file, "keep").expect("scope file");

        let detail = create_session_at(
            &database,
            CreateAssistantSessionRequest {
                scope_kind: AssistantScopeKind::Folder,
                scope_root: scope.to_string_lossy().into_owned(),
                folder_summary: folder_summary("scope"),
            },
        )
        .expect("create session");
        let session_id = detail.session.id.clone();

        append_message_at(
            &database,
            AppendAssistantMessageRequest {
                session_id: session_id.clone(),
                role: AssistantChatRole::User,
                content: "어디가 커?".to_owned(),
                provider: None,
                model: None,
            },
        )
        .expect("append user message");
        append_message_at(
            &database,
            AppendAssistantMessageRequest {
                session_id: session_id.clone(),
                role: AssistantChatRole::Assistant,
                content: "src 폴더를 확인하세요.".to_owned(),
                provider: Some(AssistantProviderKind::ClaudeCode),
                model: None,
            },
        )
        .expect("append Claude response");
        append_message_at(
            &database,
            AppendAssistantMessageRequest {
                session_id: session_id.clone(),
                role: AssistantChatRole::User,
                content: "다시 설명해줘".to_owned(),
                provider: None,
                model: None,
            },
        )
        .expect("append follow-up");
        append_message_at(
            &database,
            AppendAssistantMessageRequest {
                session_id: session_id.clone(),
                role: AssistantChatRole::Assistant,
                content: "가장 큰 항목부터 보세요.".to_owned(),
                provider: Some(AssistantProviderKind::Ollama),
                model: Some("gemma-test".to_owned()),
            },
        )
        .expect("append Ollama response");

        let restored = get_session_at(&database, &session_id).expect("restore session");
        assert_eq!(restored.messages.len(), 4);
        assert_eq!(
            restored.session.last_provider,
            Some(AssistantProviderKind::Ollama)
        );
        assert_eq!(restored.session.last_model.as_deref(), Some("gemma-test"));
        assert_eq!(
            list_sessions_at(&database).expect("list")[0].message_count,
            4
        );

        assert!(delete_session_at(&database, &session_id).expect("delete session"));
        assert!(
            list_sessions_at(&database)
                .expect("list after delete")
                .is_empty()
        );
        assert!(
            user_file.exists(),
            "deleting chat must not touch scoped files"
        );
        let connection = open_database(&database).expect("open database");
        let messages = connection
            .query_row("SELECT COUNT(*) FROM assistant_messages", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("message count");
        assert_eq!(messages, 0);
    }

    #[test]
    fn invalid_message_metadata_is_rejected_before_database_write() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let database = temporary.path().join("assistant.sqlite3");
        let detail = create_session_at(
            &database,
            CreateAssistantSessionRequest {
                scope_kind: AssistantScopeKind::Folder,
                scope_root: temporary.path().to_string_lossy().into_owned(),
                folder_summary: folder_summary("fixture"),
            },
        )
        .expect("create session");
        let error = append_message_at(
            &database,
            AppendAssistantMessageRequest {
                session_id: detail.session.id,
                role: AssistantChatRole::User,
                content: "질문".to_owned(),
                provider: Some(AssistantProviderKind::Codex),
                model: None,
            },
        )
        .expect_err("user provider must be rejected");
        assert!(error.contains("사용자 메시지"));
    }

    #[test]
    fn docker_session_persists_as_a_non_folder_scope() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let database = temporary.path().join("assistant.sqlite3");
        let mut summary = folder_summary("Docker");
        summary.children.clear();
        summary.total_logical_bytes = 0;
        summary.total_files = 0;
        summary.total_directories = 0;

        let detail = create_session_at(
            &database,
            CreateAssistantSessionRequest {
                scope_kind: AssistantScopeKind::Docker,
                scope_root: "docker://local".to_owned(),
                folder_summary: summary,
            },
        )
        .expect("create Docker session");

        assert_eq!(detail.session.scope_kind, AssistantScopeKind::Docker);
        assert_eq!(detail.session.scope_root, "docker://local");
        assert!(detail.folder_summary.children.is_empty());
    }

    #[test]
    fn version_one_sessions_migrate_to_folder_scope() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let database = temporary.path().join("assistant.sqlite3");
        let connection = Connection::open(&database).expect("open v1 database");
        connection
            .execute_batch(
                "CREATE TABLE assistant_sessions (
                    id TEXT PRIMARY KEY,
                    scope_root TEXT NOT NULL,
                    scope_name TEXT NOT NULL,
                    folder_summary_json TEXT NOT NULL,
                    created_at_unix_ms INTEGER NOT NULL,
                    updated_at_unix_ms INTEGER NOT NULL,
                    last_provider TEXT,
                    last_model TEXT
                 );
                 CREATE TABLE assistant_messages (
                    session_id TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                    content TEXT NOT NULL,
                    provider TEXT,
                    model TEXT,
                    created_at_unix_ms INTEGER NOT NULL,
                    PRIMARY KEY(session_id, sequence),
                    FOREIGN KEY(session_id) REFERENCES assistant_sessions(id) ON DELETE CASCADE
                 );
                 CREATE INDEX assistant_sessions_updated_idx
                    ON assistant_sessions(updated_at_unix_ms DESC);
                 PRAGMA user_version = 1;",
            )
            .expect("create v1 schema");
        drop(connection);

        let migrated = open_database(&database).expect("migrate database");
        let version = migrated
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .expect("schema version");
        let default_scope = migrated
            .query_row(
                "SELECT dflt_value FROM pragma_table_info('assistant_sessions') WHERE name = 'scope_kind'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("scope default");
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(default_scope, "'folder'");
    }

    #[test]
    fn database_uses_foreign_keys_and_secure_delete() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let database = temporary.path().join("assistant.sqlite3");
        let connection = open_database(&database).expect("open database");
        let foreign_keys = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("foreign keys pragma");
        let secure_delete = connection
            .query_row("PRAGMA secure_delete", [], |row| row.get::<_, i64>(0))
            .expect("secure delete pragma");
        assert_eq!(foreign_keys, 1);
        assert_eq!(secure_delete, 1);
    }
}
