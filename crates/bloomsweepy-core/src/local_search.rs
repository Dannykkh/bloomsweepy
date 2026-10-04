//! Small metadata-only name search, sharing the scanner's demand-driven walker.
use crate::{DirectoryNode, ScanError, system_time_ms};
use serde::Serialize;
use std::{
    fs,
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
