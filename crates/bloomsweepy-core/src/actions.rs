use crate::{
    CleanupCandidate, DuplicateGroup, FileEntry, FileObjectIdentity, ScanError,
    file_object_identity, full_hash, system_time_ms,
};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
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
            VerifiedSnapshot::EmptyDirectory(snapshot) => &snapshot.path,
            VerifiedSnapshot::Directory(snapshot) => &snapshot.path,
        }
    }

    pub fn logical_bytes(&self) -> u64 {
        self.logical_bytes
    }

    pub fn directory_counts(&self) -> Option<(u64, u64)> {
        match &self.snapshot {
            VerifiedSnapshot::Directory(snapshot) => Some((snapshot.files, snapshot.directories)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
enum VerifiedSnapshot {
    DuplicateFile(FileSnapshot),
    Cleanup(CleanupSnapshot),
    EmptyDirectory(EmptyDirectorySnapshot),
    Directory(DirectorySnapshot),
}

const FOLDER_MAX_ENTRIES: u64 = 20_000;
const FOLDER_MAX_PATH_BYTES: usize = 8 * 1024 * 1024;
const FOLDER_MAX_DEPTH: usize = 64;
const FOLDER_MAX_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectorySnapshot {
    path: PathBuf,
    root: PathBuf,
    root_identity: crate::FileIdentity,
    identity: FileObjectIdentity,
    modified: SystemTime,
    files: u64,
    directories: u64,
    logical_bytes: u64,
    fingerprint: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmptyDirectorySnapshot {
    path: PathBuf,
    root: PathBuf,
    root_identity: crate::FileIdentity,
    identity: FileObjectIdentity,
    modified: std::time::SystemTime,
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
        VerifiedSnapshot::EmptyDirectory(expected) => {
            let actual = capture_empty_directory(&expected.root, &expected.path)?;
            if &actual != expected {
                return Err(ActionValidationError::Changed(
                    expected.path.to_string_lossy().into_owned(),
                ));
            }
        }
        VerifiedSnapshot::Directory(expected) => {
            let actual =
                capture_directory_snapshot(&expected.root, &expected.path, &should_cancel)?;
            if &actual != expected {
                return Err(ActionValidationError::Changed(
                    expected.path.to_string_lossy().into_owned(),
                ));
            }
        }
    }
    Ok(())
}

/// Only server-owned scan entries may enter the shared, journalled trash pipeline.
pub fn validate_empty_directory_trash<C>(
    report: &crate::DirectoryScanReport,
    selected_path: &str,
    should_cancel: C,
) -> Result<VerifiedTrashItem, ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(&should_cancel)?;
    let entry = report
        .empty_directories
        .iter()
        .find(|entry| entry.path == selected_path)
        .ok_or_else(|| {
            ActionValidationError::InvalidSelection("검사 결과에 없는 빈 폴더입니다".to_owned())
        })?;
    let snapshot = capture_empty_directory(Path::new(&report.root), Path::new(selected_path))?;
    if entry.scan_identity != Some(snapshot.identity)
        || entry.scan_modified_at != Some(snapshot.modified)
    {
        return Err(ActionValidationError::Changed(selected_path.to_owned()));
    }
    check_cancelled(&should_cancel)?;
    Ok(VerifiedTrashItem {
        path: snapshot.path.clone(),
        logical_bytes: 0,
        snapshot: VerifiedSnapshot::EmptyDirectory(snapshot),
        required_keeper: None,
    })
}

fn capture_empty_directory(
    root: &Path,
    path: &Path,
) -> Result<EmptyDirectorySnapshot, ActionValidationError> {
    validate_directory_boundary(root, path)?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| unsafe_path(path, "검사 범위 밖입니다"))?;
    if relative
        .components()
        .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
    {
        return Err(unsafe_path(path, "숨김 폴더는 빈 폴더 정리에서 제외됩니다"));
    }
    let root_metadata = fs::symlink_metadata(root).map_err(|error| access_error(root, error))?;
    let metadata = fs::symlink_metadata(path).map_err(|error| access_error(path, error))?;
    let before = required_identity(path, &metadata)?;
    // read_dir includes dotfiles; an unreadable directory is never empty.
    if fs::read_dir(path)
        .map_err(|error| access_error(path, error))?
        .next()
        .is_some()
    {
        return Err(ActionValidationError::Changed(
            path.to_string_lossy().into_owned(),
        ));
    }
    let after = fs::symlink_metadata(path).map_err(|error| access_error(path, error))?;
    if is_link_or_reparse_point(&after)
        || !after.is_dir()
        || before != required_identity(path, &after)?
        || metadata.modified().ok() != after.modified().ok()
    {
        return Err(ActionValidationError::Changed(
            path.to_string_lossy().into_owned(),
        ));
    }
    Ok(EmptyDirectorySnapshot {
        path: path.to_owned(),
        root: root.to_owned(),
        root_identity: required_identity(root, &root_metadata)?.identity,
        identity: before,
        modified: metadata
            .modified()
            .map_err(|error| access_error(path, error))?,
    })
}

fn validate_directory_boundary(root: &Path, path: &Path) -> Result<(), ActionValidationError> {
    // Check every component before traversing: no symlink/reparse/cloud parent.
    if !path.is_absolute() || !root.is_absolute() || path == root || !path.starts_with(root) {
        return Err(unsafe_path(path, "검사 범위의 하위 폴더만 처리합니다"));
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|_| unsafe_path(path, "검사 범위 밖입니다"))?;
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(unsafe_path(
                path,
                "숨김 폴더 또는 비정상 경로는 처리하지 않습니다",
            ));
        }
    }
    let mut prefix = PathBuf::new();
    for component in path.components() {
        prefix.push(component);
        // Verbatim Windows prefixes report an implicit root, but `\\?\C:`
        // alone is not a directory. Wait for RootDir before metadata access.
        if matches!(component, std::path::Component::Prefix(_)) || !prefix.has_root() {
            continue;
        }
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        if matches!(
            name.as_str(),
            "system"
                | "library"
                | "applications"
                | "windows"
                | "appdata"
                | "program files"
                | "program files (x86)"
                | "programdata"
                | "$recycle.bin"
                | "system volume information"
                | ".trash"
                | ".trashes"
                | ".git"
        ) || [".app", ".bundle", ".framework"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
        {
            return Err(unsafe_path(path, "운영체제·앱·저장소의 보호 폴더입니다"));
        }
        crate::scan_policy::ensure_local_path(&prefix).map_err(|error| unsafe_path(path, error))?;
        let metadata =
            fs::symlink_metadata(&prefix).map_err(|error| access_error(&prefix, error))?;
        if is_link_or_reparse_point(&metadata) || !metadata.is_dir() {
            return Err(unsafe_path(path, "링크 또는 일반 폴더가 아닌 경로입니다"));
        }
    }
    #[cfg(unix)]
    for protected in [
        "/bin",
        "/sbin",
        "/usr",
        "/etc",
        "/private/etc",
        "/private/var/db",
        "/private/var/root",
    ] {
        if path.starts_with(protected) {
            return Err(unsafe_path(path, "운영체제 보호 경로입니다"));
        }
    }
    validate_cleanup_path_boundary(path)?;
    for protected in [
        dirs::desktop_dir(),
        dirs::document_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
        dirs::audio_dir(),
        dirs::video_dir(),
    ]
    .into_iter()
    .flatten()
    {
        if fs::canonicalize(protected).is_ok_and(|protected| paths_equal(path, &protected)) {
            return Err(unsafe_path(
                path,
                "사용자의 기본 폴더 자체는 처리하지 않습니다",
            ));
        }
    }
    Ok(())
}

/// Review only a direct child in the trusted current map. This captures metadata,
/// not file bodies; final validation cannot eliminate path-based OS races.
pub fn validate_directory_trash_folder<C>(
    report: &crate::DirectoryScanReport,
    selected_path: &str,
    should_cancel: C,
) -> Result<VerifiedTrashItem, ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(&should_cancel)?;
    let node = report
        .children
        .iter()
        .find(|node| node.path == selected_path && node.is_directory)
        .ok_or_else(|| {
            ActionValidationError::InvalidSelection(
                "현재 폴더 지도에 없는 폴더입니다. 다시 검사하세요".to_owned(),
            )
        })?;
    let path = Path::new(selected_path);
    if !path
        .parent()
        .is_some_and(|parent| paths_equal(parent, Path::new(&report.root)))
    {
        return Err(unsafe_path(path, "검사한 폴더의 직계 폴더가 아닙니다"));
    }
    let snapshot = capture_directory_snapshot(Path::new(&report.root), path, &should_cancel)?;
    if node.scan_identity != Some(snapshot.identity)
        || node.scan_modified_at != Some(snapshot.modified)
        || node.logical_bytes != snapshot.logical_bytes
        || node.file_count != snapshot.files
        || node.directory_count != snapshot.directories
    {
        return Err(ActionValidationError::Changed(selected_path.to_owned()));
    }
    Ok(VerifiedTrashItem {
        path: path.to_owned(),
        logical_bytes: snapshot.logical_bytes,
        snapshot: VerifiedSnapshot::Directory(snapshot),
        required_keeper: None,
    })
}

fn folder_metadata_hash(
    path: &Path,
    relative: &Path,
    metadata: &Metadata,
) -> Result<[u8; 32], ActionValidationError> {
    let identity = required_identity(path, metadata)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bloomsweepy-folder-metadata-v1\0");
    update_snapshot_hash(&mut hasher, relative, metadata, Some(identity));
    let modified = metadata
        .modified()
        .map_err(|error| access_error(path, error))?;
    let modified = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| unsafe_path(path, "수정 시각을 확인할 수 없습니다"))?;
    hasher.update(&modified.as_nanos().to_le_bytes());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        hasher.update(&metadata.ctime().to_le_bytes());
        hasher.update(&metadata.ctime_nsec().to_le_bytes());
        hasher.update(&metadata.mode().to_le_bytes());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        hasher.update(&metadata.creation_time().to_le_bytes());
        hasher.update(&metadata.file_attributes().to_le_bytes());
    }
    Ok(*hasher.finalize().as_bytes())
}

fn capture_directory_snapshot<C>(
    root: &Path,
    path: &Path,
    should_cancel: &C,
) -> Result<DirectorySnapshot, ActionValidationError>
where
    C: Fn() -> bool,
{
    let started = Instant::now();
    check_cancelled(should_cancel)?;
    crate::ensure_operation_memory().map_err(|error| unsafe_path(path, error))?;
    validate_directory_boundary(root, path)?;
    let root_metadata = fs::symlink_metadata(root).map_err(|error| access_error(root, error))?;
    let root_identity = required_identity(root, &root_metadata)?.identity;
    let metadata = fs::symlink_metadata(path).map_err(|error| access_error(path, error))?;
    let identity = required_identity(path, &metadata)?;
    // The selected child can itself be a mount root (for example under
    // /Volumes). Reject that boundary before opening its directory stream.
    validate_directory_device(path, root_identity, identity.identity)?;
    let modified = metadata
        .modified()
        .map_err(|error| access_error(path, error))?;
    struct Frame {
        path: PathBuf,
        entries: fs::ReadDir,
        fingerprint: [u8; 32],
    }
    // One open iterator per depth, never an unbounded per-directory child list.
    let initial = folder_metadata_hash(path, Path::new(""), &metadata)?;
    let mut stack = vec![Frame {
        path: path.to_owned(),
        entries: fs::read_dir(path).map_err(|error| access_error(path, error))?,
        fingerprint: initial,
    }];
    let mut fingerprints = vec![initial];
    let mut files = 0_u64;
    let mut directories = 1_u64;
    let mut logical_bytes = 0_u64;
    let mut path_bytes = 0_usize;
    while let Some(frame) = stack.last_mut() {
        check_cancelled(should_cancel)?;
        crate::ensure_operation_memory().map_err(|error| unsafe_path(path, error))?;
        if started.elapsed() > FOLDER_MAX_DURATION {
            return Err(unsafe_path(
                path,
                "폴더 검토 시간 상한(30초)에 도달했습니다. 더 작은 하위 폴더를 선택하세요",
            ));
        }
        let Some(entry) = frame.entries.next() else {
            let metadata = fs::symlink_metadata(&frame.path)
                .map_err(|error| access_error(&frame.path, error))?;
            let relative = frame
                .path
                .strip_prefix(path)
                .map_err(|_| unsafe_path(path, "폴더 경계가 변경됐습니다"))?;
            if is_link_or_reparse_point(&metadata)
                || !metadata.is_dir()
                || folder_metadata_hash(&frame.path, relative, &metadata)? != frame.fingerprint
            {
                return Err(ActionValidationError::Changed(
                    frame.path.to_string_lossy().into_owned(),
                ));
            }
            stack.pop();
            continue;
        };
        let entry = entry.map_err(|error| access_error(&frame.path, error))?;
        let entry_path = entry.path();
        if files + directories >= FOLDER_MAX_ENTRIES {
            return Err(unsafe_path(
                path,
                "폴더 검토 항목 상한(20,000개)에 도달했습니다. 더 작은 하위 폴더를 선택하세요",
            ));
        }
        path_bytes = path_bytes.saturating_add(entry_path.as_os_str().len());
        if path_bytes > FOLDER_MAX_PATH_BYTES {
            return Err(unsafe_path(path, "폴더 검토 경로 용량 상한에 도달했습니다"));
        }
        // Hidden files and nested downloaded apps/repositories are included, not
        // silently skipped. Links and cloud/offline content fail the entire plan.
        crate::scan_policy::ensure_local_path(&entry_path)
            .map_err(|error| unsafe_path(&entry_path, error))?;
        let entry_metadata =
            fs::symlink_metadata(&entry_path).map_err(|error| access_error(&entry_path, error))?;
        if is_link_or_reparse_point(&entry_metadata)
            || (!entry_metadata.is_file() && !entry_metadata.is_dir())
        {
            return Err(unsafe_path(
                &entry_path,
                "링크 또는 특수 항목이 포함되어 폴더 전체를 이동하지 않습니다",
            ));
        }
        let entry_identity = required_identity(&entry_path, &entry_metadata)?;
        validate_directory_device(&entry_path, identity.identity, entry_identity.identity)?;
        let relative = entry_path
            .strip_prefix(path)
            .map_err(|_| unsafe_path(path, "폴더 경계가 변경됐습니다"))?;
        let fingerprint = folder_metadata_hash(&entry_path, relative, &entry_metadata)?;
        fingerprints.push(fingerprint);
        if entry_metadata.is_dir() {
            directories += 1;
            if stack.len() >= FOLDER_MAX_DEPTH {
                return Err(unsafe_path(path, "폴더 검토 깊이 상한(64)에 도달했습니다"));
            }
            let entries =
                fs::read_dir(&entry_path).map_err(|error| access_error(&entry_path, error))?;
            stack.push(Frame {
                path: entry_path,
                entries,
                fingerprint,
            });
        } else {
            files += 1;
            logical_bytes = logical_bytes
                .checked_add(entry_metadata.len())
                .ok_or_else(|| unsafe_path(path, "폴더 용량 상한을 초과했습니다"))?;
        }
    }
    check_cancelled(should_cancel)?;
    validate_directory_boundary(root, path)?;
    let root_after = fs::symlink_metadata(root).map_err(|error| access_error(root, error))?;
    if required_identity(root, &root_after)?.identity != root_identity {
        return Err(ActionValidationError::Changed(
            root.to_string_lossy().into_owned(),
        ));
    }
    // Sorting fixed 32-byte hashes makes filesystem enumeration order irrelevant;
    // at most 20,001 hashes (< 641 KiB) are retained, no file bodies are allocated.
    fingerprints.sort_unstable();
    let mut hasher = blake3::Hasher::new();
    for fingerprint in fingerprints {
        hasher.update(&fingerprint);
    }
    Ok(DirectorySnapshot {
        path: path.to_owned(),
        root: root.to_owned(),
        root_identity,
        identity,
        modified,
        files,
        directories,
        logical_bytes,
        fingerprint: *hasher.finalize().as_bytes(),
    })
}

fn validate_directory_device(
    path: &Path,
    parent: crate::FileIdentity,
    child: crate::FileIdentity,
) -> Result<(), ActionValidationError> {
    if parent.device != child.device {
        return Err(unsafe_path(
            path,
            "다른 저장장치의 마운트 경계가 포함되어 있습니다",
        ));
    }
    Ok(())
}

/// Validate a single regular file against a server-owned directory scan.
/// A matching path alone is not sufficient: preserve scan-time identity and mtime.
pub fn validate_directory_trash_file<C>(
    report: &crate::DirectoryScanReport,
    selected_path: &str,
    should_cancel: C,
) -> Result<VerifiedTrashItem, ActionValidationError>
where
    C: Fn() -> bool,
{
    check_cancelled(&should_cancel)?;
    let node = report
        .children
        .iter()
        .find(|node| node.path == selected_path)
        .ok_or_else(|| {
            ActionValidationError::InvalidSelection(
                "현재 폴더 지도에 없는 파일입니다. 다시 검사하세요".to_owned(),
            )
        })?;
    let path = PathBuf::from(selected_path);
    if node.is_directory {
        return Err(unsafe_path(
            &path,
            "폴더 지도에서는 파일만 휴지통으로 이동할 수 있습니다",
        ));
    }
    validate_cleanup_path_boundary(&path)?;
    let metadata = safe_file_metadata(&path)?;
    let identity = required_identity(&path, &metadata)?;
    let canonical = fs::canonicalize(&path).map_err(|error| access_error(&path, error))?;
    if !canonical
        .parent()
        .is_some_and(|parent| paths_equal(parent, Path::new(&report.root)))
    {
        return Err(unsafe_path(&path, "검사한 폴더의 직계 파일이 아닙니다"));
    }
    if node.scan_identity != Some(identity)
        || node.scan_modified_at.is_none()
        || node.scan_modified_at != metadata.modified().ok()
        || node.logical_bytes != metadata.len()
    {
        return Err(ActionValidationError::Changed(selected_path.to_owned()));
    }
    // Reuse the content-fingerprinted, identity-checked trash pipeline.
    let snapshot = capture_cleanup_snapshot(&path, 1, &should_cancel)?;
    if snapshot.identity != identity
        || snapshot.logical_bytes != node.logical_bytes
        || snapshot.latest_modified_at_unix_ms != node.modified_at_unix_ms
        || !paths_equal(&snapshot.canonical_path, &canonical)
    {
        return Err(ActionValidationError::Changed(selected_path.to_owned()));
    }
    Ok(VerifiedTrashItem {
        path,
        logical_bytes: snapshot.logical_bytes,
        snapshot: VerifiedSnapshot::Cleanup(snapshot),
        required_keeper: None,
    })
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
        let mut walker = crate::streaming_walk::StreamingWalk::new(path, should_cancel);
        let mut entry_fingerprints = Vec::new();
        for item in walker.by_ref() {
            check_cancelled(should_cancel)?;
            let entry = item.map_err(|error| ActionValidationError::Access {
                path: error.path().unwrap_or(path).to_string_lossy().into_owned(),
                message: error.to_string(),
            })?;
            if entry.depth() == 0 {
                continue;
            }
            entry_count = entry_count.saturating_add(1);
            if entry_count > FOLDER_MAX_ENTRIES {
                return Err(unsafe_path(
                    path,
                    "정리 검토 항목 상한에 도달했습니다. 더 작은 항목을 선택하세요",
                ));
            }
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
            let mut entry_hasher = blake3::Hasher::new();
            update_snapshot_hash(
                &mut entry_hasher,
                relative,
                &entry_metadata,
                file_object_identity(&entry_path, &entry_metadata),
            );
            entry_fingerprints.push(*entry_hasher.finalize().as_bytes());

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
        if walker.excluded_entries() > 0 {
            return Err(unsafe_path(
                path,
                "클라우드·오프라인·링크 항목이 포함되어 폴더 전체를 이동하지 않습니다",
            ));
        }
        // Enumeration order is deliberately unspecified in the streaming walker.
        entry_fingerprints.sort_unstable();
        for fingerprint in entry_fingerprints {
            hasher.update(&fingerprint);
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
                "중복 파일의 내용 확인 번호가 올바르지 않습니다".to_owned(),
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
    fn action_tempdir() -> tempfile::TempDir {
        // Windows' default temp directory is inside protected AppData. Keep
        // allowed-action fixtures outside it without relaxing production policy.
        #[cfg(windows)]
        let directory = tempfile::tempdir_in(std::env::current_dir().unwrap());
        #[cfg(not(windows))]
        let directory = tempfile::tempdir();
        directory.expect("create isolated action fixture")
    }

    #[cfg(windows)]
    #[test]
    fn directory_boundary_checks_root_after_verbatim_drive_prefix() {
        let temp = action_tempdir();
        let child = temp.path().join("ordinary-folder");
        std::fs::create_dir(&child).unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let child = std::fs::canonicalize(child).unwrap();
        assert!(matches!(
            root.components().next(),
            Some(std::path::Component::Prefix(prefix))
                if matches!(prefix.kind(), std::path::Prefix::VerbatimDisk(_))
        ));
        super::validate_directory_boundary(&root, &child).unwrap();
        let protected = child.join("AppData");
        std::fs::create_dir(&protected).unwrap();
        assert!(super::validate_directory_boundary(&root, &protected).is_err());
    }

    #[test]
    fn empty_directory_trash_rejects_hidden_content_changed_identity_and_root() {
        let temp = action_tempdir();
        let empty = temp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        let report = crate::scan_directory_level(
            temp.path(),
            crate::DirectoryScanConfig::default(),
            |_| {},
            || false,
        )
        .unwrap();
        let path = &report.empty_directories[0].path;
        let item = super::validate_empty_directory_trash(&report, path, || false).unwrap();
        assert_eq!(item.logical_bytes(), 0);
        assert!(super::validate_empty_directory_trash(&report, &report.root, || false).is_err());
        assert!(super::validate_empty_directory_trash(&report, path, || true).is_err());
        std::fs::write(empty.join(".keep"), b"needed").unwrap();
        assert!(super::revalidate_verified_trash_item(&item, || false).is_err());
        assert!(super::validate_empty_directory_trash(&report, path, || false).is_err());
        // Move, never remove, synthetic test content to ensure a different object identity.
        std::fs::rename(&empty, temp.path().join("old-empty")).unwrap();
        std::fs::create_dir(&empty).unwrap();
        assert!(super::validate_empty_directory_trash(&report, path, || false).is_err());
        assert!(super::revalidate_verified_trash_item(&item, || false).is_err());
    }

    #[test]
    fn empty_directory_trash_protects_app_cloud_and_hidden_folders() {
        let temp = action_tempdir();
        for name in [
            ".git/empty",
            "Library/empty",
            "sample.app/empty",
            "AppData/empty",
            "Users/a/OneDrive/empty",
            "normal",
        ] {
            std::fs::create_dir_all(temp.path().join(name)).unwrap();
        }
        let report = crate::scan_directory_level(
            temp.path(),
            crate::DirectoryScanConfig::default(),
            |_| {},
            || false,
        )
        .unwrap();
        for entry in &report.empty_directories {
            let result = super::validate_empty_directory_trash(&report, &entry.path, || false);
            assert_eq!(result.is_ok(), entry.name == "normal", "{}", entry.path);
        }
        assert!(
            !report
                .empty_directories
                .iter()
                .any(|entry| entry.path.contains("OneDrive"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn empty_directory_trash_rejects_parent_symlink_swap_and_accepts_sibling_moves() {
        let temp = action_tempdir();
        let root = temp.path().join("scope");
        std::fs::create_dir_all(root.join("one")).unwrap();
        std::fs::create_dir(root.join("two")).unwrap();
        let report = crate::scan_directory_level(
            &root,
            crate::DirectoryScanConfig::default(),
            |_| {},
            || false,
        )
        .unwrap();
        let one = super::validate_empty_directory_trash(
            &report,
            &report.empty_directories[0].path,
            || false,
        )
        .unwrap();
        let two = super::validate_empty_directory_trash(
            &report,
            &report.empty_directories[1].path,
            || false,
        )
        .unwrap();
        std::fs::rename(one.path(), temp.path().join("mock-trash-one")).unwrap();
        super::revalidate_verified_trash_item(&two, || false).unwrap();
        std::fs::rename(&root, temp.path().join("scope-original")).unwrap();
        std::os::unix::fs::symlink(temp.path().join("scope-original"), &root).unwrap();
        assert!(super::revalidate_verified_trash_item(&two, || false).is_err());
    }
    use super::*;
    use crate::{
        CleanupCandidateKind, CleanupRootSpec, CleanupScanConfig, ScanConfig,
        scan_cleanup_candidates, scan_path,
    };
    use std::time::Duration;

    fn directory_report(root: &Path) -> crate::DirectoryScanReport {
        crate::scan_directory_level(root, Default::default(), |_| {}, || false).unwrap()
    }

    #[test]
    fn directory_folder_review_rejects_selected_mount_identity_before_traversal() {
        let parent = crate::FileIdentity {
            device: 1,
            index: 10,
        };
        let normal_child = crate::FileIdentity {
            device: 1,
            index: 20,
        };
        let mounted_child = crate::FileIdentity {
            device: 2,
            index: 20,
        };
        let synthetic_path = Path::new("/Volumes/External");
        assert!(validate_directory_device(synthetic_path, parent, normal_child).is_ok());
        assert!(matches!(
            validate_directory_device(synthetic_path, parent, mounted_child),
            Err(ActionValidationError::UnsafePath { .. })
        ));
    }

    #[test]
    fn directory_folder_review_includes_hidden_downloaded_apps_and_repository_contents() {
        let temp = action_tempdir();
        let folder = temp.path().join("BroomSweepy-download");
        fs::create_dir_all(folder.join("BroomSweepy.app/Contents")).unwrap();
        fs::create_dir(folder.join(".git")).unwrap();
        fs::write(folder.join(".hidden"), b"hidden").unwrap();
        fs::write(folder.join(".git/config"), b"repo").unwrap();
        fs::write(folder.join("BroomSweepy.app/Contents/Info.plist"), b"app").unwrap();
        let report = directory_report(temp.path());
        let node = &report.children[0];
        let item = validate_directory_trash_folder(&report, &node.path, || false).unwrap();
        assert_eq!(item.logical_bytes(), 13);
        assert_eq!(item.directory_counts(), Some((3, 4)));
        revalidate_verified_trash_item(&item, || false).unwrap();
        fs::write(folder.join(".hidden"), b"edited content").unwrap();
        assert!(revalidate_verified_trash_item(&item, || false).is_err());
    }

    #[test]
    fn directory_folder_review_rejects_new_content_replacement_unknown_root_and_cancel() {
        let temp = action_tempdir();
        let folder = temp.path().join("download");
        fs::create_dir(&folder).unwrap();
        let report = directory_report(temp.path());
        let path = &report.children[0].path;
        let item = validate_directory_trash_folder(&report, path, || false).unwrap();
        assert!(validate_directory_trash_folder(&report, &report.root, || false).is_err());
        assert!(validate_directory_trash_folder(&report, "/unknown", || false).is_err());
        assert!(validate_directory_trash_folder(&report, path, || true).is_err());
        assert!(revalidate_verified_trash_item(&item, || true).is_err());
        fs::write(folder.join("new"), b"new").unwrap();
        assert!(revalidate_verified_trash_item(&item, || false).is_err());
        assert!(validate_directory_trash_folder(&report, path, || false).is_err());
        fs::rename(&folder, temp.path().join("original")).unwrap();
        fs::create_dir(&folder).unwrap();
        assert!(validate_directory_trash_folder(&report, path, || false).is_err());
    }

    #[test]
    fn directory_folder_review_rejects_cloud_ancestors_protected_targets_and_deep_trees() {
        let temp = action_tempdir();
        for relative in [
            "Library",
            "sample.app",
            "Downloads",
            "download/Users/a/OneDrive/cloud",
        ] {
            fs::create_dir_all(temp.path().join(relative)).unwrap();
        }
        let report = directory_report(temp.path());
        for node in &report.children {
            if node.name == "Downloads" {
                continue;
            }
            assert!(
                validate_directory_trash_folder(&report, &node.path, || false).is_err(),
                "{}",
                node.path
            );
        }
        let deep = temp.path().join("deep");
        let mut path = deep.clone();
        for _ in 0..=FOLDER_MAX_DEPTH {
            path.push("d");
        }
        fs::create_dir_all(&path).unwrap();
        assert!(capture_directory_snapshot(temp.path(), &deep, &|| false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn directory_folder_review_rejects_nested_links_parent_swap_and_unreadable_subtrees() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let temp = action_tempdir();
        let root = temp.path().join("root");
        let folder = root.join("folder");
        fs::create_dir_all(folder.join("nested")).unwrap();
        let report = directory_report(&root);
        let item =
            validate_directory_trash_folder(&report, &report.children[0].path, || false).unwrap();
        symlink(temp.path(), folder.join("outside-link")).unwrap();
        assert!(revalidate_verified_trash_item(&item, || false).is_err());
        fs::rename(
            folder.join("outside-link"),
            temp.path().join("preserved-link"),
        )
        .unwrap();
        fs::set_permissions(folder.join("nested"), fs::Permissions::from_mode(0o0)).unwrap();
        let unreadable = capture_directory_snapshot(&root, &folder, &|| false);
        let is_unreadable = fs::read_dir(folder.join("nested")).is_err();
        fs::set_permissions(folder.join("nested"), fs::Permissions::from_mode(0o700)).unwrap();
        if is_unreadable {
            assert!(unreadable.is_err());
        }
        fs::rename(&root, temp.path().join("root-original")).unwrap();
        symlink(temp.path().join("root-original"), &root).unwrap();
        assert!(revalidate_verified_trash_item(&item, || false).is_err());
    }

    #[test]
    fn directory_trash_validates_file_and_rejects_changes() {
        let temp = action_tempdir();
        let file = temp.path().join("item.txt");
        fs::write(&file, b"original").unwrap();
        let report = directory_report(temp.path());
        let path = &report.children[0].path;
        let verified = validate_directory_trash_file(&report, path, || false).unwrap();
        revalidate_verified_trash_item(&verified, || false).unwrap();
        assert!(validate_directory_trash_file(&report, path, || true).is_err());
        fs::write(&file, b"modified file").unwrap();
        assert!(validate_directory_trash_file(&report, path, || false).is_err());
        assert!(revalidate_verified_trash_item(&verified, || false).is_err());
    }

    #[test]
    fn directory_trash_rejects_folders_unknown_paths_and_replaced_identity() {
        let temp = action_tempdir();
        let file = temp.path().join("item.txt");
        fs::write(&file, b"original").unwrap();
        fs::create_dir(temp.path().join("folder")).unwrap();
        let report = directory_report(temp.path());
        let node = report
            .children
            .iter()
            .find(|node| !node.is_directory)
            .unwrap();
        let folder = report
            .children
            .iter()
            .find(|node| node.is_directory)
            .unwrap();
        assert!(validate_directory_trash_file(&report, &folder.path, || false).is_err());
        assert!(validate_directory_trash_file(&report, "/not-in-report", || false).is_err());
        // Preserve size and modification time; only file identity changes.
        fs::rename(&file, temp.path().join("original.txt")).unwrap();
        fs::write(&file, b"original").unwrap();
        let replacement = fs::OpenOptions::new().write(true).open(&file).unwrap();
        replacement
            .set_times(fs::FileTimes::new().set_modified(node.scan_modified_at.unwrap()))
            .unwrap();
        assert!(validate_directory_trash_file(&report, &node.path, || false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn directory_trash_rejects_links_and_redirected_parent() {
        use std::os::unix::fs::symlink;
        let temp = action_tempdir();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let file = root.join("item.txt");
        fs::write(&file, b"original").unwrap();
        let report = directory_report(&root);
        let node = &report.children[0];
        fs::hard_link(&file, root.join("hardlink.txt")).unwrap();
        assert!(validate_directory_trash_file(&report, &node.path, || false).is_err());
        fs::remove_file(root.join("hardlink.txt")).unwrap();
        fs::rename(&file, root.join("saved.txt")).unwrap();
        symlink(root.join("saved.txt"), &file).unwrap();
        assert!(validate_directory_trash_file(&report, &node.path, || false).is_err());
        fs::remove_file(&file).unwrap();
        fs::rename(root.join("saved.txt"), &file).unwrap();
        fs::rename(&root, temp.path().join("moved-root")).unwrap();
        symlink(temp.path().join("moved-root"), &root).unwrap();
        assert!(validate_directory_trash_file(&report, &node.path, || false).is_err());
    }

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
        let temp = action_tempdir();
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
        let temp = action_tempdir();
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
        let temp = action_tempdir();
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
