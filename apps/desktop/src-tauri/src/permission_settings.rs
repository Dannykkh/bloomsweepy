//! Opt-in grant persistence only. Plans, approvals, tokens and operation history
//! never cross a process boundary through this store.
use bloomsweepy_core::ScanConfig;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::{AppHandle, Manager};

const MAX_PAYLOAD_BYTES: usize = 32 * 1024;
const MAX_DATABASE_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PermissionLifetime {
    #[default]
    Session,
    Remember,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApprovedRoot {
    pub path: PathBuf,
    pub canonical: PathBuf,
}

impl ApprovedRoot {
    pub fn new(root: &str) -> Result<Self, String> {
        let path = PathBuf::from(root);
        if !path.is_absolute() || !path.is_dir() {
            return Err("허용할 폴더를 확인하지 못했습니다".into());
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| "허용할 폴더를 확인하지 못했습니다")?;
        Ok(Self { path, canonical })
    }

    fn is_current(&self) -> bool {
        self.path.is_absolute()
            && self.canonical.is_absolute()
            && self.path.is_dir()
            && self
                .path
                .canonicalize()
                .is_ok_and(|path| path == self.canonical)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ScanGrant {
    pub root: ApprovedRoot,
    pub config: ScanConfig,
    pub approved_at_unix_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PermissionPreferences {
    version: u32,
    pub lifetime: PermissionLifetime,
    pub inspection_allowed: bool,
    #[serde(default)]
    pub chat_trash_without_confirmation: bool,
    pub file_root: Option<ApprovedRoot>,
    pub document_root: Option<ApprovedRoot>,
    pub scan: Option<ScanGrant>,
    pub cleanup_approved_at_unix_ms: Option<u64>,
}

impl Default for PermissionPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            lifetime: PermissionLifetime::Session,
            inspection_allowed: false,
            chat_trash_without_confirmation: false,
            file_root: None,
            document_root: None,
            scan: None,
            cleanup_approved_at_unix_ms: None,
        }
    }
}

impl PermissionPreferences {
    fn persisted(&self) -> Self {
        if self.lifetime == PermissionLifetime::Remember {
            self.clone()
        } else {
            Self::default()
        }
    }

    pub fn revalidate(&mut self) -> bool {
        let mut changed = false;
        for root in [&mut self.file_root, &mut self.document_root] {
            if root.as_ref().is_some_and(|root| !root.is_current()) {
                *root = None;
                changed = true;
            }
        }
        if self.scan.as_ref().is_some_and(|scan| {
            !scan.root.is_current()
                || crate::control_server::validate_scan_config(&scan.config).is_err()
        }) {
            self.scan = None;
            changed = true;
        }
        changed
    }

    pub fn only_restricts(&self, previous: &Self) -> bool {
        let root_subset = |next: &Option<ApprovedRoot>, old: &Option<ApprovedRoot>| {
            next.as_ref().is_none_or(|next| {
                old.as_ref()
                    .is_some_and(|old| next.canonical == old.canonical)
            })
        };
        (!self.inspection_allowed || previous.inspection_allowed)
            && (!self.chat_trash_without_confirmation || previous.chat_trash_without_confirmation)
            && root_subset(&self.file_root, &previous.file_root)
            && root_subset(&self.document_root, &previous.document_root)
            && self.scan.as_ref().is_none_or(|next| {
                previous.scan.as_ref().is_some_and(|old| {
                    next.root.canonical == old.root.canonical && next.config == old.config
                })
            })
            && (self.cleanup_approved_at_unix_ms.is_none()
                || previous.cleanup_approved_at_unix_ms.is_some())
            && !(self.lifetime == PermissionLifetime::Remember
                && previous.lifetime == PermissionLifetime::Session)
    }
}

pub(crate) fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|root| root.join("permission-settings-v1.sqlite3"))
        .map_err(|_| "권한 설정 저장 위치를 찾지 못했습니다".into())
}

fn safe_database(path: &Path) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err("권한 설정 파일을 확인하지 못했습니다".into()),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_DATABASE_BYTES
    {
        return Err("권한 설정 파일이 안전한 형식이 아닙니다".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("권한 설정 파일의 접근 제한을 확인해 주세요".into());
        }
    }
    Ok(true)
}

pub(crate) fn load(path: &Path) -> Result<PermissionPreferences, String> {
    if !safe_database(path)? {
        return Ok(PermissionPreferences::default());
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "기억한 권한 설정을 읽지 못했습니다".to_owned())?;
    let payload: Option<Option<String>> = connection.query_row(
        "SELECT CASE WHEN length(CAST(payload AS BLOB)) <= ?1 THEN payload ELSE NULL END FROM permission_settings WHERE id=1",
        [MAX_PAYLOAD_BYTES as i64], |row| row.get(0)).optional()
        .map_err(|_| "기억한 권한 설정을 읽지 못했습니다".to_owned())?;
    let Some(payload) = payload else {
        return Ok(PermissionPreferences::default());
    };
    let payload = payload.ok_or("기억한 권한 설정의 크기 한도를 초과했습니다")?;
    let settings: PermissionPreferences = serde_json::from_str(&payload)
        .map_err(|_| "기억한 권한 설정의 형식이 올바르지 않습니다".to_owned())?;
    if settings.version != 1
        || (settings.lifetime == PermissionLifetime::Session
            && (settings.inspection_allowed
                || settings.chat_trash_without_confirmation
                || settings.file_root.is_some()
                || settings.document_root.is_some()
                || settings.scan.is_some()
                || settings.cleanup_approved_at_unix_ms.is_some()))
    {
        return Err("지원하지 않는 권한 설정입니다".into());
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remembered(root: &Path) -> PermissionPreferences {
        let root = ApprovedRoot::new(root.to_str().unwrap()).unwrap();
        PermissionPreferences {
            lifetime: PermissionLifetime::Remember,
            inspection_allowed: true,
            file_root: Some(root.clone()),
            document_root: Some(root.clone()),
            scan: Some(ScanGrant {
                root,
                config: ScanConfig::default(),
                approved_at_unix_ms: 123,
            }),
            cleanup_approved_at_unix_ms: Some(456),
            ..Default::default()
        }
    }

    fn replace_payload(path: &Path, payload: &str) {
        Connection::open(path)
            .unwrap()
            .execute(
                "UPDATE permission_settings SET payload=?1 WHERE id=1",
                [payload],
            )
            .unwrap();
    }

    #[test]
    fn session_grants_never_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        let mut session = remembered(dir.path());
        session.lifetime = PermissionLifetime::Session;
        assert!(
            commit(&path, &PermissionPreferences::default(), &mut session)
                .unwrap()
                .is_none()
        );
        assert!(!path.exists());
        assert!(session.inspection_allowed);
        save(&path, &session).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(restored.lifetime, PermissionLifetime::Session);
        assert!(!restored.inspection_allowed);
        assert!(
            restored.file_root.is_none()
                && restored.document_root.is_none()
                && restored.scan.is_none()
        );
        assert!(restored.cleanup_approved_at_unix_ms.is_none());
    }

    #[test]
    fn remembered_grants_round_trip_exact_scopes_and_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        let mut settings = remembered(dir.path());
        settings.scan.as_mut().unwrap().config.max_large_files = 42;
        commit(&path, &PermissionPreferences::default(), &mut settings).unwrap();
        let mut restored = load(&path).unwrap();
        assert_eq!(restored.lifetime, PermissionLifetime::Remember);
        assert!(restored.inspection_allowed);
        assert_eq!(
            restored.file_root.as_ref().unwrap().canonical,
            dir.path().canonicalize().unwrap()
        );
        assert_eq!(
            restored.document_root.as_ref().unwrap().canonical,
            settings.document_root.as_ref().unwrap().canonical
        );
        assert_eq!(
            restored.scan.as_ref().unwrap().config,
            settings.scan.as_ref().unwrap().config
        );
        assert_eq!(restored.scan.as_ref().unwrap().approved_at_unix_ms, 123);
        assert_eq!(restored.cleanup_approved_at_unix_ms, Some(456));
        assert!(!restored.revalidate());
    }

    #[test]
    fn chat_trash_permission_is_closed_by_default_opt_in_and_lifetime_bound() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        assert!(!PermissionPreferences::default().chat_trash_without_confirmation);
        let mut settings = remembered(dir.path());
        let previous = settings.clone();
        settings.chat_trash_without_confirmation = true;
        assert!(!settings.only_restricts(&previous));
        save(&path, &settings).unwrap();
        assert!(load(&path).unwrap().chat_trash_without_confirmation);
        let mut revoked = settings.clone();
        revoked.chat_trash_without_confirmation = false;
        assert!(revoked.only_restricts(&settings));
        settings.lifetime = PermissionLifetime::Session;
        save(&path, &settings).unwrap();
        assert!(!load(&path).unwrap().chat_trash_without_confirmation);
        let mut old = serde_json::to_value(previous).unwrap();
        old.as_object_mut()
            .unwrap()
            .remove("chatTrashWithoutConfirmation");
        let restored: PermissionPreferences = serde_json::from_value(old).unwrap();
        assert!(!restored.chat_trash_without_confirmation);
    }

    #[test]
    fn revocation_is_durable_and_session_mode_forgets_without_revoking_current_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        let previous = remembered(dir.path());
        save(&path, &previous).unwrap();
        let mut next = previous.clone();
        next.inspection_allowed = false;
        next.scan = None;
        commit(&path, &previous, &mut next).unwrap();
        let restored = load(&path).unwrap();
        assert!(!restored.inspection_allowed && restored.scan.is_none());
        assert!(restored.file_root.is_some());
        let mut session = next.clone();
        session.lifetime = PermissionLifetime::Session;
        commit(&path, &next, &mut session).unwrap();
        assert!(session.file_root.is_some());
        assert!(load(&path).unwrap().file_root.is_none());
    }

    #[test]
    fn switching_duration_does_not_enable_any_permission() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        let previous = PermissionPreferences::default();
        let mut next = previous.clone();
        next.lifetime = PermissionLifetime::Remember;
        commit(&path, &previous, &mut next).unwrap();
        let restored = load(&path).unwrap();
        assert!(!restored.inspection_allowed);
        assert!(
            restored.file_root.is_none()
                && restored.document_root.is_none()
                && restored.scan.is_none()
        );
        assert!(restored.cleanup_approved_at_unix_ms.is_none());
    }

    #[test]
    fn missing_scopes_are_pruned_and_cannot_resurrect_after_recreation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("approved");
        fs::create_dir(&root).unwrap();
        let path = dir.path().join("grants.sqlite3");
        let previous = remembered(&root);
        save(&path, &previous).unwrap();
        fs::remove_dir(&root).unwrap();
        let mut next = load(&path).unwrap();
        assert!(next.revalidate());
        assert!(next.file_root.is_none() && next.document_root.is_none() && next.scan.is_none());
        commit(&path, &previous, &mut next).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(load(&path).unwrap().scan.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn retargeted_symlink_is_not_the_approved_scope() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        let link = dir.path().join("link");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        symlink(&first, &link).unwrap();
        let mut settings = remembered(&link);
        assert!(!settings.revalidate());
        fs::remove_file(&link).unwrap();
        symlink(&second, &link).unwrap();
        assert!(settings.revalidate());
        assert!(
            settings.file_root.is_none()
                && settings.document_root.is_none()
                && settings.scan.is_none()
        );
    }

    #[test]
    fn invalid_scan_configuration_is_not_restored() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = remembered(dir.path());
        settings.scan.as_mut().unwrap().config.max_large_files = usize::MAX;
        assert!(settings.revalidate());
        assert!(settings.scan.is_none());
    }

    #[test]
    fn unknown_version_fields_and_session_grants_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        save(&path, &remembered(dir.path())).unwrap();
        for payload in [
            "not json".to_owned(),
            {
                let mut value = serde_json::to_value(remembered(dir.path())).unwrap();
                value["version"] = 2.into();
                value.to_string()
            },
            {
                let mut value = serde_json::to_value(remembered(dir.path())).unwrap();
                value["pendingReview"] = serde_json::json!({"approved":true});
                value.to_string()
            },
            {
                let mut value = serde_json::to_value(remembered(dir.path())).unwrap();
                value["lifetime"] = "session".into();
                value.to_string()
            },
        ] {
            replace_payload(&path, &payload);
            assert!(load(&path).is_err());
        }
    }

    #[test]
    fn serialized_grants_do_not_include_approvals_tokens_or_operations() {
        let dir = tempfile::tempdir().unwrap();
        let value = serde_json::to_value(remembered(dir.path())).unwrap();
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "chatTrashWithoutConfirmation",
                "cleanupApprovedAtUnixMs",
                "documentRoot",
                "fileRoot",
                "inspectionAllowed",
                "lifetime",
                "scan",
                "version"
            ]
        );
    }

    #[test]
    fn oversized_database_and_payload_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        save(&path, &PermissionPreferences::default()).unwrap();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        connection
            .execute(
                "UPDATE permission_settings SET payload=?1",
                ["x".repeat(MAX_PAYLOAD_BYTES + 1)],
            )
            .unwrap();
        drop(connection);
        assert!(load(&path).unwrap_err().contains("크기"));
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(MAX_DATABASE_BYTES + 1)
            .unwrap();
        assert!(load(&path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn private_database_rejects_symlinks_and_shared_file_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grants.sqlite3");
        save(&path, &PermissionPreferences::default()).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let link = dir.path().join("linked.sqlite3");
        symlink(&path, &link).unwrap();
        assert!(load(&link).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn save_failure_cannot_expand_grants_and_revocation_discards_old_grants() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.sqlite3");
        fs::write(&path, b"not sqlite").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let previous = PermissionPreferences::default();
        let mut expanded = remembered(dir.path());
        assert!(commit(&path, &previous, &mut expanded).is_err());
        assert!(!previous.inspection_allowed);
        let previous = remembered(dir.path());
        let mut restricted = previous.clone();
        restricted.inspection_allowed = false;
        assert!(commit(&path, &previous, &mut restricted).unwrap().is_some());
        assert_eq!(restricted.lifetime, PermissionLifetime::Session);
        assert!(!path.exists());
        assert!(!load(&path).unwrap().inspection_allowed);
    }

    #[test]
    fn failed_forget_reports_that_restart_may_restore_prior_grants() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("directory-not-database");
        fs::create_dir(&path).unwrap();
        let previous = remembered(dir.path());
        let mut restricted = previous.clone();
        restricted.inspection_allowed = false;
        let warning = commit(&path, &previous, &mut restricted).unwrap().unwrap();
        assert!(!restricted.inspection_allowed);
        assert!(warning.contains("복원될 수"));
        assert_eq!(restricted.lifetime, PermissionLifetime::Remember);
        // Keep retrying the durable restriction; a later Session-only mutation
        // must not clear the warning while old grants may still be on disk.
        let mut retry = restricted.clone();
        retry.lifetime = PermissionLifetime::Session;
        assert!(commit(&path, &restricted, &mut retry).unwrap().is_some());
        assert_eq!(retry.lifetime, PermissionLifetime::Remember);
    }
}

pub(crate) fn save(path: &Path, settings: &PermissionPreferences) -> Result<(), String> {
    let payload = serde_json::to_string(&settings.persisted())
        .map_err(|_| "권한 설정을 준비하지 못했습니다")?;
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err("권한 설정 크기 한도를 초과했습니다".into());
    }
    let parent = path
        .parent()
        .ok_or("권한 설정 저장 위치가 올바르지 않습니다")?;
    fs::create_dir_all(parent).map_err(|_| "권한 설정 폴더를 만들지 못했습니다")?;
    if fs::symlink_metadata(parent)
        .map_err(|_| "권한 설정 폴더를 확인하지 못했습니다")?
        .file_type()
        .is_symlink()
    {
        return Err("권한 설정 폴더는 링크일 수 없습니다".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|_| "권한 설정 폴더를 보호하지 못했습니다")?;
    }
    if !safe_database(path)? {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(path)
            .map_err(|_| "권한 설정 파일을 만들지 못했습니다")?;
    }
    let connection = Connection::open(path).map_err(|_| "권한 설정 저장소를 열지 못했습니다")?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(|_| "권한 설정 저장소를 준비하지 못했습니다")?;
    connection
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|_| "권한 설정 저널을 준비하지 못했습니다")?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|_| "권한 설정 동기화를 준비하지 못했습니다")?;
    connection.execute_batch("CREATE TABLE IF NOT EXISTS permission_settings (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 32768));")
        .map_err(|_| "권한 설정 저장소를 준비하지 못했습니다")?;
    connection.execute("INSERT INTO permission_settings(id,payload) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload", [payload])
        .map_err(|_| "권한 설정을 저장하지 못했습니다".to_owned())?;
    Ok(())
}

// On a revocation write failure, discard this dedicated grant DB rather than
// leaving yesterday's grants to silently return. No user files are removed.
pub(crate) fn forget(path: &Path) -> Result<(), String> {
    for target in [path.with_extension("sqlite3-journal"), path.to_path_buf()] {
        match fs::remove_file(target) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err("기억한 권한 설정을 지우지 못했습니다".into()),
        }
    }
    Ok(())
}

pub(crate) fn commit(
    path: &Path,
    previous: &PermissionPreferences,
    next: &mut PermissionPreferences,
) -> Result<Option<String>, String> {
    if previous.lifetime == PermissionLifetime::Session
        && next.lifetime == PermissionLifetime::Session
    {
        return Ok(None);
    }
    match save(path, next) {
        Ok(()) => Ok(None),
        Err(_error) if next.only_restricts(previous) => {
            let forgotten = forget(path).is_ok();
            next.lifetime = if forgotten {
                PermissionLifetime::Session
            } else {
                // Do not claim Session or skip future writes while an older
                // remembered grant record could still restore on next launch.
                PermissionLifetime::Remember
            };
            Ok(Some(if forgotten {
                "저장 오류로 권한 기억을 해제했습니다. 현재 허용은 이번 실행에만 적용됩니다.".into()
            } else {
                "현재 권한은 제한했지만 기억한 설정을 지우지 못했습니다. 재실행 시 이전 허용이 복원될 수 있습니다.".into()
            }))
        }
        Err(error) => Err(error),
    }
}
