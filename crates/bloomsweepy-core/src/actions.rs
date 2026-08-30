use crate::{
    CleanupCandidate, DuplicateGroup, FileEntry, FileObjectIdentity, ScanError,
    file_object_identity, full_hash, system_time_ms,
};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_TRASH_SELECTION: usize = 500;

#[derive(Debug, Error)]
pub enum ActionValidationError {
    #[error("작업이 취소되었습니다")]
    Cancelled,
    #[error("잘못된 휴지통 이동 요청입니다: {0}")]
    InvalidSelection(String),
    #[error("안전하게 처리할 수 없는 경로입니다: {path} ({reason})")]
    UnsafePath { path: String, reason: String },
    #[error("스캔 후 항목이 변경되었습니다. 다시 스캔하세요: {0}")]
    Changed(String),
    #[error("항목을 재검사하지 못했습니다: {path} ({message})")]
    Access { path: String, message: String },
}

#[derive(Debug, Clone)]
pub struct VerifiedTrashItem {
    path: PathBuf,
    logical_bytes: u64,
    snapshot: VerifiedSnapshot,
    required_keeper: Option<FileSnapshot>,
}

impl VerifiedTrashItem {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn recovery_path(&self) -> &Path {
        match &self.snapshot {
            VerifiedSnapshot::DuplicateFile(snapshot) => &snapshot.canonical_path,
            VerifiedSnapshot::Cleanup(snapshot) => &snapshot.canonical_path,
        }
    }

    pub fn logical_bytes(&self) -> u64 {
        self.logical_bytes
    }
}

#[derive(Debug, Clone)]
enum VerifiedSnapshot {
    DuplicateFile(FileSnapshot),
    Cleanup(CleanupSnapshot),
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: PathBuf,
    canonical_path: PathBuf,
    canonical_root: PathBuf,
    identity: FileObjectIdentity,
    logical_bytes: u64,
    modified_at_unix_ms: Option<u128>,
    content_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupSnapshot {
    path: PathBuf,
    canonical_path: PathBuf,
    identity: FileObjectIdentity,
    is_directory: bool,
    logical_bytes: u64,
    entry_count: u64,
    latest_modified_at_unix_ms: Option<u128>,
    fingerprint: [u8; 32],
}

pub fn validate_duplicate_trash_selection<C>(
    root: impl AsRef<Path>,
    group: &DuplicateGroup,
    selected_paths: &[String],
    should_cancel: C,
) -> Result<Vec<VerifiedTrashItem>, ActionValidationError>
where
    C: Fn() -> bool,
{
    if selected_paths.is_empty() {
        return Err(ActionValidationError::InvalidSelection(
            "선택한 파일이 없습니다".to_owned(),
        ));
    }
    if selected_paths.len() > MAX_TRASH_SELECTION {
        return Err(ActionValidationError::InvalidSelection(format!(
            "한 번에 최대 {MAX_TRASH_SELECTION}개까지 처리할 수 있습니다"
        )));
    }
    if group.files.len() < 2 || selected_paths.len() >= group.files.len() {
        return Err(ActionValidationError::InvalidSelection(
            "각 중복 그룹에는 보관할 파일을 하나 이상 남겨야 합니다".to_owned(),
        ));
    }

    let canonical_root =
        fs::canonicalize(root.as_ref()).map_err(|error| access_error(root.as_ref(), error))?;
    let expected_hash = parse_hash(&group.content_hash)?;
    let selected: HashSet<&str> = selected_paths.iter().map(String::as_str).collect();
    if selected.len() != selected_paths.len() {
        return Err(ActionValidationError::InvalidSelection(
            "같은 파일이 두 번 선택되었습니다".to_owned(),
        ));
    }

    let members: std::collections::HashMap<&str, &FileEntry> = group
        .files
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    if selected.iter().any(|path| !members.contains_key(path)) {
        return Err(ActionValidationError::InvalidSelection(
            "현재 서버의 중복 결과에 없는 파일이 포함되었습니다".to_owned(),
        ));
    }

    let keeper_entry = group
        .files
        .iter()
        .find(|entry| !selected.contains(entry.path.as_str()))
        .ok_or_else(|| {
            ActionValidationError::InvalidSelection(
                "중복 그룹의 보관 파일을 찾을 수 없습니다".to_owned(),
            )
        })?;
    let keeper =
        validate_file_snapshot(keeper_entry, &canonical_root, expected_hash, &should_cancel)?;

    selected_paths
        .iter()
        .map(|path| {
            let entry = members[path.as_str()];
            let snapshot =
                validate_file_snapshot(entry, &canonical_root, expected_hash, &should_cancel)?;
            Ok(VerifiedTrashItem {
                path: snapshot.path.clone(),
                logical_bytes: snapshot.logical_bytes,
                snapshot: VerifiedSnapshot::DuplicateFile(snapshot),
                required_keeper: Some(keeper.clone()),
            })
        })
        .collect()
}

pub fn validate_cleanup_trash_candidate<C>(
    candidate: &CleanupCandidate,
    should_cancel: C,
) -> Result<VerifiedTrashItem, ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(&should_cancel)?;
    let path = PathBuf::from(&candidate.path);
    validate_cleanup_path_boundary(&path)?;
    let snapshot = capture_cleanup_snapshot(&path, candidate.entry_count, &should_cancel)?;

    if snapshot.logical_bytes != candidate.logical_bytes
        || snapshot.entry_count != candidate.entry_count
        || snapshot.latest_modified_at_unix_ms != candidate.modified_at_unix_ms
    {
        return Err(ActionValidationError::Changed(candidate.path.clone()));
    }

    Ok(VerifiedTrashItem {
        path,
        logical_bytes: snapshot.logical_bytes,
        snapshot: VerifiedSnapshot::Cleanup(snapshot),
        required_keeper: None,
    })
}

pub fn revalidate_verified_trash_item<C>(
    item: &VerifiedTrashItem,
    should_cancel: C,
) -> Result<(), ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(&should_cancel)?;
    match &item.snapshot {
        VerifiedSnapshot::DuplicateFile(snapshot) => {
            revalidate_file_snapshot(snapshot, &should_cancel)?;
            if let Some(keeper) = item.required_keeper.as_ref() {
                revalidate_file_snapshot(keeper, &should_cancel)?;
            }
        }
        VerifiedSnapshot::Cleanup(expected) => {
            let actual =
                capture_cleanup_snapshot(&expected.path, expected.entry_count, &should_cancel)?;
            if &actual != expected {
                return Err(ActionValidationError::Changed(
                    expected.path.to_string_lossy().into_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_file_snapshot<C>(
    entry: &FileEntry,
    canonical_root: &Path,
    expected_hash: [u8; 32],
    should_cancel: &C,
) -> Result<FileSnapshot, ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(should_cancel)?;
    let path = PathBuf::from(&entry.path);
    if !path.is_absolute() {
        return Err(unsafe_path(&path, "절대 경로가 아닙니다"));
    }
    let metadata = safe_file_metadata(&path)?;
    let canonical_path = fs::canonicalize(&path).map_err(|error| access_error(&path, error))?;
    if !path_is_within(&canonical_path, canonical_root) {
        return Err(unsafe_path(&path, "스캔한 폴더 밖의 파일입니다"));
    }
    let identity = required_identity(&path, &metadata)?;
    if identity.links > 1 {
        return Err(unsafe_path(
            &path,
            "하드링크는 회수 가능 용량으로 안전하게 계산할 수 없습니다",
        ));
    }
    if metadata.len() != entry.logical_bytes
        || system_time_ms(metadata.modified().ok()) != entry.modified_at_unix_ms
    {
        return Err(ActionValidationError::Changed(entry.path.clone()));
    }
    let actual_hash = full_hash(&path, should_cancel).map_err(|error| map_io(&path, error))?;
    if actual_hash != expected_hash {
        return Err(ActionValidationError::Changed(entry.path.clone()));
    }

    let snapshot = FileSnapshot {
        path: path.clone(),
        canonical_path,
        canonical_root: canonical_root.to_path_buf(),
        identity,
        logical_bytes: metadata.len(),
        modified_at_unix_ms: system_time_ms(metadata.modified().ok()),
        content_hash: actual_hash,
    };
    revalidate_file_snapshot_metadata(&snapshot)?;
    Ok(snapshot)
}

fn revalidate_file_snapshot<C>(
    snapshot: &FileSnapshot,
    should_cancel: &C,
) -> Result<(), ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(should_cancel)?;
    revalidate_file_snapshot_metadata(snapshot)?;
    let hash =
        full_hash(&snapshot.path, should_cancel).map_err(|error| map_io(&snapshot.path, error))?;
    if hash != snapshot.content_hash {
        return Err(ActionValidationError::Changed(
            snapshot.path.to_string_lossy().into_owned(),
        ));
    }
    revalidate_file_snapshot_metadata(snapshot)
}

fn revalidate_file_snapshot_metadata(snapshot: &FileSnapshot) -> Result<(), ActionValidationError> {
    let metadata = safe_file_metadata(&snapshot.path)?;
    let canonical_path =
        fs::canonicalize(&snapshot.path).map_err(|error| access_error(&snapshot.path, error))?;
    let identity = required_identity(&snapshot.path, &metadata)?;
    if identity != snapshot.identity
        || identity.links > 1
        || metadata.len() != snapshot.logical_bytes
        || system_time_ms(metadata.modified().ok()) != snapshot.modified_at_unix_ms
        || !paths_equal(&canonical_path, &snapshot.canonical_path)
        || !path_is_within(&canonical_path, &snapshot.canonical_root)
    {
        return Err(ActionValidationError::Changed(
            snapshot.path.to_string_lossy().into_owned(),
        ));
    }
    Ok(())
}

fn capture_cleanup_snapshot<C>(
    path: &Path,
    expected_entry_count: u64,
    should_cancel: &C,
) -> Result<CleanupSnapshot, ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(should_cancel)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| access_error(path, error))?;
    if is_link_or_reparse_point(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(unsafe_path(path, "일반 파일 또는 폴더가 아닙니다"));
    }
    let canonical_path = fs::canonicalize(path).map_err(|error| access_error(path, error))?;
    let identity = required_identity(path, &metadata)?;
    if metadata.is_file() && identity.links > 1 {
        return Err(unsafe_path(
            path,
            "하드링크 파일은 정리 후보에서 제외됩니다",
        ));
    }

    let is_directory = metadata.is_dir();
    let mut logical_bytes = if metadata.is_file() {
        metadata.len()
    } else {
        0
    };
    let mut entry_count = if metadata.is_file() { 1_u64 } else { 0_u64 };
    let mut latest_modified_at_unix_ms = system_time_ms(metadata.modified().ok());
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bloomsweepy-cleanup-snapshot-v1\0");
    update_snapshot_hash(&mut hasher, Path::new(""), &metadata, Some(identity));

    if metadata.is_file() {
        let content_hash = full_hash(path, should_cancel).map_err(|error| map_io(path, error))?;
        hasher.update(&content_hash);
    } else {
        for item in jwalk::WalkDir::new(path)
            .follow_links(false)
            .skip_hidden(false)
            .sort(true)
            .parallelism(jwalk::Parallelism::Serial)
        {
            check_cancelled(should_cancel)?;
            let entry = item.map_err(|error| ActionValidationError::Access {
                path: error.path().unwrap_or(path).to_string_lossy().into_owned(),
                message: error.to_string(),
            })?;
            if entry.depth() == 0 {
                continue;
            }
            entry_count = entry_count.saturating_add(1);
            if entry_count > expected_entry_count {
                return Err(ActionValidationError::Changed(
                    path.to_string_lossy().into_owned(),
                ));
            }

            let entry_path = entry.path();
            let entry_metadata = fs::symlink_metadata(&entry_path)
                .map_err(|error| access_error(&entry_path, error))?;
            let relative = entry_path
                .strip_prefix(path)
                .map_err(|_| unsafe_path(&entry_path, "후보 폴더의 경계를 확인할 수 없습니다"))?;
            update_snapshot_hash(
                &mut hasher,
                relative,
                &entry_metadata,
                file_object_identity(&entry_path, &entry_metadata),
            );

            if is_link_or_reparse_point(&entry_metadata) {
                continue;
            }
            if entry_metadata.is_file() {
                logical_bytes = logical_bytes.saturating_add(entry_metadata.len());
            }
            if let Some(modified) = system_time_ms(entry_metadata.modified().ok())
                && latest_modified_at_unix_ms.is_none_or(|current| modified > current)
            {
                latest_modified_at_unix_ms = Some(modified);
            }
        }
    }

    let snapshot = CleanupSnapshot {
        path: path.to_path_buf(),
        canonical_path,
        identity,
        is_directory,
        logical_bytes,
        entry_count,
        latest_modified_at_unix_ms,
        fingerprint: *hasher.finalize().as_bytes(),
    };
    revalidate_cleanup_top_metadata(&snapshot)?;
    Ok(snapshot)
}

fn revalidate_cleanup_top_metadata(
    snapshot: &CleanupSnapshot,
) -> Result<(), ActionValidationError> {
    let metadata = fs::symlink_metadata(&snapshot.path)
        .map_err(|error| access_error(&snapshot.path, error))?;
    let canonical_path =
        fs::canonicalize(&snapshot.path).map_err(|error| access_error(&snapshot.path, error))?;
    let identity = required_identity(&snapshot.path, &metadata)?;
    if is_link_or_reparse_point(&metadata)
        || metadata.is_dir() != snapshot.is_directory
        || identity != snapshot.identity
        || !paths_equal(&canonical_path, &snapshot.canonical_path)
    {
        return Err(ActionValidationError::Changed(
            snapshot.path.to_string_lossy().into_owned(),
        ));
    }
    Ok(())
}

fn update_snapshot_hash(
    hasher: &mut blake3::Hasher,
    relative: &Path,
    metadata: &Metadata,
    identity: Option<FileObjectIdentity>,
) {
    update_os_str(hasher, relative.as_os_str());
    let kind = if is_link_or_reparse_point(metadata) {
        3_u8
    } else if metadata.is_dir() {
        2
    } else if metadata.is_file() {
        1
    } else {
        4
    };
    hasher.update(&[kind]);
    hasher.update(&metadata.len().to_le_bytes());
    hasher.update(
        &system_time_ms(metadata.modified().ok())
            .unwrap_or_default()
            .to_le_bytes(),
    );
    if let Some(identity) = identity {
        hasher.update(&[1]);
        hasher.update(&identity.identity.device.to_le_bytes());
        hasher.update(&identity.identity.index.to_le_bytes());
        hasher.update(&identity.links.to_le_bytes());
    } else {
        hasher.update(&[0]);
    }
}

#[cfg(unix)]
fn update_os_str(hasher: &mut blake3::Hasher, value: &OsStr) {
    use std::os::unix::ffi::OsStrExt;
    hasher.update(value.as_bytes());
    hasher.update(&[0]);
}

#[cfg(windows)]
fn update_os_str(hasher: &mut blake3::Hasher, value: &OsStr) {
    use std::os::windows::ffi::OsStrExt;
    for code_unit in value.encode_wide() {
        hasher.update(&code_unit.to_le_bytes());
    }
    hasher.update(&[0, 0]);
}

#[cfg(not(any(unix, windows)))]
fn update_os_str(hasher: &mut blake3::Hasher, value: &OsStr) {
    hasher.update(value.to_string_lossy().as_bytes());
    hasher.update(&[0]);
}

fn validate_cleanup_path_boundary(path: &Path) -> Result<(), ActionValidationError> {
    if !path.is_absolute() || path.file_name().is_none() || path.parent().is_none() {
        return Err(unsafe_path(
            path,
            "드라이브 루트 또는 상대 경로는 처리하지 않습니다",
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|error| access_error(path, error))?;
    let protected = [
        dirs::home_dir(),
        dirs::cache_dir(),
        dirs::config_dir(),
        dirs::data_dir(),
        dirs::data_local_dir(),
        Some(std::env::temp_dir()),
    ];
    for protected_path in protected.into_iter().flatten() {
        if let Ok(protected_path) = fs::canonicalize(protected_path)
            && paths_equal(&canonical, &protected_path)
        {
            return Err(unsafe_path(
                path,
                "사용자 또는 운영체제의 기준 폴더 자체입니다",
            ));
        }
    }
    Ok(())
}

fn safe_file_metadata(path: &Path) -> Result<Metadata, ActionValidationError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| access_error(path, error))?;
    if is_link_or_reparse_point(&metadata) || !metadata.is_file() {
        return Err(unsafe_path(path, "일반 파일이 아닙니다"));
    }
    Ok(metadata)
}

fn required_identity(
    path: &Path,
    metadata: &Metadata,
) -> Result<FileObjectIdentity, ActionValidationError> {
    file_object_identity(path, metadata).ok_or_else(|| {
        unsafe_path(
            path,
            "파일 시스템이 안정적인 항목 식별자를 제공하지 않습니다",
        )
    })
}

#[cfg(windows)]
fn is_link_or_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link_or_reparse_point(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn parse_hash(value: &str) -> Result<[u8; 32], ActionValidationError> {
    blake3::Hash::from_hex(value)
        .map(|hash| *hash.as_bytes())
        .map_err(|_| {
            ActionValidationError::InvalidSelection(
                "중복 결과의 콘텐츠 해시가 올바르지 않습니다".to_owned(),
            )
        })
}

fn check_cancelled<C>(should_cancel: &C) -> Result<(), ActionValidationError>
where
    C: Fn() -> bool,
{
    if should_cancel() {
        Err(ActionValidationError::Cancelled)
    } else {
        Ok(())
    }
}

fn map_io(path: &Path, error: io::Error) -> ActionValidationError {
    if error.kind() == io::ErrorKind::Interrupted {
        ActionValidationError::Cancelled
    } else {
        access_error(path, error)
    }
}

fn access_error(path: &Path, error: impl ToString) -> ActionValidationError {
    ActionValidationError::Access {
        path: path.to_string_lossy().into_owned(),
        message: error.to_string(),
    }
}

fn unsafe_path(path: &Path, reason: impl Into<String>) -> ActionValidationError {
    ActionValidationError::UnsafePath {
        path: path.to_string_lossy().into_owned(),
        reason: reason.into(),
    }
}

#[cfg(windows)]
fn normalized_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

#[cfg(windows)]
fn paths_equal(left: &Path, right: &Path) -> bool {
    normalized_path(left) == normalized_path(right)
}

#[cfg(windows)]
fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = normalized_path(path);
    let root = normalized_path(root);
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|rest| rest.starts_with('\\'))
}

#[cfg(not(windows))]
fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(not(windows))]
fn path_is_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

impl From<ScanError> for ActionValidationError {
    fn from(error: ScanError) -> Self {
        match error {
            ScanError::Cancelled => Self::Cancelled,
            other => Self::InvalidSelection(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CleanupCandidateKind, CleanupRootSpec, CleanupScanConfig, ScanConfig,
        scan_cleanup_candidates, scan_path,
    };
    use std::time::Duration;

    fn duplicate_report(root: &Path) -> crate::ScanReport {
        scan_path(
            root,
            ScanConfig {
                min_large_file_bytes: 1,
                min_duplicate_file_bytes: 1,
                max_large_files: 10,
                max_duplicate_groups: 10,
                max_duplicate_candidates: 100,
                max_issues: 10,
            },
            |_| {},
            || false,
        )
        .expect("scan duplicates")
    }

    #[test]
    fn duplicate_selection_always_leaves_a_verified_keeper() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let first = temp.path().join("first.bin");
        let second = temp.path().join("second.bin");
        fs::write(&first, b"same-content").expect("write first");
        fs::write(&second, b"same-content").expect("write second");
        let report = duplicate_report(temp.path());
        let group = &report.duplicate_groups[0];

        let selected = vec![group.files[0].path.clone()];
        let verified = validate_duplicate_trash_selection(&report.root, group, &selected, || false)
            .expect("validate selection");
        assert_eq!(verified.len(), 1);
        revalidate_verified_trash_item(&verified[0], || false).expect("revalidate");

        let all: Vec<String> = group.files.iter().map(|file| file.path.clone()).collect();
        assert!(validate_duplicate_trash_selection(&report.root, group, &all, || false).is_err());
    }

    #[test]
    fn duplicate_revalidation_rejects_a_changed_file() {
        let temp = tempfile::tempdir().expect("create temp directory");
        fs::write(temp.path().join("first.bin"), b"same-content").expect("write first");
        fs::write(temp.path().join("second.bin"), b"same-content").expect("write second");
        let report = duplicate_report(temp.path());
        let group = &report.duplicate_groups[0];
        let selected = vec![group.files[0].path.clone()];
        let verified = validate_duplicate_trash_selection(&report.root, group, &selected, || false)
            .expect("validate selection");

        fs::write(verified[0].path(), b"changed-and-longer").expect("change file");
        assert!(revalidate_verified_trash_item(&verified[0], || false).is_err());
    }

    #[test]
    fn cleanup_revalidation_rejects_directory_structure_changes() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let candidate_path = temp.path().join("old-cache");
        fs::create_dir(&candidate_path).expect("create candidate");
        fs::write(candidate_path.join("cache.bin"), b"cache").expect("write cache");
        let report = scan_cleanup_candidates(
            CleanupScanConfig {
                roots: vec![CleanupRootSpec::new(
                    temp.path(),
                    "test",
                    CleanupCandidateKind::TemporaryEntry,
                    Duration::ZERO,
                )],
                ..CleanupScanConfig::default()
            },
            |_| {},
            || false,
        )
        .expect("scan cleanup candidates");
        let candidate = report
            .candidates
            .iter()
            .find(|candidate| candidate.path == candidate_path.to_string_lossy())
            .expect("find candidate");
        let verified =
            validate_cleanup_trash_candidate(candidate, || false).expect("validate candidate");

        fs::write(candidate_path.join("new.bin"), b"new").expect("add file");
        assert!(revalidate_verified_trash_item(&verified, || false).is_err());
    }
}
