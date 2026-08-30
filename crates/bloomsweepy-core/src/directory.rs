use serde::{Deserialize, Serialize};
use std::collections::{HashMap, hash_map::Entry};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use super::{ScanError, ScanIssue, bounded_worker_threads, push_issue, system_time_ms};

const DIRECTORY_PROGRESS_ENTRY_INTERVAL: u64 = 2_048;
const MAX_TRACKED_CHILDREN: usize = 65_536;
const MAX_EMPTY_DIRECTORY_RESULTS: usize = 10_000;
const MAX_DIRECTORY_ISSUES: usize = 1_000;

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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyDirectory {
    pub name: String,
    pub path: String,
    pub modified_at_unix_ms: Option<u128>,
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
}

pub fn scan_directory_level<F, C>(
    root: impl AsRef<Path>,
    config: DirectoryScanConfig,
    mut on_progress: F,
    should_cancel: C,
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
    let requested_root = root.as_ref();

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

    let root = requested_root
        .canonicalize()
        .map_err(|error| ScanError::Access(error.to_string()))?;

    on_progress(DirectoryScanProgress {
        message: "현재 폴더의 용량 지도를 만들고 있습니다".to_owned(),
        processed_entries: 0,
        processed_files: 0,
        processed_bytes: 0,
        unreadable_entries: 0,
    });

    let mut children: HashMap<PathBuf, NodeAccumulator> = HashMap::new();
    let empty_directory_count = Arc::new(AtomicU64::new(0));
    let empty_directory_paths = Arc::new(Mutex::new(Vec::new()));
    let empty_count_for_walker = Arc::clone(&empty_directory_count);
    let empty_paths_for_walker = Arc::clone(&empty_directory_paths);
    let mut issues = Vec::new();
    let mut processed_entries = 0_u64;
    let mut total_files = 0_u64;
    let mut total_directories = 0_u64;
    let mut direct_child_count = 0_u64;
    let mut total_logical_bytes = 0_u64;
    let mut unreadable_entries = 0_u64;
    let mut tracking_limit_reached = false;

    let walker = jwalk::WalkDir::new(&root)
        .follow_links(false)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(bounded_worker_threads()))
        .process_read_dir(move |depth, path, _, entries| {
            if depth.is_some_and(|depth| depth > 0) && entries.is_empty() {
                empty_count_for_walker.fetch_add(1, Ordering::Relaxed);
                if let Ok(mut paths) = empty_paths_for_walker.lock()
                    && paths.len() < max_empty_directories
                {
                    paths.push(path.to_path_buf());
                }
            }
        });

    for item in walker {
        if should_cancel() {
            return Err(ScanError::Cancelled);
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
        processed_entries = processed_entries.saturating_add(1);

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
                &mut tracking_limit_reached,
            ) {
                child.is_directory = true;
                child.directory_count = child.directory_count.saturating_add(1);
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
            let logical_bytes = metadata.len();
            let modified_at = metadata.modified().ok();
            if let Some(child) = tracked_child(
                &mut children,
                direct_path,
                max_tracked_children,
                &mut tracking_limit_reached,
            ) {
                child.is_directory = !is_direct_child;
                child.logical_bytes = child.logical_bytes.saturating_add(logical_bytes);
                child.file_count = child.file_count.saturating_add(1);
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

    on_progress(DirectoryScanProgress {
        message: "비례사각형과 빈 폴더 목록을 정리하고 있습니다".to_owned(),
        processed_entries,
        processed_files: total_files,
        processed_bytes: total_logical_bytes,
        unreadable_entries,
    });

    let empty_directory_count = empty_directory_count.load(Ordering::Relaxed);
    let mut empty_directories: Vec<EmptyDirectory> = empty_directory_paths
        .lock()
        .map(|paths| paths.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|path| EmptyDirectory {
            name: display_name(&path),
            path: path.to_string_lossy().into_owned(),
            modified_at_unix_ms: system_time_ms(
                std::fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .ok(),
            ),
        })
        .collect();
    empty_directories.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    let empty_directories_truncated = empty_directory_count > empty_directories.len() as u64;

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
    let children_truncated = direct_child_count > children.len() as u64;
    let omitted_child_count = direct_child_count.saturating_sub(children.len() as u64);
    let omitted_logical_bytes = total_logical_bytes.saturating_sub(returned_child_bytes);

    if tracking_limit_reached {
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
    limit_reached: &mut bool,
) -> Option<&'a mut NodeAccumulator> {
    let can_insert = children.len() < limit;
    match children.entry(path) {
        Entry::Occupied(entry) => Some(entry.into_mut()),
        Entry::Vacant(entry) if can_insert => {
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
}
