//! Small metadata-only name search, sharing the scanner's demand-driven walker.
use crate::{DirectoryNode, ScanError, system_time_ms};
use serde::Serialize;
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

const MAX_RESULTS: usize = 200;
const MAX_RESULT_PATH_BYTES: usize = 2 * 1024 * 1024;
const MAX_VISITED: u64 = 250_000;
const MAX_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSearchReport {
    pub entries: Vec<DirectoryNode>,
    pub visited_entries: u64,
    pub unreadable_entries: u64,
    pub truncated: bool,
    pub completed_at_unix_ms: u128,
}

/// Reject linked ancestors BEFORE canonicalization (including Windows junctions).
/// This does not eliminate path-based OS races; trash revalidates again separately.
pub fn validate_local_directory_path(path: &Path) -> Result<PathBuf, ScanError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(ScanError::Access(
            "절대 로컬 폴더 경로만 사용할 수 있습니다".into(),
        ));
    }
    crate::scan_policy::ensure_local_path(path).map_err(ScanError::Access)?;
    for ancestor in path.ancestors() {
        crate::scan_policy::ensure_local_path(ancestor).map_err(ScanError::Access)?;
        let metadata =
            fs::symlink_metadata(ancestor).map_err(|error| ScanError::Access(error.to_string()))?;
        if metadata.file_type().is_symlink() || crate::scan_policy::is_reparse_point(&metadata) {
            return Err(ScanError::Access(
                "링크 또는 연결 경로는 대화 작업 대상으로 사용할 수 없습니다".into(),
            ));
        }
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| ScanError::Access(error.to_string()))?;
    crate::scan_policy::ensure_local_path(&canonical).map_err(ScanError::Access)?;
    if !canonical.is_dir() {
        return Err(ScanError::NotDirectory(path.display().to_string()));
    }
    Ok(canonical)
}

/// Validate a root for searches confined to a single local mounted volume.
pub fn validate_local_volume_directory(path: &Path) -> Result<PathBuf, ScanError> {
    crate::scan_policy::reject_network_path(path)?;
    let canonical = validate_local_directory_path(path)?;
    crate::scan_policy::reject_network_path(&canonical)?;
    crate::scan_policy::ensure_local_volume(&canonical)?;
    Ok(canonical)
}

/// Metadata-only name search that stays on the root's local mounted volume.
/// Caller limits may lower, but never raise, the operation's safety ceilings.
/// Time limits are cooperative between filesystem metadata operations.
#[allow(clippy::too_many_arguments)]
pub fn search_local_volume_entries<C>(
    requested_root: &Path,
    query: &str,
    excluded_roots: &[PathBuf],
    max_visited: u64,
    max_results: usize,
    max_path_bytes: usize,
    deadline: Instant,
    should_cancel: C,
) -> Result<LocalSearchReport, ScanError>
where
    C: Fn() -> bool,
{
    let started = Instant::now();
    if should_cancel() {
        return Err(ScanError::Cancelled);
    }
    let query = query.trim().to_lowercase();
    if query.is_empty() || query.chars().count() > 240 || query.contains(['/', '\\', '\0']) {
        return Err(ScanError::Access(
            "검색은 경로 대신 240자 이하의 이름 일부를 사용하세요".into(),
        ));
    }
    let root = validate_local_volume_directory(requested_root)?;
    let root_metadata =
        fs::symlink_metadata(&root).map_err(|error| ScanError::Access(error.to_string()))?;
    let root_identity = required_directory_identity(&root, &root_metadata)?;
    let max_visited = max_visited.min(MAX_VISITED);
    let max_results = max_results.min(MAX_RESULTS);
    let max_path_bytes = max_path_bytes.min(MAX_RESULT_PATH_BYTES);
    let deadline = deadline.min(started + MAX_DURATION);
    let mut report = LocalSearchReport {
        entries: Vec::new(),
        visited_entries: 0,
        unreadable_entries: 0,
        truncated: false,
        completed_at_unix_ms: 0,
    };
    let mut path_bytes = 0_usize;
    let stop_walk = || should_cancel() || Instant::now() >= deadline;
    let root_changed = std::cell::Cell::new(false);
    let guard = |path: &Path, metadata: &fs::Metadata| {
        if path == root.as_path()
            && required_directory_identity(path, metadata)
                .map_or(true, |identity| identity != root_identity)
        {
            root_changed.set(true);
            return Err(io::Error::other("검색 대상 폴더의 신원이 변경되었습니다"));
        }
        Ok(())
    };
    let prune = |path: &Path, metadata: &fs::Metadata| {
        excluded_roots
            .iter()
            .any(|excluded| path.starts_with(excluded))
            || crate::file_object_identity(path, metadata)
                .is_none_or(|identity| identity.identity.device != root_identity.device)
            || crate::scan_policy::ensure_local_volume(path).is_err()
    };
    // The walker counts the root as well as excluded entries. Reserve one
    // root visit so the public budget describes entries below the root.
    let mut walk = crate::streaming_walk::StreamingWalk::with_directory_guard(
        &root,
        &stop_walk,
        &guard,
        max_visited as usize + 1,
    )
    .with_directory_prune(&prune);
    loop {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        if Instant::now() >= deadline
            || report.entries.len() >= max_results
            || path_bytes >= max_path_bytes
            || walk.visited_entries().saturating_sub(1) as u64 >= max_visited
        {
            report.truncated = true;
            break;
        }
        let Some(item) = walk.next() else {
            break;
        };
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        if root_changed.get() {
            return Err(root_changed_error());
        }
        if walk.visit_limit_reached() {
            report.truncated = true;
            break;
        }
        if let Err(error) = &item
            && error.is_resource_limit()
        {
            return Err(ScanError::Access(error.to_string()));
        }
        if Instant::now() >= deadline {
            report.truncated = true;
            break;
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => {
                report.unreadable_entries = report.unreadable_entries.saturating_add(1);
                continue;
            }
        };
        if entry.depth() == 0 || (!entry.file_type().is_file() && !entry.file_type().is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().contains(&query) {
            continue;
        }
        let path = entry.path();
        let bytes = path.as_os_str().len().saturating_mul(6);
        if path_bytes.saturating_add(bytes) > max_path_bytes {
            report.truncated = true;
            break;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| ScanError::Access(error.to_string()))?;
        let identity = crate::file_object_identity(&path, &metadata);
        if identity.is_none_or(|identity| identity.identity.device != root_identity.device) {
            continue;
        }
        let modified = metadata.modified().ok();
        let is_directory = metadata.is_dir();
        report.entries.push(DirectoryNode {
            name,
            path: path.to_string_lossy().into_owned(),
            logical_bytes: if is_directory { 0 } else { metadata.len() },
            file_count: u64::from(!is_directory),
            directory_count: u64::from(is_directory),
            is_directory,
            modified_at_unix_ms: system_time_ms(modified),
            scan_identity: identity,
            scan_modified_at: modified,
        });
        path_bytes = path_bytes.saturating_add(bytes);
    }
    report.visited_entries = (walk.visited_entries().saturating_sub(1) as u64).min(max_visited);
    drop(walk);
    if should_cancel() {
        return Err(ScanError::Cancelled);
    }
    let revalidated = validate_local_volume_directory(requested_root)?;
    let metadata =
        fs::symlink_metadata(&root).map_err(|error| ScanError::Access(error.to_string()))?;
    if revalidated != root || required_directory_identity(&root, &metadata)? != root_identity {
        return Err(root_changed_error());
    }
    if should_cancel() {
        return Err(ScanError::Cancelled);
    }
    report.truncated |= Instant::now() >= deadline;
    report.completed_at_unix_ms = system_time_ms(Some(SystemTime::now())).unwrap_or_default();
    Ok(report)
}

pub(crate) fn required_directory_identity(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<crate::FileIdentity, ScanError> {
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || crate::scan_policy::is_reparse_point(metadata)
        || crate::scan_policy::is_online_only(metadata)
    {
        return Err(root_changed_error());
    }
    crate::file_object_identity(path, metadata)
        .map(|identity| identity.identity)
        .ok_or_else(root_changed_error)
}

fn root_changed_error() -> ScanError {
    ScanError::Access("검색 대상 폴더의 신원이 변경되었습니다. 다시 검색해 주세요".into())
}

pub fn search_local_entries<C>(
    root: &Path,
    query: &str,
    should_cancel: C,
) -> Result<LocalSearchReport, ScanError>
where
    C: Fn() -> bool,
{
    let query = query.trim().to_lowercase();
    if query.is_empty() || query.chars().count() > 240 || query.contains(['/', '\\', '\0']) {
        return Err(ScanError::Access(
            "검색은 경로 대신 240자 이하의 이름 일부를 사용하세요".into(),
        ));
    }
    let root = validate_local_directory_path(root)?;
    let started = Instant::now();
    let mut report = LocalSearchReport {
        entries: Vec::new(),
        visited_entries: 0,
        unreadable_entries: 0,
        truncated: false,
        completed_at_unix_ms: 0,
    };
    let mut path_bytes = 0;
    for item in crate::streaming_walk::StreamingWalk::new(&root, &should_cancel) {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        if report.visited_entries >= MAX_VISITED || started.elapsed() >= MAX_DURATION {
            report.truncated = true;
            break;
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(error) if error.is_resource_limit() => {
                return Err(ScanError::Access(error.to_string()));
            }
            Err(_) => {
                report.unreadable_entries += 1;
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        report.visited_entries += 1;
        if !entry.file_type().is_file() && !entry.file_type().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().contains(&query) {
            continue;
        }
        let path = entry.path();
        let bytes = path.as_os_str().len().saturating_mul(6);
        if report.entries.len() >= MAX_RESULTS || path_bytes + bytes > MAX_RESULT_PATH_BYTES {
            report.truncated = true;
            break;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| ScanError::Access(error.to_string()))?;
        let modified = metadata.modified().ok();
        let is_directory = metadata.is_dir();
        report.entries.push(DirectoryNode {
            name,
            path: path.to_string_lossy().into_owned(),
            logical_bytes: if is_directory { 0 } else { metadata.len() },
            file_count: u64::from(!is_directory),
            directory_count: u64::from(is_directory),
            is_directory,
            modified_at_unix_ms: system_time_ms(modified),
            scan_identity: crate::file_object_identity(&path, &metadata),
            scan_modified_at: modified,
        });
        path_bytes += bytes;
    }
    if should_cancel() {
        return Err(ScanError::Cancelled);
    }
    report.completed_at_unix_ms = system_time_ms(Some(SystemTime::now())).unwrap_or_default();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume_search(
        root: &Path,
        excluded: &[PathBuf],
        max_visited: u64,
        max_results: usize,
        max_path_bytes: usize,
        deadline: Instant,
    ) -> LocalSearchReport {
        search_local_volume_entries(
            root,
            "match",
            excluded,
            max_visited,
            max_results,
            max_path_bytes,
            deadline,
            || false,
        )
        .unwrap()
    }

    #[test]
    fn volume_search_prunes_nested_roots_and_counts_excluded_visits() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let excluded = root.join("other-volume");
        fs::create_dir(&excluded).unwrap();
        fs::write(excluded.join("match-private"), []).unwrap();
        fs::create_dir_all(root.join("Library/CloudStorage/provider")).unwrap();
        fs::write(root.join("Library/CloudStorage/provider/match-cloud"), []).unwrap();
        fs::write(root.join("match-visible"), []).unwrap();
        let found = volume_search(
            &root,
            &[excluded],
            MAX_VISITED,
            MAX_RESULTS,
            MAX_RESULT_PATH_BYTES,
            Instant::now() + MAX_DURATION,
        );
        assert_eq!(found.entries.len(), 1);
        assert_eq!(found.entries[0].name, "match-visible");
        assert_eq!(found.visited_entries, 4);
        assert!(!found.truncated);
    }

    #[test]
    fn volume_search_returns_partial_reports_for_each_budget() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::write(root.join("match-one"), []).unwrap();
        fs::write(root.join("match-two"), []).unwrap();
        let deadline = Instant::now() + MAX_DURATION;
        let visited = volume_search(&root, &[], 1, MAX_RESULTS, MAX_RESULT_PATH_BYTES, deadline);
        assert_eq!(visited.visited_entries, 1);
        assert!(visited.truncated);
        let results = volume_search(&root, &[], MAX_VISITED, 1, MAX_RESULT_PATH_BYTES, deadline);
        assert_eq!(results.entries.len(), 1);
        assert!(results.truncated);
        let paths = volume_search(&root, &[], MAX_VISITED, MAX_RESULTS, 1, deadline);
        assert!(paths.entries.is_empty());
        assert!(paths.truncated);
        let time = volume_search(
            &root,
            &[],
            MAX_VISITED,
            MAX_RESULTS,
            MAX_RESULT_PATH_BYTES,
            Instant::now(),
        );
        assert_eq!(time.visited_entries, 0);
        assert!(time.entries.is_empty());
        assert!(time.truncated);
        assert!(matches!(
            search_local_volume_entries(&root, "match", &[], 0, 0, 0, Instant::now(), || true,),
            Err(ScanError::Cancelled)
        ));
    }

    #[test]
    fn volume_search_cannot_raise_result_safety_ceiling() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        for index in 0..205 {
            fs::write(root.join(format!("match-{index}")), []).unwrap();
        }
        let found = volume_search(
            &root,
            &[],
            u64::MAX,
            usize::MAX,
            usize::MAX,
            Instant::now() + MAX_DURATION,
        );
        assert_eq!(found.entries.len(), MAX_RESULTS);
        assert!(found.truncated);
    }

    #[test]
    fn volume_search_rejects_a_root_replaced_during_enumeration() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("root");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("match-one"), []).unwrap();
        let calls = std::cell::Cell::new(0_u32);
        let search = search_local_volume_entries(
            &root,
            "match",
            &[],
            MAX_VISITED,
            MAX_RESULTS,
            MAX_RESULT_PATH_BYTES,
            Instant::now() + MAX_DURATION,
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 5 {
                    fs::rename(&root, base.join("previous-root")).unwrap();
                    fs::create_dir(&root).unwrap();
                }
                false
            },
        );
        assert!(matches!(search, Err(ScanError::Access(_))));
    }

    #[test]
    fn recursive_search_is_bounded_and_does_not_enter_clouds() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested/Promo-video.mp4"), "fixture").unwrap();
        fs::create_dir_all(temp.path().join("Library/CloudStorage/OneDrive-Test")).unwrap();
        fs::write(
            temp.path()
                .join("Library/CloudStorage/OneDrive-Test/Promo-secret.txt"),
            "private",
        )
        .unwrap();
        let root = temp.path().canonicalize().unwrap();
        let found = search_local_entries(&root, "promo", || false).unwrap();
        assert_eq!(found.entries.len(), 1);
        assert_eq!(found.entries[0].name, "Promo-video.mp4");
        for index in 0..205 {
            fs::write(root.join(format!("match-{index}")), "a").unwrap();
        }
        let found = search_local_entries(&root, "match", || false).unwrap();
        assert_eq!(found.entries.len(), MAX_RESULTS);
        assert!(found.truncated);
        assert!(search_local_entries(&root, "../", || false).is_err());
        assert!(matches!(
            search_local_entries(&root, "promo", || true),
            Err(ScanError::Cancelled)
        ));
    }
    #[cfg(unix)]
    #[test]
    fn linked_roots_and_ancestors_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join("real")).unwrap();
        fs::create_dir(root.join("real/child")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();
        assert!(search_local_entries(&root.join("link/child"), "x", || false).is_err());
        assert!(validate_local_directory_path(&root.join("link")).is_err());
    }
}
