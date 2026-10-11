//! Bounded, metadata-only observations for the optional desktop review loop.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;

use crate::streaming_walk::StreamingWalk;
use crate::{FileIdentity, ScanError, file_object_identity, scan_policy, system_time_ms};

const MAX_VISITED_ENTRIES: usize = 10_000;
const MAX_SNAPSHOT_PATH_BYTES: usize = 4 * 1024 * 1024;
const MAX_SCAN_DURATION: Duration = Duration::from_secs(8);
const MAX_REPORT_CHANGES: usize = 250;
const MAX_REPORT_BYTES: usize = 1024 * 1024;
const BULK_CHANGE_THRESHOLD: usize = 50;

#[derive(Debug)]
pub struct FileChangeSnapshot {
    root: PathBuf,
    root_identity: ObjectIdentity,
    entries: BTreeMap<PathBuf, EntryMetadata>,
    checked_at_unix_ms: u128,
}

impl FileChangeSnapshot {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn checked_at_unix_ms(&self) -> u128 {
        self.checked_at_unix_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Created,
    Modified,
    Renamed,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileReviewState {
    ReviewNeeded,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeEntry {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_path: Option<String>,
    pub kind: FileChangeKind,
    pub review_state: FileReviewState,
    pub reasons: Vec<String>,
    pub is_directory: bool,
    pub logical_bytes: u64,
    pub modified_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeReport {
    pub changes: Vec<FileChangeEntry>,
    pub total_changes: usize,
    pub truncated: bool,
    pub baseline_entries: usize,
    pub checked_at_unix_ms: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ObjectIdentity {
    file: FileIdentity,
    created: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryType {
    File,
    Directory,
    Link,
    Other,
}

#[derive(Debug, Clone)]
struct EntryMetadata {
    identity: ObjectIdentity,
    links: u64,
    kind: EntryType,
    logical_bytes: u64,
    modified: SystemTime,
    executable_bits: u32,
}

impl EntryMetadata {
    fn capture(path: &Path, metadata: &Metadata) -> Result<Self, ScanError> {
        let object = file_object_identity(path, metadata)
            .ok_or_else(|| access("파일의 메타데이터 신원을 확인하지 못했습니다"))?;
        let kind = if metadata.file_type().is_symlink() {
            EntryType::Link
        } else if metadata.is_file() {
            EntryType::File
        } else if metadata.is_dir() {
            EntryType::Directory
        } else {
            EntryType::Other
        };
        #[cfg(unix)]
        let executable_bits = {
            use std::os::unix::fs::MetadataExt;
            metadata.mode() & 0o111
        };
        #[cfg(not(unix))]
        let executable_bits = 0;
        Ok(Self {
            identity: ObjectIdentity {
                file: object.identity,
                created: metadata.created().ok(),
            },
            links: object.links,
            kind,
            logical_bytes: metadata.len(),
            modified: metadata
                .modified()
                .map_err(|error| access(format!("수정 시각을 확인하지 못했습니다: {error}")))?,
            executable_bits,
        })
    }

    fn changed_from(&self, previous: &Self) -> bool {
        if self.identity != previous.identity || self.kind != previous.kind {
            return true;
        }
        if self.kind == EntryType::Directory {
            return self.executable_bits != previous.executable_bits;
        }
        // minimal: identical size/mtime/identity cannot reveal content edits —
        // reconsider only if a separately authorized content-inspection mode exists.
        self.logical_bytes != previous.logical_bytes
            || self.modified != previous.modified
            || self.executable_bits != previous.executable_bits
    }

    fn reportable(&self) -> bool {
        matches!(self.kind, EntryType::File | EntryType::Directory)
    }
}

#[derive(Clone, Copy)]
struct SnapshotLimits {
    max_visited_entries: usize,
    max_path_bytes: usize,
    max_duration: Duration,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_visited_entries: MAX_VISITED_ENTRIES,
            max_path_bytes: MAX_SNAPSHOT_PATH_BYTES,
            max_duration: MAX_SCAN_DURATION,
        }
    }
}

/// Return a complete local metadata observation, or an error without a baseline.
/// File contents are never opened, executed, or written by this operation.
pub fn capture_file_change_snapshot<C>(
    root: impl AsRef<Path>,
    should_cancel: C,
) -> Result<FileChangeSnapshot, ScanError>
where
    C: Fn() -> bool,
{
    capture_snapshot(root.as_ref(), &should_cancel, SnapshotLimits::default())
}

fn capture_snapshot<C: Fn() -> bool>(
    requested_root: &Path,
    should_cancel: &C,
    limits: SnapshotLimits,
) -> Result<FileChangeSnapshot, ScanError> {
    let started = Instant::now();
    check_budget(started, should_cancel, limits)?;
    crate::ensure_operation_memory().map_err(ScanError::Access)?;
    reject_network_path(requested_root)?;
    if scan_policy::is_cloud_path(requested_root) {
        return Err(access(scan_policy::CLOUD_EXCLUDED_MESSAGE));
    }
    let requested_metadata = fs::symlink_metadata(requested_root).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ScanError::MissingPath(requested_root.to_string_lossy().into_owned())
        } else {
            access(error.to_string())
        }
    })?;
    require_directory(&requested_metadata, requested_root)?;
    ensure_local_volume(requested_root)?;
    let root = crate::validate_local_directory_path(requested_root)?;
    require_displayable_path(&root)?;
    reject_network_path(&root)?;
    if scan_policy::is_cloud_path(&root) {
        return Err(access(scan_policy::CLOUD_EXCLUDED_MESSAGE));
    }
    ensure_local_volume(&root)?;
    let root_metadata = fs::symlink_metadata(&root).map_err(|error| access(error.to_string()))?;
    require_directory(&root_metadata, &root)?;
    let root_state = EntryMetadata::capture(&root, &root_metadata)?;
    let root_identity = root_state.identity;
    if EntryMetadata::capture(requested_root, &requested_metadata)?.identity != root_identity {
        return Err(root_changed_error());
    }
    let mut path_bytes = native_path_bytes(&root);
    if path_bytes > limits.max_path_bytes {
        return Err(path_limit_error());
    }
    let mut entries = BTreeMap::new();
    let stop_walk = || should_cancel() || started.elapsed() >= limits.max_duration;
    let directory_guard = |path: &Path, metadata: &Metadata| {
        let identity = file_object_identity(path, metadata)
            .ok_or_else(|| io::Error::other("폴더의 신원을 확인하지 못했습니다"))?;
        if identity.identity.device != root_identity.file.device {
            return Err(io::Error::other(
                "다른 저장장치의 마운트 경계가 포함되어 검사를 완료하지 못했습니다",
            ));
        }
        Ok(())
    };
    let mut walk = StreamingWalk::with_directory_guard(
        &root,
        &stop_walk,
        &directory_guard,
        limits.max_visited_entries,
    );
    while let Some(result) = walk.next() {
        check_budget(started, should_cancel, limits)?;
        reject_excluded_entries(walk.excluded_entries())?;
        let entry = result.map_err(|error| access(error.to_string()))?;
        let path = entry.path();
        require_displayable_path(&path)?;
        let metadata = entry
            .metadata()
            .map_err(|error| access(error.to_string()))?;
        let state = EntryMetadata::capture(&path, &metadata)?;
        if state.identity.file.device != root_identity.file.device {
            return Err(access(
                "다른 저장장치의 마운트 경계가 포함되어 검사를 완료하지 못했습니다",
            ));
        }
        if entry.depth() == 0 {
            if state.identity != root_identity {
                return Err(root_changed_error());
            }
            continue;
        }
        path_bytes = path_bytes.saturating_add(native_path_bytes(&path));
        if path_bytes > limits.max_path_bytes {
            return Err(path_limit_error());
        }
        entries.insert(path, state);
    }
    check_budget(started, should_cancel, limits)?;
    reject_excluded_entries(walk.excluded_entries())?;
    // Directory revalidation catches additions/removals during enumeration.
    // Individual OS metadata calls remain cooperatively, not forcibly, timed.
    for (path, observed) in std::iter::once((&root, &root_state)).chain(
        entries
            .iter()
            .filter(|(_, state)| state.kind == EntryType::Directory),
    ) {
        check_budget(started, should_cancel, limits)?;
        let metadata = fs::symlink_metadata(path).map_err(|error| access(error.to_string()))?;
        require_directory(&metadata, path)?;
        let current = EntryMetadata::capture(path, &metadata)?;
        if current.identity != observed.identity || current.modified != observed.modified {
            return Err(access(
                "검사 중 폴더가 변경되어 검사를 완료하지 못했습니다. 다시 확인해 주세요",
            ));
        }
    }
    check_budget(started, should_cancel, limits)?;
    if crate::validate_local_directory_path(&root)? != root {
        return Err(root_changed_error());
    }
    check_budget(started, should_cancel, limits)?;
    Ok(FileChangeSnapshot {
        root,
        root_identity,
        entries,
        checked_at_unix_ms: system_time_ms(Some(SystemTime::now())).unwrap_or_default(),
    })
}

fn check_budget<C: Fn() -> bool>(
    started: Instant,
    should_cancel: &C,
    limits: SnapshotLimits,
) -> Result<(), ScanError> {
    if should_cancel() {
        return Err(ScanError::Cancelled);
    }
    if started.elapsed() >= limits.max_duration {
        return Err(access(
            "변경 확인 시간 상한(8초)에 도달했습니다. 더 작은 폴더를 선택해 주세요",
        ));
    }
    Ok(())
}

fn require_directory(metadata: &Metadata, path: &Path) -> Result<(), ScanError> {
    if metadata.file_type().is_symlink() || scan_policy::is_reparse_point(metadata) {
        return Err(access("링크 폴더는 변경 확인 대상으로 선택할 수 없습니다"));
    }
    if scan_policy::is_online_only(metadata) {
        return Err(access(scan_policy::CLOUD_EXCLUDED_MESSAGE));
    }
    if !metadata.is_dir() {
        return Err(ScanError::NotDirectory(path.to_string_lossy().into_owned()));
    }
    Ok(())
}

fn require_displayable_path(path: &Path) -> Result<(), ScanError> {
    if path.to_str().is_none() {
        return Err(access(
            "표시할 수 없는 파일 이름이 포함되어 검사를 완료하지 못했습니다. 다른 로컬 폴더를 선택해 주세요",
        ));
    }
    Ok(())
}

fn reject_excluded_entries(count: u64) -> Result<(), ScanError> {
    if count != 0 {
        // The shared walk reports exclusion counts, not native excluded keys.
        // Refuse an incomplete baseline rather than infer placeholder deletions.
        return Err(access(
            "클라우드 또는 온라인 전용 항목이 포함되어 검사를 완료하지 못했습니다. 더 작은 로컬 폴더를 선택해 주세요",
        ));
    }
    Ok(())
}

fn reject_network_path(path: &Path) -> Result<(), ScanError> {
    let raw = path.as_os_str().to_string_lossy();
    if (raw.starts_with("\\\\") && !raw.starts_with("\\\\?\\"))
        || raw.to_ascii_lowercase().starts_with("\\\\?\\unc\\")
    {
        return Err(access(
            "네트워크 폴더는 변경 확인 대상으로 선택할 수 없습니다",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn ensure_local_volume(path: &Path) -> Result<(), ScanError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| access("폴더 경로에 잘못된 문자가 포함되어 있습니다"))?;
    let mut information = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // statfs reads mount metadata and does not open or hydrate file contents.
    if unsafe { libc::statfs(path.as_ptr(), information.as_mut_ptr()) } != 0 {
        return Err(access(io::Error::last_os_error().to_string()));
    }
    let information = unsafe { information.assume_init() };
    if information.f_flags & libc::MNT_LOCAL as u32 == 0 {
        return Err(access(
            "네트워크 저장장치는 변경 확인 대상으로 선택할 수 없습니다",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn ensure_local_volume(path: &Path) -> Result<(), ScanError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut volume = vec![0_u16; 32_768];
    if unsafe { GetVolumePathNameW(wide.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) } == 0 {
        return Err(access(io::Error::last_os_error().to_string()));
    }
    let drive_type = unsafe { GetDriveTypeW(volume.as_ptr()) };
    // Win32 DRIVE_REMOVABLE(2), DRIVE_FIXED(3), DRIVE_CDROM(5), DRIVE_RAMDISK(6).
    // Keep the existing FileSystem feature without adding WindowsProgramming.
    if !matches!(drive_type, 2 | 3 | 5 | 6) {
        return Err(access(
            "네트워크 저장장치는 변경 확인 대상으로 선택할 수 없습니다",
        ));
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn ensure_local_volume(_path: &Path) -> Result<(), ScanError> {
    Ok(())
}

#[cfg(windows)]
fn native_path_bytes(path: &Path) -> usize {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().count().saturating_mul(2)
}

#[cfg(not(windows))]
fn native_path_bytes(path: &Path) -> usize {
    path.as_os_str().len()
}

fn path_limit_error() -> ScanError {
    access("변경 확인의 보관 경로 상한(4 MiB)에 도달했습니다. 더 작은 폴더를 선택해 주세요")
}

fn root_changed_error() -> ScanError {
    access("대상 폴더의 신원이 변경되었습니다. 새 비교 기준을 만들어 주세요")
}

fn access(message: impl Into<String>) -> ScanError {
    ScanError::Access(message.into())
}

#[derive(Clone, Copy)]
struct Change<'a> {
    path: &'a Path,
    previous_path: Option<&'a Path>,
    metadata: &'a EntryMetadata,
    kind: FileChangeKind,
    metadata_changed: bool,
}

/// Compare complete observations of the same native root identity.
/// Only unique single-link identities can supply rename evidence.
pub fn compare_file_change_snapshots(
    previous: &FileChangeSnapshot,
    next: &FileChangeSnapshot,
) -> Result<FileChangeReport, ScanError> {
    if previous.root != next.root || previous.root_identity != next.root_identity {
        return Err(root_changed_error());
    }
    crate::ensure_operation_memory().map_err(ScanError::Access)?;
    let previous_identities = identity_counts(&previous.entries);
    let next_identities = identity_counts(&next.entries);
    let mut remaining_previous = BTreeMap::new();
    let mut remaining_next = BTreeMap::new();
    let mut changes = Vec::new();
    for (path, state) in &previous.entries {
        if let Some(current) = next.entries.get(path)
            && current.identity == state.identity
            && current.kind == state.kind
        {
            if current.reportable() && current.changed_from(state) {
                changes.push(Change {
                    path,
                    previous_path: None,
                    metadata: current,
                    kind: FileChangeKind::Modified,
                    metadata_changed: true,
                });
            }
        } else {
            remaining_previous.insert(path.as_path(), state);
        }
    }
    for (path, state) in &next.entries {
        if !previous
            .entries
            .get(path)
            .is_some_and(|old| old.identity == state.identity && old.kind == state.kind)
        {
            remaining_next.insert(path.as_path(), state);
        }
    }
    let mut consumed_previous = HashSet::new();
    let mut consumed_next = HashSet::new();
    for (&old_path, &old) in &remaining_previous {
        if !old.reportable() || (old.kind == EntryType::File && old.links != 1) {
            continue;
        }
        let Some(&(1, Some(new_path))) = next_identities.get(&old.identity) else {
            continue;
        };
        if previous_identities
            .get(&old.identity)
            .map(|(count, _)| *count)
            != Some(1)
        {
            continue;
        }
        let Some(&current) = remaining_next.get(new_path) else {
            continue;
        };
        if (current.kind == EntryType::File && current.links != 1)
            || current.kind != old.kind
            || new_path == old_path
        {
            continue;
        }
        changes.push(Change {
            path: new_path,
            previous_path: Some(old_path),
            metadata: current,
            kind: FileChangeKind::Renamed,
            metadata_changed: current.changed_from(old),
        });
        consumed_previous.insert(old_path);
        consumed_next.insert(new_path);
    }
    for (&path, &old) in &remaining_previous {
        if consumed_previous.contains(path) {
            continue;
        }
        if let Some(&current) = remaining_next.get(path)
            && !consumed_next.contains(path)
        {
            if old.reportable() != current.reportable() {
                return Err(access(
                    "항목이 링크 또는 특수 파일로 바뀌어 비교를 완료하지 못했습니다",
                ));
            }
            if current.reportable() {
                changes.push(Change {
                    path,
                    previous_path: None,
                    metadata: current,
                    kind: FileChangeKind::Modified,
                    metadata_changed: true,
                });
            }
            consumed_next.insert(path);
        } else if old.reportable() {
            changes.push(Change {
                path,
                previous_path: None,
                metadata: old,
                kind: FileChangeKind::Removed,
                metadata_changed: false,
            });
        }
    }
    for (&path, &current) in &remaining_next {
        if !consumed_next.contains(path) && current.reportable() {
            changes.push(Change {
                path,
                previous_path: None,
                metadata: current,
                kind: FileChangeKind::Created,
                metadata_changed: false,
            });
        }
    }
    changes.sort_unstable_by(|a, b| a.path.cmp(b.path));
    let total_changes = changes.len();
    let bulk = total_changes >= BULK_CHANGE_THRESHOLD;
    let mut result = Vec::with_capacity(total_changes.min(MAX_REPORT_CHANGES));
    let mut report_bytes = 0_usize;
    for change in changes.into_iter().take(MAX_REPORT_CHANGES) {
        let entry = describe_change(change, bulk);
        let retained = (entry.path.len()
            + entry.name.len()
            + entry.previous_path.as_ref().map_or(0, String::len))
        .saturating_mul(6)
            + entry.reasons.iter().map(String::len).sum::<usize>()
            + 256;
        if report_bytes.saturating_add(retained) > MAX_REPORT_BYTES {
            break;
        }
        report_bytes += retained;
        result.push(entry);
    }
    Ok(FileChangeReport {
        truncated: result.len() < total_changes,
        changes: result,
        total_changes,
        baseline_entries: previous.entry_count(),
        checked_at_unix_ms: next.checked_at_unix_ms,
    })
}

fn identity_counts(
    entries: &BTreeMap<PathBuf, EntryMetadata>,
) -> HashMap<ObjectIdentity, (usize, Option<&Path>)> {
    let mut identities = HashMap::new();
    for (path, entry) in entries {
        let value = identities.entry(entry.identity).or_insert((0, None));
        value.0 += 1;
        value.1 = if value.0 == 1 {
            Some(path.as_path())
        } else {
            None
        };
    }
    identities
}

fn describe_change(change: Change<'_>, bulk: bool) -> FileChangeEntry {
    let name = change
        .path
        .file_name()
        .unwrap_or(change.path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut reasons = Vec::new();
    let extension = change
        .path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let executable = (change.metadata.kind == EntryType::File
        && (change.metadata.executable_bits != 0 || executable_extension(&extension)))
        || (change.metadata.kind == EntryType::Directory && extension == "app");
    if executable {
        reasons.push("executable".to_owned());
    }
    if change.metadata.kind == EntryType::File && disguised_double_extension(change.path) {
        reasons.push("double_extension".to_owned());
    }
    if bulk {
        reasons.push("bulk_change".to_owned());
    }
    let review_state = if reasons.is_empty() {
        FileReviewState::Skipped
    } else {
        FileReviewState::ReviewNeeded
    };
    if change.metadata.kind == EntryType::Directory {
        reasons.push("directory_changed".to_owned());
    }
    if change.kind == FileChangeKind::Removed {
        reasons.push("removed".to_owned());
    }
    if change.metadata_changed {
        reasons.push("metadata_changed".to_owned());
    }
    if review_state == FileReviewState::Skipped {
        reasons.push("no_rule_match".to_owned());
    }
    FileChangeEntry {
        path: change.path.to_string_lossy().into_owned(),
        name,
        previous_path: change
            .previous_path
            .map(|path| path.to_string_lossy().into_owned()),
        kind: change.kind,
        review_state,
        reasons,
        is_directory: change.metadata.kind == EntryType::Directory,
        logical_bytes: if change.metadata.kind == EntryType::Directory {
            0
        } else {
            change.metadata.logical_bytes
        },
        modified_at_unix_ms: system_time_ms(Some(change.metadata.modified)),
    }
}

fn executable_extension(extension: &str) -> bool {
    matches!(
        extension,
        "exe"
            | "com"
            | "bat"
            | "cmd"
            | "ps1"
            | "psm1"
            | "vbs"
            | "vbe"
            | "js"
            | "jse"
            | "wsf"
            | "wsh"
            | "scr"
            | "msi"
            | "msp"
            | "cpl"
            | "dll"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "command"
            | "py"
            | "pyw"
            | "pl"
            | "rb"
            | "applescript"
            | "scpt"
            | "jar"
            | "app"
    )
}

fn disguised_double_extension(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };
    if !executable_extension(&extension.to_string_lossy().to_ascii_lowercase()) {
        return false;
    }
    let Some(stem) = path.file_stem() else {
        return false;
    };
    let Some(previous_extension) = Path::new(stem).extension() else {
        return false;
    };
    matches!(
        previous_extension
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_str(),
        "pdf"
            | "txt"
            | "rtf"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "csv"
            | "ppt"
            | "pptx"
            | "hwp"
            | "hwpx"
            | "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "svg"
            | "mp3"
            | "wav"
            | "mp4"
            | "mov"
            | "zip"
            | "rar"
            | "7z"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn capture(root: &Path) -> FileChangeSnapshot {
        capture_file_change_snapshot(root.canonicalize().unwrap(), || false).unwrap()
    }

    #[test]
    fn baseline_fixture_reports_creation_modification_rename_and_removal() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("keep.txt"), b"old").unwrap();
        fs::write(temp.path().join("move.txt"), b"move").unwrap();
        fs::write(temp.path().join("remove.txt"), b"remove").unwrap();
        let before = capture(temp.path());
        assert!(
            compare_file_change_snapshots(&before, &before)
                .unwrap()
                .changes
                .is_empty()
        );
        fs::write(temp.path().join("keep.txt"), b"longer").unwrap();
        fs::rename(temp.path().join("move.txt"), temp.path().join("moved.txt")).unwrap();
        fs::remove_file(temp.path().join("remove.txt")).unwrap();
        fs::write(temp.path().join("invoice.pdf.exe"), b"fixture").unwrap();
        let after = capture(temp.path());
        let report = compare_file_change_snapshots(&before, &after).unwrap();
        assert_eq!(report.total_changes, 4);
        assert_eq!(report.baseline_entries, 3);
        let disguised = report
            .changes
            .iter()
            .find(|entry| entry.name == "invoice.pdf.exe")
            .unwrap();
        assert_eq!(disguised.kind, FileChangeKind::Created);
        assert_eq!(disguised.review_state, FileReviewState::ReviewNeeded);
        assert!(
            disguised
                .reasons
                .iter()
                .any(|reason| reason == "double_extension")
        );
        let renamed = report
            .changes
            .iter()
            .find(|entry| entry.kind == FileChangeKind::Renamed)
            .unwrap();
        assert!(
            renamed
                .previous_path
                .as_ref()
                .unwrap()
                .ends_with("move.txt")
        );
        let removed = report
            .changes
            .iter()
            .find(|entry| entry.kind == FileChangeKind::Removed)
            .unwrap();
        assert_eq!(removed.review_state, FileReviewState::Skipped);
        assert_eq!(removed.reasons, ["removed", "no_rule_match"]);
    }

    #[test]
    fn renamed_and_modified_file_keeps_both_metadata_and_identity_evidence() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("old.txt"), b"old").unwrap();
        let before = capture(temp.path());
        fs::rename(temp.path().join("old.txt"), temp.path().join("new.txt")).unwrap();
        fs::write(temp.path().join("new.txt"), b"new and longer").unwrap();
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 1);
        assert_eq!(report.changes[0].kind, FileChangeKind::Renamed);
        assert!(
            report.changes[0]
                .reasons
                .iter()
                .any(|reason| reason == "metadata_changed")
        );
    }

    #[test]
    fn hard_link_rename_is_ambiguous_and_remains_create_remove() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("one.txt"), b"one").unwrap();
        fs::hard_link(temp.path().join("one.txt"), temp.path().join("two.txt")).unwrap();
        let before = capture(temp.path());
        fs::rename(temp.path().join("two.txt"), temp.path().join("three.txt")).unwrap();
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 2);
        assert!(
            report
                .changes
                .iter()
                .any(|entry| entry.kind == FileChangeKind::Created)
        );
        assert!(
            report
                .changes
                .iter()
                .any(|entry| entry.kind == FileChangeKind::Removed)
        );
        assert!(
            !report
                .changes
                .iter()
                .any(|entry| entry.kind == FileChangeKind::Renamed)
        );
    }

    #[test]
    fn renamed_file_and_new_file_at_old_path_are_distinct_observations() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("one.txt"), b"one").unwrap();
        let before = capture(temp.path());
        fs::rename(temp.path().join("one.txt"), temp.path().join("two.txt")).unwrap();
        fs::write(temp.path().join("one.txt"), b"another").unwrap();
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 2);
        assert!(
            report
                .changes
                .iter()
                .any(|entry| entry.kind == FileChangeKind::Created)
        );
        assert!(
            report
                .changes
                .iter()
                .any(|entry| entry.kind == FileChangeKind::Renamed)
        );
    }

    #[test]
    fn directory_mtime_is_quiet_but_directory_create_remove_is_reported() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("folder")).unwrap();
        let before = capture(temp.path());
        fs::write(temp.path().join("folder/one.txt"), b"one").unwrap();
        fs::create_dir(temp.path().join("Tool.app")).unwrap();
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 2);
        let app = report
            .changes
            .iter()
            .find(|entry| entry.name == "Tool.app")
            .unwrap();
        assert_eq!(app.review_state, FileReviewState::ReviewNeeded);
        assert!(app.is_directory);
        assert!(
            app.reasons
                .iter()
                .any(|reason| reason == "directory_changed")
        );
    }

    #[test]
    fn directory_rename_has_unique_identity_even_with_directory_link_count() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("old-folder")).unwrap();
        let before = capture(temp.path());
        fs::rename(
            temp.path().join("old-folder"),
            temp.path().join("new-folder"),
        )
        .unwrap();
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 1);
        assert_eq!(report.changes[0].kind, FileChangeKind::Renamed);
        assert!(report.changes[0].is_directory);
        assert_eq!(report.changes[0].review_state, FileReviewState::Skipped);
    }

    #[test]
    fn bulk_rule_counts_all_changes_even_when_report_is_truncated() {
        let temp = tempfile::tempdir().unwrap();
        let before = capture(temp.path());
        for index in 0..260 {
            fs::write(temp.path().join(format!("ordinary-{index}.txt")), []).unwrap();
        }
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 260);
        assert_eq!(report.changes.len(), MAX_REPORT_CHANGES);
        assert!(report.truncated);
        assert!(
            report
                .changes
                .iter()
                .all(|entry| entry.review_state == FileReviewState::ReviewNeeded
                    && entry.reasons.iter().any(|reason| reason == "bulk_change"))
        );
    }

    #[test]
    fn cancelled_and_bounded_observations_do_not_produce_a_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::write(temp.path().join("one.txt"), b"one").unwrap();
        assert!(matches!(
            capture_file_change_snapshot(temp.path(), || true),
            Err(ScanError::Cancelled)
        ));
        let limits = SnapshotLimits {
            max_visited_entries: 1,
            ..SnapshotLimits::default()
        };
        assert!(
            capture_snapshot(&root, &|| false, limits)
                .unwrap_err()
                .to_string()
                .contains("항목 상한")
        );
        let limits = SnapshotLimits {
            max_path_bytes: native_path_bytes(&root),
            ..SnapshotLimits::default()
        };
        assert!(
            capture_snapshot(&root, &|| false, limits)
                .unwrap_err()
                .to_string()
                .contains("경로 상한")
        );
        let limits = SnapshotLimits {
            max_duration: Duration::ZERO,
            ..SnapshotLimits::default()
        };
        assert!(
            capture_snapshot(&root, &|| false, limits)
                .unwrap_err()
                .to_string()
                .contains("시간 상한")
        );
        let before = capture(temp.path());
        let calls = Cell::new(0);
        assert!(
            capture_file_change_snapshot(&root, || {
                calls.set(calls.get() + 1);
                calls.get() > 5
            })
            .is_err()
        );
        assert!(
            compare_file_change_snapshots(&before, &before)
                .unwrap()
                .changes
                .is_empty()
        );
    }

    #[test]
    fn replaced_root_requires_a_new_baseline() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("one.txt"), b"one").unwrap();
        let before = capture(&root);
        fs::rename(&root, temp.path().join("old-root")).unwrap();
        fs::create_dir(&root).unwrap();
        let after = capture(&root);
        assert!(compare_file_change_snapshots(&before, &after).is_err());
    }

    #[test]
    fn cloud_exclusions_cannot_be_interpreted_as_removals() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let cloud = temp.path().join("Users/test/Library/CloudStorage/provider");
        fs::create_dir_all(&cloud).unwrap();
        fs::write(cloud.join("one.txt"), b"one").unwrap();
        assert!(
            capture_file_change_snapshot(&root, || false)
                .unwrap_err()
                .to_string()
                .contains("클라우드")
        );
        assert!(capture_file_change_snapshot(&cloud, || false).is_err());
        assert!(reject_network_path(Path::new("\\\\server\\share")).is_err());
        assert!(reject_network_path(Path::new("\\\\?\\UNC\\server\\share")).is_err());
        assert!(reject_network_path(Path::new("\\\\?\\C:\\local")).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn links_are_not_followed_and_file_to_link_transition_is_incomplete() {
        use std::os::unix::fs::symlink;
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.exe"), b"outside").unwrap();
        let temp = tempfile::tempdir().unwrap();
        symlink(outside.path(), temp.path().join("link")).unwrap();
        let before = capture(temp.path());
        assert_eq!(before.entry_count(), 1);
        fs::write(outside.path().join("another.exe"), b"another").unwrap();
        assert!(
            compare_file_change_snapshots(&before, &capture(temp.path()))
                .unwrap()
                .changes
                .is_empty()
        );
        fs::write(temp.path().join("one.txt"), b"one").unwrap();
        let before = capture(temp.path());
        fs::remove_file(temp.path().join("one.txt")).unwrap();
        symlink(
            outside.path().join("outside.exe"),
            temp.path().join("one.txt"),
        )
        .unwrap();
        assert!(compare_file_change_snapshots(&before, &capture(temp.path())).is_err());
        assert!(capture_file_change_snapshot(temp.path().join("link"), || false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn linked_ancestor_is_rejected_before_canonicalizing_the_selected_folder() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("real/child")).unwrap();
        symlink(root.join("real"), root.join("linked")).unwrap();
        let error = capture_file_change_snapshot(root.join("linked/child"), || false).unwrap_err();
        assert!(error.to_string().contains("링크"));
        assert!(capture_file_change_snapshot(root.join("real/child"), || false).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn unix_execution_bit_changes_are_reviewed_and_native_names_do_not_collapse() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("tool");
        fs::write(&script, b"fixture").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o600)).unwrap();
        let before = capture(temp.path());
        assert_eq!(before.entry_count(), 1);
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let report = compare_file_change_snapshots(&before, &capture(temp.path())).unwrap();
        assert_eq!(report.total_changes, 1);
        assert_eq!(
            report.changes[0].review_state,
            FileReviewState::ReviewNeeded
        );
        assert!(
            report.changes[0]
                .reasons
                .iter()
                .any(|reason| reason == "executable")
        );
        let first = temp.path().join(OsString::from_vec(vec![b'a', 0xfe]));
        let second = temp.path().join(OsString::from_vec(vec![b'a', 0xff]));
        assert_ne!(first, second);
        assert_eq!(first.to_string_lossy(), second.to_string_lossy());
        for path in [&first, &second] {
            assert!(
                require_displayable_path(path)
                    .unwrap_err()
                    .to_string()
                    .contains("파일 이름")
            );
        }
        // APFS rejects these names at creation; Linux permits an end-to-end fixture.
        #[cfg(target_os = "linux")]
        {
            fs::write(first, []).unwrap();
            fs::write(second, []).unwrap();
            let result =
                capture_file_change_snapshot(temp.path().canonicalize().unwrap(), || false);
            assert!(result.unwrap_err().to_string().contains("파일 이름"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_subtree_or_disappearing_directory_does_not_commit_partial_observation() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let folder = temp.path().join("private");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("one.txt"), b"one").unwrap();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o000)).unwrap();
        let unreadable = fs::read_dir(&folder).is_err();
        let result = capture_file_change_snapshot(&root, || false);
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
        if unreadable {
            assert!(result.is_err());
        }
        let calls = Cell::new(0);
        let result = capture_file_change_snapshot(&root, || {
            calls.set(calls.get() + 1);
            if calls.get() == 6 {
                fs::remove_file(folder.join("one.txt")).unwrap();
                fs::remove_dir(&folder).unwrap();
            }
            false
        });
        assert!(result.is_err());
    }
}
