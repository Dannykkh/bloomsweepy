use serde::{Deserialize, Serialize};
use std::collections::{HashMap, hash_map::Entry};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use std::{fs, io};

use super::{ScanError, ScanIssue, push_issue, system_time_ms};

const DIRECTORY_PROGRESS_ENTRY_INTERVAL: u64 = 2_048;
const MAX_TRACKED_CHILDREN: usize = 65_536;
const MAX_EMPTY_DIRECTORY_RESULTS: usize = 10_000;
const MAX_DIRECTORY_ISSUES: usize = 1_000;
const MAX_TRACKED_CHILD_PATH_BYTES: usize = 8 * 1024 * 1024;
const MAX_EMPTY_DIRECTORY_PATH_BYTES: usize = 4 * 1024 * 1024;
const MAX_LOCAL_VOLUME_VISITED: usize = 250_000;
const MAX_LOCAL_VOLUME_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DirectoryScanConfig {
    pub max_children: usize,
    pub max_tracked_children: usize,
    pub max_empty_directories: usize,
    pub max_issues: usize,
}

impl Default for DirectoryScanConfig {
    fn default() -> Self {
        Self {
            max_children: 512,
            max_tracked_children: 16_384,
            max_empty_directories: 200,
            max_issues: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryNode {
    pub name: String,
    pub path: String,
    pub logical_bytes: u64,
    pub file_count: u64,
    pub directory_count: u64,
    pub is_directory: bool,
    pub modified_at_unix_ms: Option<u128>,
    // Kept only in the trusted in-process report, never supplied by the UI.
    #[serde(skip)]
    pub(crate) scan_identity: Option<super::FileObjectIdentity>,
    #[serde(skip)]
    pub(crate) scan_modified_at: Option<SystemTime>,
}

impl DirectoryNode {
    /// In-process identity comparison; serialized names/paths are never authority.
    pub fn same_entry_as(&self, other: &Self) -> bool {
        self.path == other.path
            && self.is_directory == other.is_directory
            && self.scan_identity.is_some()
            && self.scan_identity == other.scan_identity
            && self.scan_modified_at.is_some()
            && self.scan_modified_at == other.scan_modified_at
            && (self.is_directory || self.logical_bytes == other.logical_bytes)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyDirectory {
    pub name: String,
    pub path: String,
    pub modified_at_unix_ms: Option<u128>,
    #[serde(skip)]
    pub(crate) scan_identity: Option<super::FileObjectIdentity>,
    #[serde(skip)]
    pub(crate) scan_modified_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryScanProgress {
    pub message: String,
    pub processed_entries: u64,
    pub processed_files: u64,
    pub processed_bytes: u64,
    pub unreadable_entries: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryScanReport {
    pub root: String,
    pub name: String,
    pub parent: Option<String>,
    pub completed_at_unix_ms: u128,
    pub duration_ms: u128,
    pub total_logical_bytes: u64,
    pub total_files: u64,
    pub total_directories: u64,
    pub direct_child_count: u64,
    pub children_truncated: bool,
    pub tracking_limit_reached: bool,
    pub omitted_child_count: u64,
    pub omitted_logical_bytes: u64,
    pub empty_directory_count: u64,
    pub empty_directories_truncated: bool,
    pub unreadable_entries: u64,
    pub children: Vec<DirectoryNode>,
    pub empty_directories: Vec<EmptyDirectory>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Default)]
struct NodeAccumulator {
    path: PathBuf,
    logical_bytes: u64,
    file_count: u64,
    directory_count: u64,
    is_directory: bool,
    modified_at: Option<SystemTime>,
    scan_identity: Option<super::FileObjectIdentity>,
    scan_modified_at: Option<SystemTime>,
}

pub fn scan_directory_level<F, C>(
    root: impl AsRef<Path>,
    config: DirectoryScanConfig,
    on_progress: F,
    should_cancel: C,
) -> Result<DirectoryScanReport, ScanError>
where
    F: FnMut(DirectoryScanProgress),
    C: Fn() -> bool,
{
    scan_directory_level_inner(root.as_ref(), config, on_progress, should_cancel, None)
}

/// A bounded current-folder map confined to one local mounted volume.
/// Any omitted boundary or unfinished traversal makes the report read-only.
pub fn scan_local_volume_directory_level<F, C>(
    root: impl AsRef<Path>,
    config: DirectoryScanConfig,
    excluded_roots: &[PathBuf],
    on_progress: F,
    should_cancel: C,
) -> Result<DirectoryScanReport, ScanError>
where
    F: FnMut(DirectoryScanProgress),
    C: Fn() -> bool,
{
    scan_directory_level_inner(
        root.as_ref(),
        config,
        on_progress,
        should_cancel,
        Some(LocalVolumeScope {
            excluded_roots,
            max_visited: MAX_LOCAL_VOLUME_VISITED,
            max_duration: MAX_LOCAL_VOLUME_DURATION,
        }),
    )
}

#[derive(Clone, Copy)]
struct LocalVolumeScope<'a> {
    excluded_roots: &'a [PathBuf],
    max_visited: usize,
    max_duration: Duration,
}

fn scan_directory_level_inner<F, C>(
    requested_root: &Path,
    config: DirectoryScanConfig,
    mut on_progress: F,
    should_cancel: C,
    local_volume: Option<LocalVolumeScope<'_>>,
) -> Result<DirectoryScanReport, ScanError>
where
    F: FnMut(DirectoryScanProgress),
    C: Fn() -> bool,
{
    let started = Instant::now();
    let max_children = config.max_children.min(MAX_TRACKED_CHILDREN);
    let max_tracked_children = config
        .max_tracked_children
        .max(max_children)
        .min(MAX_TRACKED_CHILDREN);
    let max_empty_directories = config
        .max_empty_directories
        .min(MAX_EMPTY_DIRECTORY_RESULTS);
    let max_issues = config.max_issues.min(MAX_DIRECTORY_ISSUES);
    let validated_root = if local_volume.is_some() {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        Some(crate::validate_local_volume_directory(requested_root)?)
    } else {
        None
    };
    crate::scan_policy::ensure_local_path(requested_root).map_err(ScanError::Access)?;

    if !requested_root.exists() {
        return Err(ScanError::MissingPath(
            requested_root.to_string_lossy().into_owned(),
        ));
    }
    if !requested_root.is_dir() {
        return Err(ScanError::NotDirectory(
            requested_root.to_string_lossy().into_owned(),
        ));
    }

    let root = if let Some(root) = validated_root {
        root
    } else {
        requested_root
            .canonicalize()
            .map_err(|error| ScanError::Access(error.to_string()))?
    };
    crate::scan_policy::ensure_local_path(&root).map_err(ScanError::Access)?;
    let root_identity = if local_volume.is_some() {
        let metadata =
            fs::symlink_metadata(&root).map_err(|error| ScanError::Access(error.to_string()))?;
        Some(crate::local_search::required_directory_identity(
            &root, &metadata,
        )?)
    } else {
        None
    };

    on_progress(DirectoryScanProgress {
        message: "현재 폴더의 용량 지도를 만들고 있습니다".to_owned(),
        processed_entries: 0,
        processed_files: 0,
        processed_bytes: 0,
        unreadable_entries: 0,
    });

    let mut children: HashMap<PathBuf, NodeAccumulator> = HashMap::new();
    let mut empty_directory_count = 0_u64;
    let mut empty_directories = Vec::new();
    let mut empty_directory_path_bytes = 0_usize;
    let mut tracked_child_path_bytes = 0_usize;
    let mut issues = Vec::new();
    let mut processed_entries = 0_u64;
    let mut total_files = 0_u64;
    let mut total_directories = 0_u64;
    let mut direct_child_count = 0_u64;
    let mut total_logical_bytes = 0_u64;
    let mut unreadable_entries = 0_u64;
    let mut tracking_limit_reached = false;
    let mut scope_incomplete = false;
    let root_changed = std::cell::Cell::new(false);
    let stop_walk = || {
        should_cancel() || local_volume.is_some_and(|scope| started.elapsed() >= scope.max_duration)
    };
    let guard = |path: &Path, metadata: &fs::Metadata| {
        if let Some(identity) = root_identity
            && path == root.as_path()
            && crate::local_search::required_directory_identity(path, metadata)
                .map_or(true, |current| current != identity)
        {
            root_changed.set(true);
            return Err(io::Error::other("검사 대상 폴더의 신원이 변경되었습니다"));
        }
        Ok(())
    };
    let prune = |path: &Path, metadata: &fs::Metadata| {
        let (Some(scope), Some(root_identity)) = (local_volume, root_identity) else {
            return false;
        };
        scope
            .excluded_roots
            .iter()
            .any(|excluded| path.starts_with(excluded))
            || crate::file_object_identity(path, metadata)
                .is_none_or(|identity| identity.identity.device != root_identity.device)
            || crate::scan_policy::ensure_local_volume(path).is_err()
    };
    let mut walk = if let Some(scope) = local_volume {
        crate::streaming_walk::StreamingWalk::with_directory_guard(
            &root,
            &stop_walk,
            &guard,
            scope.max_visited.saturating_add(1),
        )
        .with_directory_prune(&prune)
    } else {
        crate::streaming_walk::StreamingWalk::new(&root, &stop_walk)
    };
    loop {
        if local_volume.is_some_and(|scope| {
            started.elapsed() >= scope.max_duration
                || walk.visited_entries().saturating_sub(1) >= scope.max_visited
        }) {
            scope_incomplete = true;
            break;
        }
        let Some(item) = walk.next() else {
            break;
        };
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        if root_changed.get() {
            return Err(ScanError::Access(
                "검사 대상 폴더의 신원이 변경되었습니다".into(),
            ));
        }
        if local_volume.is_some() && walk.visit_limit_reached() {
            scope_incomplete = true;
            break;
        }
        if let Err(error) = &item
            && error.is_resource_limit()
        {
            return Err(ScanError::Access(error.to_string()));
        }
        if local_volume.is_some_and(|scope| started.elapsed() >= scope.max_duration) {
            scope_incomplete = true;
            break;
        }

        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                unreadable_entries = unreadable_entries.saturating_add(1);
                push_issue(
                    &mut issues,
                    max_issues,
                    error.path().map(|path| path.to_string_lossy().into_owned()),
                    error.to_string(),
                );
                continue;
            }
        };

        if entry.depth() == 0 {
            continue;
        }

        let path = entry.path();
        if let (Some(scope), Some(root_identity)) = (local_volume, root_identity)
            && (scope
                .excluded_roots
                .iter()
                .any(|excluded| path.starts_with(excluded))
                || entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| super::file_object_identity(&path, &metadata))
                    .is_none_or(|identity| identity.identity.device != root_identity.device))
        {
            scope_incomplete = true;
            unreadable_entries = unreadable_entries.saturating_add(1);
            continue;
        }
        processed_entries = processed_entries.saturating_add(1);

        if entry.is_empty_directory() {
            empty_directory_count = empty_directory_count.saturating_add(1);
            // Display name and lossy display path may expand invalid bytes.
            let retained_bytes = path.as_os_str().len().saturating_mul(6);
            if empty_directories.len() < max_empty_directories
                && empty_directory_path_bytes.saturating_add(retained_bytes)
                    <= MAX_EMPTY_DIRECTORY_PATH_BYTES
            {
                let metadata = entry.metadata().ok();
                let modified = metadata
                    .as_ref()
                    .and_then(|metadata| metadata.modified().ok());
                empty_directories.push(EmptyDirectory {
                    name: display_name(&path),
                    path: path.to_string_lossy().into_owned(),
                    modified_at_unix_ms: system_time_ms(modified),
                    scan_identity: metadata
                        .as_ref()
                        .and_then(|metadata| super::file_object_identity(&path, metadata)),
                    scan_modified_at: modified,
                });
                empty_directory_path_bytes += retained_bytes;
            }
        }

        let relative = path.strip_prefix(&root).unwrap_or(&path);
        let Some(first_component) = relative.components().next() else {
            continue;
        };
        let direct_path = root.join(first_component.as_os_str());
        let is_direct_child = entry.depth() == 1;
        let file_type = entry.file_type();
        if is_direct_child && (file_type.is_dir() || file_type.is_file()) {
            direct_child_count = direct_child_count.saturating_add(1);
        }

        if file_type.is_dir() {
            total_directories = total_directories.saturating_add(1);
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(error) => {
                    unreadable_entries = unreadable_entries.saturating_add(1);
                    push_issue(
                        &mut issues,
                        max_issues,
                        Some(path.to_string_lossy().into_owned()),
                        error.to_string(),
                    );
                    if let Some(child) = tracked_child(
                        &mut children,
                        direct_path,
                        max_tracked_children,
                        &mut tracked_child_path_bytes,
                        &mut tracking_limit_reached,
                    ) {
                        child.is_directory = true;
                        child.directory_count = child.directory_count.saturating_add(1);
                    }
                    continue;
                }
            };
            let modified_at = metadata.modified().ok();
            if let Some(child) = tracked_child(
                &mut children,
                direct_path,
                max_tracked_children,
                &mut tracked_child_path_bytes,
                &mut tracking_limit_reached,
            ) {
                child.is_directory = true;
                child.directory_count = child.directory_count.saturating_add(1);
                if is_direct_child {
                    child.scan_identity = super::file_object_identity(&path, &metadata);
                    child.scan_modified_at = modified_at;
                }
                update_latest(&mut child.modified_at, modified_at);
            }
        } else if file_type.is_file() {
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(error) => {
                    unreadable_entries = unreadable_entries.saturating_add(1);
                    push_issue(
                        &mut issues,
                        max_issues,
                        Some(path.to_string_lossy().into_owned()),
                        error.to_string(),
                    );
                    continue;
                }
            };
            if crate::scan_policy::is_online_only(&metadata) {
                if is_direct_child {
                    direct_child_count = direct_child_count.saturating_sub(1);
                }
                continue;
            }
            let logical_bytes = metadata.len();
            let modified_at = metadata.modified().ok();
            if let Some(child) = tracked_child(
                &mut children,
                direct_path,
                max_tracked_children,
                &mut tracked_child_path_bytes,
                &mut tracking_limit_reached,
            ) {
                child.is_directory = !is_direct_child;
                child.logical_bytes = child.logical_bytes.saturating_add(logical_bytes);
                child.file_count = child.file_count.saturating_add(1);
                if is_direct_child {
                    child.scan_identity = super::file_object_identity(&path, &metadata);
                    child.scan_modified_at = modified_at;
                }
                update_latest(&mut child.modified_at, modified_at);
            }

            total_files = total_files.saturating_add(1);
            total_logical_bytes = total_logical_bytes.saturating_add(logical_bytes);
        }

        if processed_entries.is_multiple_of(DIRECTORY_PROGRESS_ENTRY_INTERVAL) {
            on_progress(DirectoryScanProgress {
                message: format!("{processed_entries}개 항목의 폴더 용량을 계산했습니다"),
                processed_entries,
                processed_files: total_files,
                processed_bytes: total_logical_bytes,
                unreadable_entries,
            });
        }
    }

    if local_volume.is_some() {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        let excluded_entries = walk.excluded_entries();
        if excluded_entries != 0 {
            scope_incomplete = true;
            unreadable_entries = unreadable_entries.saturating_add(excluded_entries);
            push_issue(
                &mut issues,
                max_issues,
                Some(root.to_string_lossy().into_owned()),
                "마운트 경계, 클라우드 또는 온라인 전용 항목을 제외한 부분 지도입니다. 삭제 검토에 사용할 수 없습니다".into(),
            );
        }
        scope_incomplete |= unreadable_entries != 0;
        drop(walk);
        let revalidated = crate::validate_local_volume_directory(requested_root)?;
        let metadata =
            fs::symlink_metadata(&root).map_err(|error| ScanError::Access(error.to_string()))?;
        if revalidated != root
            || Some(crate::local_search::required_directory_identity(
                &root, &metadata,
            )?) != root_identity
        {
            return Err(ScanError::Access(
                "검사 대상 폴더의 신원이 변경되었습니다".into(),
            ));
        }
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        scope_incomplete |=
            local_volume.is_some_and(|scope| started.elapsed() >= scope.max_duration);
        if scope_incomplete {
            tracking_limit_reached = true;
            unreadable_entries = unreadable_entries.max(1);
            push_issue(
                &mut issues,
                max_issues,
                Some(root.to_string_lossy().into_owned()),
                "로컬 저장장치 검사를 일부만 완료했습니다. 용량과 항목 수는 확인한 범위의 합계이며 삭제 검토에 사용할 수 없습니다".into(),
            );
        }
    }

    on_progress(DirectoryScanProgress {
        message: "비례사각형과 빈 폴더 목록을 정리하고 있습니다".to_owned(),
        processed_entries,
        processed_files: total_files,
        processed_bytes: total_logical_bytes,
        unreadable_entries,
    });

    empty_directories.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    let empty_directories_truncated =
        scope_incomplete || empty_directory_count > empty_directories.len() as u64;

    let mut children: Vec<DirectoryNode> = children
        .into_values()
        .map(|child| DirectoryNode {
            name: display_name(&child.path),
            path: child.path.to_string_lossy().into_owned(),
            logical_bytes: child.logical_bytes,
            file_count: child.file_count,
            directory_count: child.directory_count,
            is_directory: child.is_directory,
            modified_at_unix_ms: system_time_ms(child.modified_at),
            scan_identity: child.scan_identity,
            scan_modified_at: child.scan_modified_at,
        })
        .collect();
    children.sort_unstable_by(|left, right| {
        right
            .logical_bytes
            .cmp(&left.logical_bytes)
            .then_with(|| right.is_directory.cmp(&left.is_directory))
            .then_with(|| left.path.cmp(&right.path))
    });

    children.truncate(max_children);
    let returned_child_bytes = children.iter().fold(0_u64, |total, child| {
        total.saturating_add(child.logical_bytes)
    });
    let children_truncated = scope_incomplete || direct_child_count > children.len() as u64;
    let omitted_child_count = direct_child_count.saturating_sub(children.len() as u64);
    let omitted_logical_bytes = total_logical_bytes.saturating_sub(returned_child_bytes);

    if local_volume.is_some() && tracking_limit_reached {
        // A partial map is display-only. Strip in-process action authority too;
        // callers must reject its flags before starting subtree revalidation.
        for child in &mut children {
            child.scan_identity = None;
            child.scan_modified_at = None;
        }
        for directory in &mut empty_directories {
            directory.scan_identity = None;
            directory.scan_modified_at = None;
        }
    }
    if tracking_limit_reached && !scope_incomplete {
        push_issue(
            &mut issues,
            max_issues,
            Some(root.to_string_lossy().into_owned()),
            "직계 항목 집계가 메모리 안전 상한에 도달해 나머지는 합계로만 계산했습니다".to_owned(),
        );
    }

    Ok(DirectoryScanReport {
        root: root.to_string_lossy().into_owned(),
        name: display_name(&root),
        parent: root
            .parent()
            .map(|path| path.to_string_lossy().into_owned()),
        completed_at_unix_ms: system_time_ms(Some(SystemTime::now())).unwrap_or_default(),
        duration_ms: started.elapsed().as_millis(),
        total_logical_bytes,
        total_files,
        total_directories,
        direct_child_count,
        children_truncated,
        tracking_limit_reached,
        omitted_child_count,
        omitted_logical_bytes,
        empty_directory_count,
        empty_directories_truncated,
        unreadable_entries,
        children,
        empty_directories,
        issues,
    })
}

fn tracked_child<'a>(
    children: &'a mut HashMap<PathBuf, NodeAccumulator>,
    path: PathBuf,
    limit: usize,
    retained_path_bytes: &mut usize,
    limit_reached: &mut bool,
) -> Option<&'a mut NodeAccumulator> {
    let path_bytes = path.as_os_str().len().saturating_mul(6);
    let can_insert = children.len() < limit
        && retained_path_bytes.saturating_add(path_bytes) <= MAX_TRACKED_CHILD_PATH_BYTES;
    match children.entry(path) {
        Entry::Occupied(entry) => Some(entry.into_mut()),
        Entry::Vacant(entry) if can_insert => {
            *retained_path_bytes += path_bytes;
            let path = entry.key().clone();
            Some(entry.insert(NodeAccumulator {
                path,
                ..NodeAccumulator::default()
            }))
        }
        Entry::Vacant(_) => {
            *limit_reached = true;
            None
        }
    }
}

fn update_latest(current: &mut Option<SystemTime>, candidate: Option<SystemTime>) {
    if candidate > *current {
        *current = candidate;
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn local_volume_map_keeps_complete_local_children_actionable() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join("folder")).unwrap();
        fs::write(root.join("folder/file"), b"local").unwrap();
        let report = scan_local_volume_directory_level(
            &root,
            DirectoryScanConfig::default(),
            &[],
            |_| {},
            || false,
        )
        .unwrap();
        assert_eq!(report.total_files, 1);
        assert_eq!(report.total_logical_bytes, 5);
        assert!(!report.tracking_limit_reached);
        assert_eq!(report.unreadable_entries, 0);
        assert!(report.children[0].scan_identity.is_some());
    }

    #[test]
    fn local_volume_map_prunes_boundaries_and_returns_a_read_only_partial_map() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let excluded = root.join("other-volume");
        fs::create_dir(&excluded).unwrap();
        fs::write(excluded.join("private"), b"excluded").unwrap();
        fs::create_dir_all(root.join("Library/CloudStorage/provider")).unwrap();
        fs::write(root.join("Library/CloudStorage/provider/private"), b"cloud").unwrap();
        fs::create_dir(root.join("empty")).unwrap();
        fs::write(root.join("visible"), b"local").unwrap();
        let report = scan_local_volume_directory_level(
            &root,
            DirectoryScanConfig::default(),
            &[excluded],
            |_| {},
            || false,
        )
        .unwrap();
        assert_eq!(report.total_files, 1);
        assert_eq!(report.total_logical_bytes, 5);
        assert!(report.tracking_limit_reached);
        assert!(report.children_truncated);
        assert!(report.empty_directories_truncated);
        assert_eq!(report.unreadable_entries, 2);
        assert!(
            report
                .children
                .iter()
                .all(|child| child.scan_identity.is_none())
        );
        assert!(
            report
                .empty_directories
                .iter()
                .all(|directory| directory.scan_identity.is_none())
        );
        assert!(
            report
                .children
                .iter()
                .all(|child| child.name != "other-volume")
        );

        let previous =
            scan_directory_level(&root, DirectoryScanConfig::default(), |_| {}, || false).unwrap();
        assert_eq!(
            previous.total_files, 2,
            "existing engine scope must remain unchanged"
        );
        assert!(!previous.tracking_limit_reached);
    }

    #[test]
    fn local_volume_map_visit_and_time_budgets_return_partial_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::write(root.join("one"), []).unwrap();
        fs::write(root.join("two"), []).unwrap();
        for scope in [
            LocalVolumeScope {
                excluded_roots: &[],
                max_visited: 1,
                max_duration: MAX_LOCAL_VOLUME_DURATION,
            },
            LocalVolumeScope {
                excluded_roots: &[],
                max_visited: MAX_LOCAL_VOLUME_VISITED,
                max_duration: Duration::ZERO,
            },
        ] {
            let report = scan_directory_level_inner(
                &root,
                DirectoryScanConfig::default(),
                |_| {},
                || false,
                Some(scope),
            )
            .unwrap();
            assert!(report.total_files <= 1);
            assert!(report.tracking_limit_reached);
            assert!(report.children_truncated);
            assert!(report.unreadable_entries > 0);
        }
        assert!(matches!(
            scan_local_volume_directory_level(
                &root,
                DirectoryScanConfig::default(),
                &[],
                |_| {},
                || true
            ),
            Err(ScanError::Cancelled)
        ));
    }

    #[test]
    fn local_volume_map_rejects_root_replacement_before_enumeration() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("root");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("file"), []).unwrap();
        let result = scan_local_volume_directory_level(
            &root,
            DirectoryScanConfig::default(),
            &[],
            |_| {
                fs::rename(&root, base.join("previous-root")).unwrap();
                fs::create_dir(&root).unwrap();
            },
            || false,
        );
        assert!(matches!(result, Err(ScanError::Access(_))));
    }

    #[test]
    fn aggregates_direct_children_and_finds_strictly_empty_directories() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let alpha = temp.path().join("alpha");
        let nested = alpha.join("nested");
        let empty = temp.path().join("empty");
        let empty_parent = temp.path().join("empty-parent");
        let empty_leaf = empty_parent.join("empty-leaf");
        fs::create_dir_all(&nested).expect("create nested directory");
        fs::create_dir_all(&empty).expect("create empty directory");
        fs::create_dir_all(&empty_leaf).expect("create empty leaf");
        fs::write(alpha.join("alpha.bin"), vec![b'a'; 11]).expect("write alpha file");
        fs::write(nested.join("nested.bin"), vec![b'b'; 17]).expect("write nested file");
        fs::write(temp.path().join("root.bin"), vec![b'c'; 5]).expect("write root file");

        let report = scan_directory_level(
            temp.path(),
            DirectoryScanConfig::default(),
            |_| {},
            || false,
        )
        .expect("scan directory level");

        let alpha_node = report
            .children
            .iter()
            .find(|node| node.name == "alpha")
            .expect("alpha node");
        assert!(alpha_node.is_directory);
        assert_eq!(alpha_node.logical_bytes, 28);
        assert_eq!(alpha_node.file_count, 2);
        assert_eq!(report.total_logical_bytes, 33);
        assert_eq!(report.total_files, 3);
        assert_eq!(report.empty_directory_count, 2);
        assert!(
            report
                .empty_directories
                .iter()
                .any(|directory| directory.name == "empty")
        );
        assert!(
            report
                .empty_directories
                .iter()
                .any(|directory| directory.name == "empty-leaf")
        );
        assert!(
            report
                .empty_directories
                .iter()
                .all(|directory| directory.name != "empty-parent")
        );
    }

    #[test]
    fn limits_payload_without_losing_omitted_totals() {
        let temp = tempfile::tempdir().expect("create temp directory");
        fs::create_dir(temp.path().join("empty-a")).expect("create empty a");
        fs::create_dir(temp.path().join("empty-b")).expect("create empty b");
        fs::write(temp.path().join("large.bin"), vec![0_u8; 20]).expect("write large");
        fs::write(temp.path().join("small.bin"), vec![0_u8; 7]).expect("write small");

        let report = scan_directory_level(
            temp.path(),
            DirectoryScanConfig {
                max_children: 1,
                max_tracked_children: 8,
                max_empty_directories: 1,
                max_issues: 10,
            },
            |_| {},
            || false,
        )
        .expect("scan directory level");

        assert!(report.children_truncated);
        assert_eq!(report.children.len(), 1);
        assert_eq!(report.omitted_child_count, 3);
        assert_eq!(report.omitted_logical_bytes, 7);
        assert_eq!(report.empty_directory_count, 2);
        assert!(report.empty_directories_truncated);
        assert_eq!(report.empty_directories.len(), 1);
    }

    #[test]
    fn bounds_direct_child_tracking_and_keeps_global_totals() {
        let temp = tempfile::tempdir().expect("create temp directory");
        for index in 0..100_u64 {
            fs::write(temp.path().join(format!("{index:03}.bin")), vec![0_u8; 10])
                .expect("write direct file");
        }

        let report = scan_directory_level(
            temp.path(),
            DirectoryScanConfig {
                max_children: 4,
                max_tracked_children: 4,
                max_empty_directories: 1,
                max_issues: 10,
            },
            |_| {},
            || false,
        )
        .expect("scan directory level");

        assert!(report.tracking_limit_reached);
        assert!(report.children_truncated);
        assert_eq!(report.children.len(), 4);
        assert_eq!(report.direct_child_count, 100);
        assert_eq!(report.omitted_child_count, 96);
        assert_eq!(report.total_files, 100);
        assert_eq!(report.total_logical_bytes, 1_000);
        assert_eq!(report.omitted_logical_bytes, 960);
    }

    #[test]
    fn honours_directory_scan_cancellation() {
        let temp = tempfile::tempdir().expect("create temp directory");
        fs::write(temp.path().join("file.bin"), b"content").expect("write file");

        let result =
            scan_directory_level(temp.path(), DirectoryScanConfig::default(), |_| {}, || true);

        assert!(matches!(result, Err(ScanError::Cancelled)));
    }

    #[test]
    fn child_path_budget_rejects_new_keys_but_keeps_existing_totals() {
        let mut children = HashMap::new();
        let mut bytes = 0;
        let mut limited = false;
        let path = PathBuf::from("existing");
        tracked_child(&mut children, path.clone(), 10, &mut bytes, &mut limited)
            .unwrap()
            .file_count = 1;
        bytes = MAX_TRACKED_CHILD_PATH_BYTES;
        assert!(
            tracked_child(
                &mut children,
                PathBuf::from("new"),
                10,
                &mut bytes,
                &mut limited
            )
            .is_none()
        );
        assert!(limited);
        tracked_child(&mut children, path.clone(), 10, &mut bytes, &mut limited)
            .unwrap()
            .file_count += 1;
        assert_eq!(children.len(), 1);
        assert_eq!(children[&path].file_count, 2);
    }

    #[test]
    fn folder_snapshot_keeps_own_identity_and_mtime_separate_from_display_rollup() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("folder");
        fs::create_dir(&folder).unwrap();
        let nested = folder.join("file");
        fs::write(&nested, b"fixture").unwrap();
        let future = SystemTime::now() + std::time::Duration::from_secs(3_600);
        fs::OpenOptions::new()
            .write(true)
            .open(&nested)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(future))
            .unwrap();
        let metadata = fs::symlink_metadata(&folder).unwrap();
        let report = scan_directory_level(
            temp.path(),
            DirectoryScanConfig::default(),
            |_| {},
            || false,
        )
        .unwrap();
        let node = report
            .children
            .iter()
            .find(|node| node.name == "folder")
            .unwrap();
        assert_eq!(
            node.scan_identity,
            super::super::file_object_identity(&folder, &metadata)
        );
        assert_eq!(node.scan_modified_at, metadata.modified().ok());
        assert!(node.modified_at_unix_ms > system_time_ms(node.scan_modified_at));
    }
}
