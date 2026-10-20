//! Fixed, local-only Finder settings. AI inspection shares this reader, never
//! the mutation commands. A durable prepared row precedes every native write.
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[cfg(target_os = "macos")]
mod native;

const DATABASE_FILE: &str = "mac-settings-v1.sqlite3";
// minimal: preserve all original-value chains within 128 records — add explicit
// archive/export policy before users reach the limit; never silently prune Undo.
const MAX_HISTORY: usize = 128;

#[derive(Default)]
pub(crate) struct MacSettingsState {
    gate: Arc<Mutex<()>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MacSettingsSnapshot {
    pub supported: bool,
    pub platform: String,
    pub settings: Vec<MacSetting>,
    pub history: Vec<MacSettingChange>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MacSetting {
    pub id: String,
    pub title: String,
    pub description: String,
    pub domain: String,
    pub key: String,
    pub value: Option<bool>,
    pub state: String,
    pub managed: bool,
    pub can_modify: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MacSettingChange {
    pub id: String,
    pub setting_id: String,
    pub previous_value: Option<bool>,
    pub applied_value: bool,
    pub status: String,
    pub created_at_unix_ms: u64,
    pub can_restore: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApplyMacSettingRequest {
    pub setting_id: String,
    pub enabled: bool,
    #[serde(deserialize_with = "required_nullable_bool")]
    pub expected_value: Option<bool>,
}

fn required_nullable_bool<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<bool>, D::Error> {
    Option::<bool>::deserialize(deserializer)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RestoreMacSettingRequest {
    pub change_id: String,
}

#[derive(Clone, Copy)]
struct Definition {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    domain: &'static str,
    key: &'static str,
}

const DEFINITIONS: [Definition; 5] = [
    Definition {
        id: "show_extensions",
        title: "파일 확장자 표시",
        description: "Finder에서 알려진 파일 형식의 확장자도 표시합니다.",
        domain: ".GlobalPreferences",
        key: "AppleShowAllExtensions",
    },
    Definition {
        id: "show_hidden_files",
        title: "숨김 파일 표시",
        description: "Finder에서 숨김 파일을 표시합니다. 시스템 파일의 삭제를 권장하지 않습니다.",
        domain: "com.apple.finder",
        key: "AppleShowAllFiles",
    },
    Definition {
        id: "show_path_bar",
        title: "경로 막대 표시",
        description: "Finder 창 아래에 현재 폴더의 경로를 표시합니다.",
        domain: "com.apple.finder",
        key: "ShowPathbar",
    },
    Definition {
        id: "show_status_bar",
        title: "상태 막대 표시",
        description: "Finder 창 아래에 항목 수와 여유 공간을 표시합니다.",
        domain: "com.apple.finder",
        key: "ShowStatusBar",
    },
    Definition {
        id: "folders_first",
        title: "폴더를 먼저 표시",
        description: "이름순으로 정렬할 때 폴더를 파일보다 먼저 표시합니다.",
        domain: "com.apple.finder",
        key: "_FXSortFoldersFirst",
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
enum StoredValue {
    Unset,
    Bool(bool),
    TypeMismatch,
}

#[derive(Clone, Debug)]
struct Observation {
    value: StoredValue,
    managed: bool,
}

impl Observation {
    fn editable_value(&self) -> Result<Option<bool>, String> {
        if self.managed {
            return Err("조직 또는 구성 프로파일이 관리하는 설정은 변경할 수 없습니다".into());
        }
        match self.value {
            StoredValue::Unset => Ok(None),
            StoredValue::Bool(value) => Ok(Some(value)),
            StoredValue::TypeMismatch => {
                Err("저장된 설정 값이 참/거짓 형식이 아니므로 변경하지 않았습니다".into())
            }
        }
    }
}

trait PreferenceStore {
    fn platform(&self) -> String;
    fn supported(&self) -> bool;
    fn read(&self, definition: &Definition) -> Result<Observation, String>;
    fn write(&self, definition: &Definition, value: Option<bool>) -> Result<(), String>;
}

struct PlatformStore;

impl PreferenceStore for PlatformStore {
    fn platform(&self) -> String {
        #[cfg(target_os = "macos")]
        {
            native::platform()
        }
        #[cfg(not(target_os = "macos"))]
        {
            std::env::consts::OS.to_owned()
        }
    }

    fn supported(&self) -> bool {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            native::supported()
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            false
        }
    }

    fn read(&self, definition: &Definition) -> Result<Observation, String> {
        #[cfg(target_os = "macos")]
        {
            native::read(definition)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = definition;
            Err("이 운영체제에서는 Finder 설정을 지원하지 않습니다".into())
        }
    }

    fn write(&self, definition: &Definition, value: Option<bool>) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            native::write(definition, value)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (definition, value);
            Err("이 운영체제에서는 Finder 설정을 지원하지 않습니다".into())
        }
    }
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|root| root.join(DATABASE_FILE))
        .map_err(|_| "Mac 설정 기록 저장 위치를 찾지 못했습니다".to_owned())
}

fn gate(app: &AppHandle) -> Arc<Mutex<()>> {
    app.state::<MacSettingsState>().gate.clone()
}

fn trusted_local_caller(label: &str, url: &tauri::Url, development: bool) -> bool {
    if label != "main" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let local_bundle = matches!(
        (url.scheme(), url.host_str()),
        ("tauri", Some("localhost")) | ("http", Some("tauri.localhost"))
    ) && url.port().is_none();
    let local_development = development
        && url.scheme() == "http"
        && url.host_str() == Some("localhost")
        && url.port() == Some(1420);
    local_bundle || local_development
}

fn require_main(window: &WebviewWindow) -> Result<(), String> {
    let url = window
        .url()
        .map_err(|_| "앱 창의 출처를 확인하지 못했습니다")?;
    if trusted_local_caller(window.label(), &url, cfg!(debug_assertions)) {
        Ok(())
    } else {
        Err("Mac 설정 변경은 앱의 로컬 기본 창에서만 사용할 수 있습니다".into())
    }
}

#[tauri::command]
pub(crate) async fn get_mac_settings(
    app: AppHandle,
    state: State<'_, MacSettingsState>,
) -> Result<MacSettingsSnapshot, String> {
    let path = database_path(&app)?;
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Mac 설정 작업이 중단되었습니다")?;
        snapshot(&PlatformStore, &path)
    })
    .await
    .map_err(|error| format!("Mac 설정 조회 작업이 중단되었습니다: {error}"))?
}

/// Read-only common inspector. Its caller must enforce the external inspection
/// grant and run it off the UI thread, as other app-tools inspectors do.
pub(crate) async fn inspect_mac_settings(app: &AppHandle) -> Result<MacSettingsSnapshot, String> {
    let gate = gate(app);
    let path = database_path(app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Mac 설정 작업이 중단되었습니다")?;
        snapshot(&PlatformStore, &path)
    })
    .await
    .map_err(|error| format!("Mac 설정 조회 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn apply_mac_setting(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, MacSettingsState>,
    request: ApplyMacSettingRequest,
) -> Result<MacSettingsSnapshot, String> {
    require_main(&window)?;
    let path = database_path(&app)?;
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Mac 설정 작업이 중단되었습니다")?;
        apply(&PlatformStore, &path, request)?;
        snapshot(&PlatformStore, &path)
    })
    .await
    .map_err(|error| format!("Mac 설정 변경 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn restore_mac_setting(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, MacSettingsState>,
    request: RestoreMacSettingRequest,
) -> Result<MacSettingsSnapshot, String> {
    require_main(&window)?;
    let path = database_path(&app)?;
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Mac 설정 작업이 중단되었습니다")?;
        restore(&PlatformStore, &path, &request.change_id)?;
        snapshot(&PlatformStore, &path)
    })
    .await
    .map_err(|error| format!("Mac 설정 복원 작업이 중단되었습니다: {error}"))?
}

fn definition(id: &str) -> Result<&'static Definition, String> {
    DEFINITIONS
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "지원하지 않는 Mac 설정입니다".to_owned())
}

fn journal_error(error: rusqlite::Error) -> String {
    format!("Mac 설정 변경 기록을 안전하게 저장하거나 읽지 못했습니다: {error}")
}

fn open_journal(path: &Path, write: bool) -> Result<Connection, String> {
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err("Mac 설정 기록 위치가 일반 파일이 아닙니다".into());
    }
    if write {
        let parent = path
            .parent()
            .ok_or("Mac 설정 기록 위치가 올바르지 않습니다")?;
        std::fs::create_dir_all(parent).map_err(|_| "Mac 설정 기록 폴더를 만들지 못했습니다")?;
    }
    let flags = if write {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let connection = Connection::open_with_flags(path, flags).map_err(journal_error)?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(journal_error)?;
    if write {
        connection
            .execute_batch(
                "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA trusted_schema=OFF;
             CREATE TABLE IF NOT EXISTS mac_setting_changes (
               id TEXT PRIMARY KEY NOT NULL, setting_id TEXT NOT NULL,
               previous_value INTEGER CHECK(previous_value IN (0,1)),
               applied_value INTEGER NOT NULL CHECK(applied_value IN (0,1)),
               status TEXT NOT NULL, created_at_unix_ms INTEGER NOT NULL
             );",
            )
            .map_err(journal_error)?;
    }
    Ok(connection)
}

fn history(connection: &Connection) -> Result<Vec<MacSettingChange>, String> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM mac_setting_changes", [], |row| {
            row.get(0)
        })
        .map_err(journal_error)?;
    if count > MAX_HISTORY as i64 {
        return Err("Mac 설정 기록이 허용된 상한을 초과했습니다".into());
    }
    let mut statement = connection
        .prepare(
            "SELECT id, setting_id, previous_value, applied_value, status, created_at_unix_ms
         FROM mac_setting_changes ORDER BY rowid DESC LIMIT 128",
        )
        .map_err(journal_error)?;
    let changes = statement
        .query_map([], |row| {
            let timestamp: i64 = row.get(5)?;
            Ok(MacSettingChange {
                id: row.get(0)?,
                setting_id: row.get(1)?,
                previous_value: row.get(2)?,
                applied_value: row.get(3)?,
                status: row.get(4)?,
                created_at_unix_ms: u64::try_from(timestamp)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, timestamp))?,
                can_restore: false,
            })
        })
        .map_err(journal_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(journal_error)?;
    for change in &changes {
        definition(&change.setting_id)?;
        if !matches!(
            change.status.as_str(),
            "prepared" | "applied" | "failed" | "restorePrepared" | "restored" | "restoreFailed"
        ) {
            return Err("Mac 설정 변경 기록의 상태가 올바르지 않습니다".into());
        }
    }
    Ok(changes)
}

fn load_history(path: &Path) -> Result<Vec<MacSettingChange>, String> {
    if !path
        .try_exists()
        .map_err(|_| "Mac 설정 기록을 확인하지 못했습니다")?
    {
        return Ok(Vec::new());
    }
    history(&open_journal(path, false)?)
}

fn snapshot(store: &impl PreferenceStore, path: &Path) -> Result<MacSettingsSnapshot, String> {
    let supported = store.supported();
    let mut warnings = Vec::new();
    let mut settings = Vec::with_capacity(DEFINITIONS.len());
    for item in &DEFINITIONS {
        let (value, state, managed) = if !supported {
            (None, "unsupported", false)
        } else {
            match store.read(item) {
                Ok(observation) => {
                    let value = match observation.value {
                        StoredValue::Bool(value) => Some(value),
                        _ => None,
                    };
                    let state = if observation.managed {
                        "managed"
                    } else {
                        match observation.value {
                            StoredValue::Unset => "unset",
                            StoredValue::Bool(_) => "value",
                            StoredValue::TypeMismatch => "typeMismatch",
                        }
                    };
                    (value, state, observation.managed)
                }
                Err(_) => {
                    warnings.push(format!("{} 값을 읽지 못했습니다", item.title));
                    (None, "error", false)
                }
            }
        };
        settings.push(MacSetting {
            id: item.id.into(),
            title: item.title.into(),
            description: item.description.into(),
            domain: item.domain.into(),
            key: item.key.into(),
            value,
            state: state.into(),
            managed,
            can_modify: supported && matches!(state, "value" | "unset"),
        });
    }
    let mut changes = load_history(path)?;
    let mut latest = HashSet::new();
    for change in &mut changes {
        if change.status == "restored" {
            continue;
        }
        let is_latest = latest.insert(change.setting_id.clone());
        change.can_restore = is_latest
            && supported
            && settings.iter().any(|item| {
                item.id == change.setting_id
                    && item.can_modify
                    && (item.value == Some(change.applied_value)
                        || (change.status != "applied" && item.value == change.previous_value))
            });
    }
    if changes.iter().any(|item| {
        matches!(
            item.status.as_str(),
            "prepared" | "restorePrepared" | "failed" | "restoreFailed"
        )
    }) {
        warnings.push("완료를 확인하지 못한 변경 기록이 있습니다. 자동으로 재실행하지 않습니다. 현재 저장 값이 변경 값과 일치할 때만 되돌립니다. 이미 원래 값이라면 복원 선택 시 기록만 완료합니다.".into());
    }
    if supported {
        warnings.push("저장되지 않은 값은 macOS 기본값으로 추측하지 않습니다. 변경 사항은 Finder 창을 다시 열거나 Finder를 다시 시작한 뒤 반영될 수 있습니다. 앱은 Finder를 자동으로 다시 시작하지 않습니다.".into());
    } else {
        warnings.push("Finder 설정 관리는 Apple Silicon의 macOS 14·15·26에서만 지원 대상으로 제공됩니다. 다른 플랫폼이나 이후 OS에서는 변경하지 않습니다.".into());
    }
    Ok(MacSettingsSnapshot {
        supported,
        platform: store.platform(),
        settings,
        history: changes,
        warnings,
    })
}

fn new_change_id() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "Mac 설정 기록 식별자를 만들지 못했습니다")?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn now_ms() -> Result<u64, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "시스템 시각을 확인하지 못했습니다")?
        .as_millis();
    u64::try_from(millis).map_err(|_| "시스템 시각이 범위를 벗어났습니다".into())
}

fn make_room(connection: &Connection) -> Result<(), String> {
    // Never truncate an original-value rollback chain to make room. The
    // bounded first-version journal deliberately stops accepting new writes.
    if history(connection)?.len() >= MAX_HISTORY {
        return Err("Mac 설정 기록 128개를 보존하기 위해 추가 변경을 중단했습니다. 기존 변경의 복원은 계속 사용할 수 있습니다".into());
    }
    Ok(())
}

fn set_status(connection: &Connection, id: &str, status: &str) -> Result<(), String> {
    let changed = connection
        .execute(
            "UPDATE mac_setting_changes SET status=?2 WHERE id=?1",
            params![id, status],
        )
        .map_err(journal_error)?;
    if changed != 1 {
        return Err("Mac 설정 변경 기록이 사라져 작업을 중단했습니다".into());
    }
    Ok(())
}

fn execute_write(
    store: &impl PreferenceStore,
    connection: &Connection,
    item: &Definition,
    id: &str,
    expected: Option<bool>,
    target: Option<bool>,
    restoring: bool,
) -> Result<(), String> {
    let result = (|| {
        // Recheck after the durable journal commit. CFPreferences has no
        // cross-process CAS; this prevents stale UI writes, not an OS lock.
        if store.read(item)?.editable_value()? != expected {
            return Err(
                "다른 앱 또는 사용자가 설정을 변경했습니다. 새로 고침 후 다시 확인해 주세요".into(),
            );
        }
        store.write(item, target)?;
        if store.read(item)?.editable_value()? != target {
            return Err(
                "변경 후 저장 값을 확인하지 못했습니다. 성공으로 처리하지 않았습니다".into(),
            );
        }
        Ok(())
    })();
    match result {
        Ok(()) => set_status(
            connection,
            id,
            if restoring { "restored" } else { "applied" },
        ),
        Err(error) => {
            if let Err(journal_failure) = set_status(
                connection,
                id,
                if restoring { "restoreFailed" } else { "failed" },
            ) {
                return Err(format!(
                    "{error}. {journal_failure}. 미완료 기록을 보존했습니다"
                ));
            }
            Err(error)
        }
    }
}

fn apply(
    store: &impl PreferenceStore,
    path: &Path,
    request: ApplyMacSettingRequest,
) -> Result<(), String> {
    let item = definition(&request.setting_id)?;
    if !store.supported() {
        return Err("이 시스템에서는 Mac 설정 변경을 지원하지 않습니다".into());
    }
    let previous = store.read(item)?.editable_value()?;
    if previous != request.expected_value {
        return Err("화면에 표시한 뒤 설정이 바뀌었습니다. 새로 고침 후 다시 시도해 주세요".into());
    }
    if previous == Some(request.enabled) {
        return Ok(());
    }
    let connection = open_journal(path, true)?;
    if history(&connection)?.iter().any(|change| {
        change.setting_id == item.id
            && matches!(change.status.as_str(), "prepared" | "restorePrepared")
    }) {
        return Err("이 설정에 완료를 확인하지 못한 기록이 있습니다. 먼저 현재 상태와 복원 가능 여부를 확인해 주세요".into());
    }
    make_room(&connection)?;
    let id = new_change_id()?;
    let timestamp =
        i64::try_from(now_ms()?).map_err(|_| "시스템 시각이 저장 가능한 범위를 벗어났습니다")?;
    connection.execute(
        "INSERT INTO mac_setting_changes(id,setting_id,previous_value,applied_value,status,created_at_unix_ms) VALUES(?1,?2,?3,?4,'prepared',?5)",
        params![id, item.id, previous, request.enabled, timestamp]
    ).map_err(journal_error)?;
    execute_write(
        store,
        &connection,
        item,
        &id,
        previous,
        Some(request.enabled),
        false,
    )
}

fn restore(store: &impl PreferenceStore, path: &Path, id: &str) -> Result<(), String> {
    if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Mac 설정 복원 식별자가 올바르지 않습니다".into());
    }
    if !store.supported() {
        return Err("이 시스템에서는 Mac 설정 복원을 지원하지 않습니다".into());
    }
    let connection = open_journal(path, false)?;
    let changes = history(&connection)?;
    let change = changes
        .iter()
        .find(|change| change.id == id)
        .ok_or("Mac 설정 변경 기록을 찾지 못했습니다")?;
    if change.status == "restored" {
        return Err("이미 복원한 변경 기록입니다".into());
    }
    if changes
        .iter()
        .find(|candidate| {
            candidate.setting_id == change.setting_id && candidate.status != "restored"
        })
        .is_none_or(|latest| latest.id != change.id)
    {
        return Err("이 설정의 더 최근 변경을 먼저 복원해 주세요".into());
    }
    let item = definition(&change.setting_id)?;
    let current = store.read(item)?.editable_value()?;
    let original_already_present = change.status != "applied" && current == change.previous_value;
    if current != Some(change.applied_value) && !original_already_present {
        return Err(
            "현재 값이 앱에서 변경한 값과 다릅니다. 사용자의 이후 변경을 덮어쓰지 않았습니다"
                .into(),
        );
    }
    // Open writable only after all conflict checks; preparing is its own FULL
    // synchronous commit and survives a crash before/after the native write.
    drop(connection);
    let connection = open_journal(path, true)?;
    if original_already_present {
        // Explicit recovery after a crash before the initial native write, or
        // after the restore write but before its final journal commit. Never
        // replay a write, and do not strand the setting on an unfinished row.
        if store.read(item)?.editable_value()? != change.previous_value {
            return Err("현재 설정이 다시 변경되어 복원 기록을 완료하지 않았습니다".into());
        }
        return set_status(&connection, id, "restored");
    }
    set_status(&connection, id, "restorePrepared")?;
    execute_write(
        store,
        &connection,
        item,
        id,
        Some(change.applied_value),
        change.previous_value,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct FakeStore {
        value: RefCell<Observation>,
        writes: Cell<usize>,
        fail_write: Cell<bool>,
        ignore_write: Cell<bool>,
        fail_read: Cell<bool>,
        fail_read_after_write: Cell<bool>,
        supported: bool,
    }
    impl FakeStore {
        fn new(value: StoredValue) -> Self {
            Self {
                value: RefCell::new(Observation {
                    value,
                    managed: false,
                }),
                writes: Cell::new(0),
                fail_write: Cell::new(false),
                ignore_write: Cell::new(false),
                fail_read: Cell::new(false),
                fail_read_after_write: Cell::new(false),
                supported: true,
            }
        }
    }
    impl PreferenceStore for FakeStore {
        fn platform(&self) -> String {
            "test".into()
        }
        fn supported(&self) -> bool {
            self.supported
        }
        fn read(&self, _: &Definition) -> Result<Observation, String> {
            if self.fail_read.get() {
                return Err("read failed".into());
            }
            Ok(self.value.borrow().clone())
        }
        fn write(&self, _: &Definition, value: Option<bool>) -> Result<(), String> {
            self.writes.set(self.writes.get() + 1);
            if self.fail_write.get() {
                return Err("write failed".into());
            }
            if !self.ignore_write.get() {
                self.value.borrow_mut().value = value.map_or(StoredValue::Unset, StoredValue::Bool);
            }
            if self.fail_read_after_write.get() {
                self.fail_read.set(true);
            }
            Ok(())
        }
    }
    fn request(expected: Option<bool>, enabled: bool) -> ApplyMacSettingRequest {
        ApplyMacSettingRequest {
            setting_id: "show_path_bar".into(),
            enabled,
            expected_value: expected,
        }
    }
    #[test]
    fn applies_and_restores_absent_not_false_and_rejects_duplicate_restore() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);
        apply(&store, &path, request(None, true)).unwrap();
        let snapshot = snapshot(&store, &path).unwrap();
        assert_eq!(snapshot.history.len(), 1);
        assert_eq!(snapshot.history[0].status, "applied");
        assert_eq!(snapshot.history[0].previous_value, None);
        assert!(snapshot.history[0].can_restore);
        restore(&store, &path, &snapshot.history[0].id).unwrap();
        assert_eq!(store.value.borrow().value, StoredValue::Unset);
        assert!(restore(&store, &path, &snapshot.history[0].id).is_err());
        assert_eq!(store.writes.get(), 2);
    }
    #[test]
    fn restores_multistep_chain_newest_first_and_noops_or_stale_requests_never_write() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);

        apply(&store, &path, request(None, true)).unwrap();
        let first = snapshot(&store, &path).unwrap().history[0].id.clone();
        assert_eq!(store.writes.get(), 1);

        apply(&store, &path, request(Some(true), true)).unwrap();
        assert!(apply(&store, &path, request(None, false)).is_err());
        assert_eq!(store.writes.get(), 1);
        assert_eq!(load_history(&path).unwrap().len(), 1);

        apply(&store, &path, request(Some(true), false)).unwrap();
        // Wall clock can move backward. The newest insertion, not timestamp,
        // must stay at the head of the rollback chain.
        let newest_id = load_history(&path).unwrap()[0].id.clone();
        open_journal(&path, true)
            .unwrap()
            .execute(
                "UPDATE mac_setting_changes SET created_at_unix_ms=0 WHERE id=?1",
                [&newest_id],
            )
            .unwrap();
        let after_second = snapshot(&store, &path).unwrap();
        let second = after_second.history[0].id.clone();
        assert_ne!(first, second);
        assert_eq!(after_second.history[0].previous_value, Some(true));
        assert!(after_second.history[0].can_restore);
        assert!(
            !after_second
                .history
                .iter()
                .find(|change| change.id == first)
                .unwrap()
                .can_restore
        );
        assert!(restore(&store, &path, &first).is_err());
        assert_eq!(store.writes.get(), 2);
        assert_eq!(store.value.borrow().value, StoredValue::Bool(false));

        restore(&store, &path, &second).unwrap();
        assert_eq!(store.value.borrow().value, StoredValue::Bool(true));
        assert_eq!(store.writes.get(), 3);
        let after_restoring_second = snapshot(&store, &path).unwrap();
        assert!(
            after_restoring_second
                .history
                .iter()
                .find(|change| change.id == first)
                .unwrap()
                .can_restore
        );
        assert!(restore(&store, &path, &second).is_err());
        assert_eq!(store.writes.get(), 3);

        restore(&store, &path, &first).unwrap();
        assert_eq!(store.value.borrow().value, StoredValue::Unset);
        assert_eq!(store.writes.get(), 4);
        assert!(restore(&store, &path, &first).is_err());
        assert!(restore(&store, &path, &second).is_err());
        assert_eq!(store.writes.get(), 4);
        let completed = snapshot(&store, &path).unwrap();
        assert_eq!(completed.history.len(), 2);
        assert!(
            completed
                .history
                .iter()
                .all(|change| change.status == "restored" && !change.can_restore)
        );
    }
    #[test]
    fn preserves_false_original_and_rejects_external_change() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Bool(false));
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert!(!path.exists());
        apply(&store, &path, request(Some(false), true)).unwrap();
        let id = snapshot(&store, &path).unwrap().history[0].id.clone();
        store.value.borrow_mut().value = StoredValue::Bool(false);
        assert!(restore(&store, &path, &id).is_err());
        assert_eq!(store.writes.get(), 1);
        store.value.borrow_mut().value = StoredValue::Bool(true);
        restore(&store, &path, &id).unwrap();
        assert_eq!(store.value.borrow().value, StoredValue::Bool(false));
    }
    #[test]
    fn managed_wrong_type_unknown_id_read_error_and_unsupported_never_write() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);
        store.value.borrow_mut().managed = true;
        assert!(apply(&store, &path, request(None, true)).is_err());
        store.value.borrow_mut().managed = false;
        store.value.borrow_mut().value = StoredValue::TypeMismatch;
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert_eq!(
            snapshot(&store, &path).unwrap().settings[0].state,
            "typeMismatch"
        );
        store.fail_read.set(true);
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert_eq!(snapshot(&store, &path).unwrap().settings[0].state, "error");
        let mut invalid = request(None, true);
        invalid.setting_id = "com.apple.finder; rm".into();
        assert!(apply(&store, &path, invalid).is_err());
        assert_eq!(store.writes.get(), 0);
        let mut unsupported = FakeStore::new(StoredValue::Unset);
        unsupported.supported = false;
        assert!(!snapshot(&unsupported, &path).unwrap().supported);
        assert!(apply(&unsupported, &path, request(None, true)).is_err());
    }
    #[test]
    fn mutation_requests_reject_unknown_fields_and_invalid_types() {
        assert!(serde_json::from_value::<ApplyMacSettingRequest>(serde_json::json!({"settingId":"show_path_bar","enabled":true,"expectedValue":null,"domain":"evil"})).is_err());
        assert!(serde_json::from_value::<ApplyMacSettingRequest>(serde_json::json!({"settingId":"show_path_bar","enabled":"true","expectedValue":null})).is_err());
        assert!(
            serde_json::from_value::<ApplyMacSettingRequest>(
                serde_json::json!({"settingId":"show_path_bar","enabled":true})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<RestoreMacSettingRequest>(
                serde_json::json!({"changeId":"x","force":true})
            )
            .is_err()
        );
    }
    #[test]
    fn only_local_main_window_can_mutate_not_remote_secondary_or_external_tools() {
        for url in [
            "https://example.com/",
            "http://localhost:1420/",
            "tauri://evil/",
            "http://tauri.localhost.example.com/",
            "file:///tmp/index.html",
            "tauri://user@localhost/",
        ] {
            assert!(
                !trusted_local_caller("main", &tauri::Url::parse(url).unwrap(), false),
                "{url}"
            );
        }
        let local = tauri::Url::parse("tauri://localhost/index.html").unwrap();
        assert!(trusted_local_caller("main", &local, false));
        assert!(!trusted_local_caller("mcp", &local, false));
        assert!(!trusted_local_caller("inspector", &local, false));
        let dev = tauri::Url::parse("http://localhost:1420/").unwrap();
        assert!(trusted_local_caller("main", &dev, true));
    }
    #[test]
    fn unavailable_journal_prevents_write() {
        let temp = tempfile::tempdir().unwrap();
        let store = FakeStore::new(StoredValue::Unset);
        assert!(apply(&store, temp.path(), request(None, true)).is_err());
        assert_eq!(store.writes.get(), 0);
    }
    #[test]
    fn write_and_readback_failures_remain_failed_not_successful() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);
        store.fail_write.set(true);
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert_eq!(load_history(&path).unwrap()[0].status, "failed");
        store.fail_write.set(false);
        store.ignore_write.set(true);
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert!(
            load_history(&path)
                .unwrap()
                .iter()
                .all(|item| item.status == "failed")
        );
    }
    #[test]
    fn failed_readback_keeps_original_and_does_not_claim_success_after_native_write() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);
        store.fail_read_after_write.set(true);
        assert!(apply(&store, &path, request(None, true)).is_err());
        let changes = load_history(&path).unwrap();
        assert_eq!(changes[0].status, "failed");
        assert_eq!(changes[0].previous_value, None);
        let error_snapshot = snapshot(&store, &path).unwrap();
        assert!(
            error_snapshot
                .settings
                .iter()
                .all(|item| item.state == "error" && !item.can_modify)
        );
        assert_eq!(store.writes.get(), 1);
        store.fail_read.set(false);
        assert!(snapshot(&store, &path).unwrap().history[0].can_restore);
        assert_eq!(store.writes.get(), 1);
    }
    #[test]
    fn prepared_crash_record_is_not_replayed_but_matching_write_can_be_restored() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let connection = open_journal(&path, true).unwrap();
        let id = "00112233445566778899aabbccddeeff";
        connection
            .execute(
                "INSERT INTO mac_setting_changes VALUES(?1,'show_path_bar',NULL,1,'prepared',1)",
                [id],
            )
            .unwrap();
        drop(connection);
        let store = FakeStore::new(StoredValue::Unset);
        assert!(snapshot(&store, &path).unwrap().history[0].can_restore);
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert_eq!(store.writes.get(), 0);
        store.value.borrow_mut().value = StoredValue::Bool(true);
        assert!(snapshot(&store, &path).unwrap().history[0].can_restore);
        assert_eq!(store.writes.get(), 0);
        restore(&store, &path, id).unwrap();
        assert_eq!(store.value.borrow().value, StoredValue::Unset);
    }
    #[test]
    fn prepared_commit_is_visible_before_write_and_final_commit_failure_stays_pending() {
        struct DurableProbe {
            store: FakeStore,
            path: PathBuf,
        }
        impl PreferenceStore for DurableProbe {
            fn platform(&self) -> String {
                "test".into()
            }
            fn supported(&self) -> bool {
                true
            }
            fn read(&self, definition: &Definition) -> Result<Observation, String> {
                self.store.read(definition)
            }
            fn write(&self, definition: &Definition, value: Option<bool>) -> Result<(), String> {
                // A separate SQLite connection must see the committed row.
                let rows = load_history(&self.path).unwrap();
                assert_eq!(rows[0].status, "prepared");
                let connection = open_journal(&self.path, true).unwrap();
                connection.execute_batch("CREATE TRIGGER reject_finish BEFORE UPDATE OF status ON mac_setting_changes WHEN NEW.status='applied' BEGIN SELECT RAISE(FAIL,'synthetic disk failure'); END;").unwrap();
                self.store.write(definition, value)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = DurableProbe {
            store: FakeStore::new(StoredValue::Unset),
            path: path.clone(),
        };
        assert!(apply(&store, &path, request(None, true)).is_err());
        assert_eq!(store.store.writes.get(), 1);
        let observed = snapshot(&store, &path).unwrap();
        assert_eq!(observed.history[0].status, "prepared");
        assert!(observed.history[0].can_restore);
        assert_eq!(store.store.writes.get(), 1);
    }
    #[test]
    fn restore_effect_success_then_final_commit_failure_recovers_without_replaying_write() {
        struct RestoreCommitProbe<'a> {
            store: &'a FakeStore,
            path: &'a Path,
        }
        impl PreferenceStore for RestoreCommitProbe<'_> {
            fn platform(&self) -> String {
                self.store.platform()
            }
            fn supported(&self) -> bool {
                self.store.supported()
            }
            fn read(&self, definition: &Definition) -> Result<Observation, String> {
                self.store.read(definition)
            }
            fn write(&self, definition: &Definition, value: Option<bool>) -> Result<(), String> {
                // A separate connection proves the real restore() committed
                // its preparation before the simulated native effect.
                let durable = load_history(self.path).unwrap();
                assert_eq!(durable[0].status, "restorePrepared");
                assert_eq!(value, None, "restore must remove the original override");
                self.store.write(definition, value)
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);
        apply(&store, &path, request(None, true)).unwrap();
        let id = load_history(&path).unwrap()[0].id.clone();
        assert_eq!(store.writes.get(), 1);

        // Permit restorePrepared and the native operation, but fail only the
        // final restored update. No Finder preference or OS Trash is involved.
        open_journal(&path, true).unwrap().execute_batch(
            "CREATE TRIGGER reject_restore_finish BEFORE UPDATE OF status ON mac_setting_changes
             WHEN NEW.status='restored'
             BEGIN SELECT RAISE(FAIL,'synthetic restore commit failure'); END;"
        ).unwrap();
        let probe = RestoreCommitProbe {
            store: &store,
            path: &path,
        };
        let failure = restore(&probe, &path, &id).unwrap_err();
        assert!(failure.contains("synthetic restore commit failure"));
        assert_eq!(store.value.borrow().value, StoredValue::Unset);
        assert_eq!(
            store.writes.get(),
            2,
            "the restore native effect happened exactly once"
        );
        let pending = load_history(&path).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].status, "restorePrepared");
        assert_eq!(pending[0].previous_value, None);
        assert!(pending[0].applied_value);

        // Model a new process with only the actual stored value and durable
        // journal. Inspection must not replay or silently finish the operation.
        let restarted = FakeStore::new(StoredValue::Unset);
        let recovered = snapshot(&restarted, &path).unwrap();
        assert_eq!(recovered.history[0].status, "restorePrepared");
        assert!(recovered.history[0].can_restore);
        assert_eq!(restarted.writes.get(), 0);

        // The synthetic storage failure has cleared. Explicit recovery should
        // now finish only the journal because the original value is present.
        open_journal(&path, true)
            .unwrap()
            .execute_batch("DROP TRIGGER reject_restore_finish;")
            .unwrap();
        restore(&restarted, &path, &id).unwrap();
        let completed = snapshot(&restarted, &path).unwrap();
        assert_eq!(completed.history[0].status, "restored");
        assert!(!completed.history[0].can_restore);
        assert_eq!(restarted.writes.get(), 0);
        assert_eq!(restarted.value.borrow().value, StoredValue::Unset);
        assert!(restore(&restarted, &path, &id).is_err());
        assert_eq!(restarted.writes.get(), 0);
    }
    #[test]
    fn explicit_recovery_closes_unwritten_or_finished_restore_without_replaying() {
        for status in ["prepared", "restorePrepared", "failed", "restoreFailed"] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("journal.sqlite3");
            let connection = open_journal(&path, true).unwrap();
            let id = "00112233445566778899aabbccddeeff";
            connection
                .execute(
                    "INSERT INTO mac_setting_changes VALUES(?1,'show_path_bar',NULL,1,?2,1)",
                    params![id, status],
                )
                .unwrap();
            drop(connection);
            let store = FakeStore::new(StoredValue::Unset);
            assert!(snapshot(&store, &path).unwrap().history[0].can_restore);
            restore(&store, &path, id).unwrap();
            assert_eq!(store.writes.get(), 0);
            assert_eq!(load_history(&path).unwrap()[0].status, "restored");
        }
    }
    #[test]
    fn read_only_inspection_never_creates_journal_and_history_is_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.sqlite3");
        let store = FakeStore::new(StoredValue::Unset);
        assert!(snapshot(&store, &path).unwrap().history.is_empty());
        assert!(!path.exists());
        for index in 0..128 {
            let expected = store.value.borrow().editable_value().unwrap();
            apply(&store, &path, request(expected, index % 2 == 0)).unwrap();
        }
        assert_eq!(load_history(&path).unwrap().len(), MAX_HISTORY);
        let expected = store.value.borrow().editable_value().unwrap();
        assert!(apply(&store, &path, request(expected, !expected.unwrap())).is_err());
        assert_eq!(store.writes.get(), MAX_HISTORY);
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    #[ignore = "opt-in: reads actual fixed Finder preferences, never writes"]
    fn native_catalog_readonly_smoke() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("must-not-be-created.sqlite3");
        let actual = snapshot(&PlatformStore, &path).unwrap();
        println!(
            "supported={} platform={}",
            actual.supported, actual.platform
        );
        for setting in &actual.settings {
            println!(
                "{} state={} value={:?}",
                setting.id, setting.state, setting.value
            );
        }
        assert!(actual.supported);
        assert_eq!(actual.settings.len(), 5);
        assert!(
            actual
                .settings
                .iter()
                .all(|setting| setting.state != "error")
        );
        assert!(!path.exists());
    }
}
