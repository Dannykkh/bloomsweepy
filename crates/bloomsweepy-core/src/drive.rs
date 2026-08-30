use super::{ScanError, ScanIssue, push_issue, system_time_ms};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

const DRIVE_PROGRESS_FILE_INTERVAL: u64 = 4_096;
const MAX_TRACKED_LOCATIONS: usize = 65_536;
const MAX_DRIVE_LOCATION_RESULTS: usize = 10_000;
const MAX_DRIVE_ISSUES: usize = 1_000;
#[cfg(not(windows))]
const MAX_DRIVE_HARD_LINK_IDENTITIES: usize = 250_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DriveScanConfig {
    pub max_locations: usize,
    pub max_tracked_locations: usize,
    pub max_issues: usize,
    pub location_depth: usize,
}

impl Default for DriveScanConfig {
    fn default() -> Self {
        Self {
            max_locations: 40,
            max_tracked_locations: 16_384,
            max_issues: 100,
            location_depth: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum StorageCategoryKind {
    Applications,
    System,
    TemporaryFiles,
    RecycleBin,
    Desktop,
    Documents,
    Downloads,
    Photos,
    Videos,
    Audio,
    Archives,
    Developer,
    OtherUsers,
    Other,
}

const STORAGE_CATEGORY_ORDER: [StorageCategoryKind; 14] = [
    StorageCategoryKind::Applications,
    StorageCategoryKind::System,
    StorageCategoryKind::TemporaryFiles,
    StorageCategoryKind::RecycleBin,
    StorageCategoryKind::Desktop,
    StorageCategoryKind::Documents,
    StorageCategoryKind::Downloads,
    StorageCategoryKind::Photos,
    StorageCategoryKind::Videos,
    StorageCategoryKind::Audio,
    StorageCategoryKind::Archives,
    StorageCategoryKind::Developer,
    StorageCategoryKind::OtherUsers,
    StorageCategoryKind::Other,
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategory {
    pub kind: StorageCategoryKind,
    pub logical_bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageLocation {
    pub name: String,
    pub path: String,
    pub logical_bytes: u64,
    pub file_count: u64,
    pub dominant_category: StorageCategoryKind,
    pub modified_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DriveScanPhase {
    Discovering,
    Finalizing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveScanProgress {
    pub phase: DriveScanPhase,
    pub message: String,
    pub processed_files: u64,
    pub processed_bytes: u64,
    pub unreadable_entries: u64,
    pub categories: Vec<StorageCategory>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveScanReport {
    pub root: String,
    pub completed_at_unix_ms: u128,
    pub duration_ms: u128,
    pub total_files: u64,
    pub total_logical_bytes: u64,
    pub hard_links_skipped: u64,
    pub hard_link_deduplication: bool,
    pub hard_link_identity_limit_reached: bool,
    pub location_tracking_limit_reached: bool,
    pub unreadable_entries: u64,
    pub categories: Vec<StorageCategory>,
    pub largest_locations: Vec<StorageLocation>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Default)]
struct CategoryAccumulator {
    logical_bytes: u64,
    file_count: u64,
}

#[derive(Default)]
struct LocationAccumulator {
    logical_bytes: u64,
    file_count: u64,
    category_bytes: HashMap<StorageCategoryKind, u64>,
    modified_at: Option<SystemTime>,
}

pub fn scan_drive<F, C>(
    root: impl AsRef<Path>,
    config: DriveScanConfig,
    mut on_progress: F,
    should_cancel: C,
) -> Result<DriveScanReport, ScanError>
where
    F: FnMut(DriveScanProgress),
    C: Fn() -> bool,
{
    let started = Instant::now();
    let max_locations = config.max_locations.min(MAX_DRIVE_LOCATION_RESULTS);
    let max_tracked_locations = config
        .max_tracked_locations
        .max(max_locations)
        .min(MAX_TRACKED_LOCATIONS);
    let max_issues = config.max_issues.min(MAX_DRIVE_ISSUES);
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
    let classifier = StorageClassifier::new(&root);
    let location_depth = config.location_depth.clamp(1, 8);
    let mut categories: HashMap<StorageCategoryKind, CategoryAccumulator> = HashMap::new();
    let mut locations: HashMap<PathBuf, LocationAccumulator> = HashMap::new();
    let mut seen_files = HashSet::new();
    let mut issues = Vec::new();
    let mut total_files = 0_u64;
    let mut total_logical_bytes = 0_u64;
    let mut hard_links_skipped = 0_u64;
    let mut hard_link_identity_limit_reached = false;
    let mut location_tracking_limit_reached = false;
    let mut unreadable_entries = 0_u64;

    on_progress(DriveScanProgress {
        phase: DriveScanPhase::Discovering,
        message: "드라이브의 파일과 저장공간 범주를 확인하고 있습니다".to_owned(),
        processed_files: 0,
        processed_bytes: 0,
        unreadable_entries: 0,
        categories: category_summaries(&categories),
    });

    for item in jwalk::WalkDir::new(&root)
        .follow_links(false)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(
            super::bounded_worker_threads(),
        ))
    {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }

        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                unreadable_entries = unreadable_entries.saturating_add(1);
                push_issue(&mut issues, max_issues, None, error.to_string());
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
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

        match check_hard_link(&path, &metadata, &mut seen_files) {
            HardLinkCheck::Repeated => {
                hard_links_skipped = hard_links_skipped.saturating_add(1);
                continue;
            }
            HardLinkCheck::TrackingLimit => hard_link_identity_limit_reached = true,
            HardLinkCheck::NotRepeated => {}
        }

        let logical_bytes = metadata.len();
        let category = classifier.classify(&path);
        let modified_at = metadata.modified().ok();
        let category_entry = categories.entry(category).or_default();
        category_entry.logical_bytes = category_entry.logical_bytes.saturating_add(logical_bytes);
        category_entry.file_count = category_entry.file_count.saturating_add(1);

        let bucket = location_bucket(&root, &path, location_depth);
        if let Some(location) = tracked_location(
            &mut locations,
            bucket,
            max_tracked_locations,
            &mut location_tracking_limit_reached,
        ) {
            location.logical_bytes = location.logical_bytes.saturating_add(logical_bytes);
            location.file_count = location.file_count.saturating_add(1);
            let category_bytes = location.category_bytes.entry(category).or_default();
            *category_bytes = category_bytes.saturating_add(logical_bytes);
            if modified_at > location.modified_at {
                location.modified_at = modified_at;
            }
        }

        total_files = total_files.saturating_add(1);
        total_logical_bytes = total_logical_bytes.saturating_add(logical_bytes);

        if total_files.is_multiple_of(DRIVE_PROGRESS_FILE_INTERVAL) {
            on_progress(DriveScanProgress {
                phase: DriveScanPhase::Discovering,
                message: format!("{total_files}개 파일의 용량을 분류했습니다"),
                processed_files: total_files,
                processed_bytes: total_logical_bytes,
                unreadable_entries,
                categories: category_summaries(&categories),
            });
        }
    }

    on_progress(DriveScanProgress {
        phase: DriveScanPhase::Finalizing,
        message: "큰 위치와 범주별 사용량을 정리하고 있습니다".to_owned(),
        processed_files: total_files,
        processed_bytes: total_logical_bytes,
        unreadable_entries,
        categories: category_summaries(&categories),
    });

    if location_tracking_limit_reached {
        push_issue(
            &mut issues,
            max_issues,
            Some(root.to_string_lossy().into_owned()),
            "큰 위치 집계가 메모리 안전 상한에 도달해 일부 위치는 범주 합계에만 반영했습니다"
                .to_owned(),
        );
    }
    if hard_link_identity_limit_reached {
        push_issue(
            &mut issues,
            max_issues,
            Some(root.to_string_lossy().into_owned()),
            "하드링크 식별자 집계가 메모리 안전 상한에 도달했습니다".to_owned(),
        );
    }

    let mut largest_locations: Vec<StorageLocation> = locations
        .into_iter()
        .map(|(path, accumulator)| StorageLocation {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned()),
            path: path.to_string_lossy().into_owned(),
            logical_bytes: accumulator.logical_bytes,
            file_count: accumulator.file_count,
            dominant_category: dominant_category(&accumulator.category_bytes),
            modified_at_unix_ms: system_time_ms(accumulator.modified_at),
        })
        .collect();
    largest_locations.sort_unstable_by(|left, right| {
        right
            .logical_bytes
            .cmp(&left.logical_bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
    largest_locations.truncate(max_locations);

    Ok(DriveScanReport {
        root: root.to_string_lossy().into_owned(),
        completed_at_unix_ms: system_time_ms(Some(SystemTime::now())).unwrap_or_default(),
        duration_ms: started.elapsed().as_millis(),
        total_files,
        total_logical_bytes,
        hard_links_skipped,
        hard_link_deduplication: cfg!(not(windows)) && !hard_link_identity_limit_reached,
        hard_link_identity_limit_reached,
        location_tracking_limit_reached,
        unreadable_entries,
        categories: category_summaries(&categories),
        largest_locations,
        issues,
    })
}

#[cfg_attr(windows, allow(dead_code))]
enum HardLinkCheck {
    NotRepeated,
    Repeated,
    TrackingLimit,
}

#[cfg(not(windows))]
fn check_hard_link(
    path: &Path,
    metadata: &std::fs::Metadata,
    seen_files: &mut HashSet<super::FileIdentity>,
) -> HardLinkCheck {
    let Some(identity) = super::file_identity(path, metadata) else {
        return HardLinkCheck::NotRepeated;
    };
    if seen_files.contains(&identity) {
        return HardLinkCheck::Repeated;
    }
    if seen_files.len() >= MAX_DRIVE_HARD_LINK_IDENTITIES {
        return HardLinkCheck::TrackingLimit;
    }
    seen_files.insert(identity);
    HardLinkCheck::NotRepeated
}

#[cfg(windows)]
fn check_hard_link(
    _path: &Path,
    _metadata: &std::fs::Metadata,
    _seen_files: &mut HashSet<super::FileIdentity>,
) -> HardLinkCheck {
    // Opening every file only to obtain an NTFS file ID makes a whole-drive
    // inventory unusably slow. The Windows baseline therefore reports logical
    // bytes and leaves allocation-accurate deduplication to the future MFT/USN
    // adapter. The duplicate-file scanner still performs exact file-ID checks.
    HardLinkCheck::NotRepeated
}

fn tracked_location<'a>(
    locations: &'a mut HashMap<PathBuf, LocationAccumulator>,
    path: PathBuf,
    limit: usize,
    limit_reached: &mut bool,
) -> Option<&'a mut LocationAccumulator> {
    let can_insert = locations.len() < limit;
    match locations.entry(path) {
        Entry::Occupied(entry) => Some(entry.into_mut()),
        Entry::Vacant(entry) if can_insert => Some(entry.insert(LocationAccumulator::default())),
        Entry::Vacant(_) => {
            *limit_reached = true;
            None
        }
    }
}

fn category_summaries(
    categories: &HashMap<StorageCategoryKind, CategoryAccumulator>,
) -> Vec<StorageCategory> {
    STORAGE_CATEGORY_ORDER
        .into_iter()
        .map(|kind| {
            let accumulator = categories.get(&kind);
            StorageCategory {
                kind,
                logical_bytes: accumulator.map_or(0, |value| value.logical_bytes),
                file_count: accumulator.map_or(0, |value| value.file_count),
            }
        })
        .collect()
}

fn dominant_category(category_bytes: &HashMap<StorageCategoryKind, u64>) -> StorageCategoryKind {
    let mut dominant = StorageCategoryKind::Other;
    let mut largest_bytes = 0_u64;
    for kind in STORAGE_CATEGORY_ORDER {
        let bytes = category_bytes.get(&kind).copied().unwrap_or_default();
        if bytes > largest_bytes {
            dominant = kind;
            largest_bytes = bytes;
        }
    }
    dominant
}

fn location_bucket(root: &Path, path: &Path, depth: usize) -> PathBuf {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let mut bucket = root.to_path_buf();
    let mut added = false;

    for component in parent.components().take(depth) {
        bucket.push(component.as_os_str());
        added = true;
    }

    if added { bucket } else { root.to_path_buf() }
}

struct KnownCategoryPath {
    normalized: String,
    kind: StorageCategoryKind,
}

struct StorageClassifier {
    root: String,
    home: Option<String>,
    known_paths: Vec<KnownCategoryPath>,
}

impl StorageClassifier {
    fn new(root: &Path) -> Self {
        let mut known_paths = Vec::new();
        add_known_path(
            &mut known_paths,
            Some(std::env::temp_dir()),
            StorageCategoryKind::TemporaryFiles,
        );
        #[cfg(not(windows))]
        add_known_path(
            &mut known_paths,
            dirs::cache_dir(),
            StorageCategoryKind::TemporaryFiles,
        );
        add_known_path(
            &mut known_paths,
            dirs::desktop_dir(),
            StorageCategoryKind::Desktop,
        );
        add_known_path(
            &mut known_paths,
            dirs::document_dir(),
            StorageCategoryKind::Documents,
        );
        add_known_path(
            &mut known_paths,
            dirs::download_dir(),
            StorageCategoryKind::Downloads,
        );
        add_known_path(
            &mut known_paths,
            dirs::picture_dir(),
            StorageCategoryKind::Photos,
        );
        add_known_path(
            &mut known_paths,
            dirs::video_dir(),
            StorageCategoryKind::Videos,
        );
        add_known_path(
            &mut known_paths,
            dirs::audio_dir(),
            StorageCategoryKind::Audio,
        );
        add_known_path(
            &mut known_paths,
            dirs::data_local_dir(),
            StorageCategoryKind::Applications,
        );

        known_paths.sort_unstable_by_key(|path| Reverse(path.normalized.len()));
        known_paths.dedup_by(|left, right| left.normalized == right.normalized);

        Self {
            root: normalize_path(root),
            home: dirs::home_dir().map(|path| normalize_path(&path)),
            known_paths,
        }
    }

    fn classify(&self, path: &Path) -> StorageCategoryKind {
        let normalized = normalize_path(path);

        for known in &self.known_paths {
            if is_within(&normalized, &known.normalized) {
                return known.kind;
            }
        }

        if let Some(category) = platform_category(&normalized, &self.root, self.home.as_deref()) {
            return category;
        }

        if is_developer_path(&normalized) {
            return StorageCategoryKind::Developer;
        }

        category_from_extension(path).unwrap_or(StorageCategoryKind::Other)
    }
}

fn add_known_path(
    known_paths: &mut Vec<KnownCategoryPath>,
    path: Option<PathBuf>,
    kind: StorageCategoryKind,
) {
    if let Some(path) = path {
        known_paths.push(KnownCategoryPath {
            normalized: normalize_path(&path),
            kind,
        });
    }
}

fn normalize_path(path: &Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");

    #[cfg(windows)]
    {
        if let Some(rest) = normalized.strip_prefix("//?/UNC/") {
            normalized = format!("//{rest}");
        } else if let Some(rest) = normalized.strip_prefix("//?/") {
            normalized = rest.to_owned();
        }
        normalized = normalized.to_lowercase();
    }

    if normalized.len() > 1 {
        normalized = normalized.trim_end_matches('/').to_owned();
    }
    normalized
}

fn is_within(path: &str, base: &str) -> bool {
    if base == "/" {
        return path.starts_with('/');
    }
    path == base
        || path
            .strip_prefix(base)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

fn relative_path<'a>(path: &'a str, root: &str) -> Option<&'a str> {
    if root == "/" {
        return Some(path.trim_start_matches('/'));
    }
    path.strip_prefix(root)
        .map(|relative| relative.trim_start_matches('/'))
}

#[cfg(windows)]
fn platform_category(path: &str, root: &str, home: Option<&str>) -> Option<StorageCategoryKind> {
    let relative = relative_path(path, root)?;
    let components: Vec<&str> = relative
        .split('/')
        .filter(|value| !value.is_empty())
        .collect();
    let first = components.first().copied().unwrap_or_default();
    let file_name = components.last().copied().unwrap_or_default();

    if matches!(
        file_name,
        "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "memory.dmp"
    ) {
        return Some(StorageCategoryKind::System);
    }
    if first == "$recycle.bin" {
        return Some(StorageCategoryKind::RecycleBin);
    }
    if matches!(
        first,
        "windows"
            | "system volume information"
            | "recovery"
            | "perflogs"
            | "$winreagent"
            | "$windows.~bt"
            | "$windows.~ws"
    ) {
        return Some(StorageCategoryKind::System);
    }
    if matches!(
        first,
        "program files" | "program files (x86)" | "programdata"
    ) {
        return Some(StorageCategoryKind::Applications);
    }
    if first == "users" && home.is_none_or(|home| !is_within(path, home)) {
        return Some(StorageCategoryKind::OtherUsers);
    }

    None
}

#[cfg(target_os = "macos")]
fn platform_category(path: &str, root: &str, home: Option<&str>) -> Option<StorageCategoryKind> {
    let relative = relative_path(path, root)?;
    let components: Vec<&str> = relative
        .split('/')
        .filter(|value| !value.is_empty())
        .collect();
    let first = components.first().copied().unwrap_or_default();

    if components
        .iter()
        .any(|component| matches!(*component, ".Trash" | ".Trashes"))
    {
        return Some(StorageCategoryKind::RecycleBin);
    }
    if first == "Applications" {
        return Some(StorageCategoryKind::Applications);
    }
    if matches!(
        first,
        "System" | "Library" | "private" | "usr" | "bin" | "sbin" | "dev" | "cores"
    ) {
        return Some(StorageCategoryKind::System);
    }
    if first == "Users" && home.is_none_or(|home| !is_within(path, home)) {
        return Some(StorageCategoryKind::OtherUsers);
    }

    None
}

#[cfg(not(any(windows, target_os = "macos")))]
fn platform_category(_path: &str, _root: &str, _home: Option<&str>) -> Option<StorageCategoryKind> {
    None
}

fn is_developer_path(path: &str) -> bool {
    path.split('/').any(|component| {
        let component = component.to_ascii_lowercase();
        matches!(
            component.as_str(),
            "node_modules"
                | "target"
                | ".gradle"
                | ".m2"
                | "deriveddata"
                | ".next"
                | ".nuxt"
                | ".turbo"
                | ".git"
                | ".svn"
                | "coverage"
                | "pods"
        )
    })
}

fn category_from_extension(path: &Path) -> Option<StorageCategoryKind> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let extension = extension.as_str();

    if matches!(
        extension,
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "heic"
            | "heif"
            | "tif"
            | "tiff"
            | "bmp"
            | "raw"
            | "dng"
            | "svg"
    ) {
        return Some(StorageCategoryKind::Photos);
    }
    if matches!(
        extension,
        "mp4" | "mkv" | "mov" | "avi" | "wmv" | "webm" | "m4v" | "flv" | "mts" | "m2ts"
    ) {
        return Some(StorageCategoryKind::Videos);
    }
    if matches!(
        extension,
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "opus" | "wma" | "aiff"
    ) {
        return Some(StorageCategoryKind::Audio);
    }
    if matches!(
        extension,
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" | "zst" | "iso"
    ) {
        return Some(StorageCategoryKind::Archives);
    }
    if matches!(
        extension,
        "rs" | "swift"
            | "kt"
            | "java"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "cs"
            | "go"
            | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "vue"
            | "svelte"
            | "rb"
            | "php"
            | "sh"
            | "ps1"
            | "sql"
    ) {
        return Some(StorageCategoryKind::Developer);
    }
    if matches!(
        extension,
        "pdf"
            | "txt"
            | "md"
            | "rtf"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "odt"
            | "ods"
            | "odp"
            | "hwp"
            | "hwpx"
            | "pages"
            | "numbers"
            | "key"
    ) {
        return Some(StorageCategoryKind::Documents);
    }
    if matches!(extension, "exe" | "msi" | "msix" | "appx" | "dmg" | "pkg") {
        return Some(StorageCategoryKind::Applications);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_drive_without_hashing_and_keeps_category_totals_consistent() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let nested = temp.path().join("alpha").join("beta");
        fs::create_dir_all(&nested).expect("create nested directory");
        fs::write(nested.join("note.txt"), vec![b'a'; 31]).expect("write document");
        fs::write(nested.join("image.png"), vec![b'b'; 17]).expect("write image");

        let mut progress_events = Vec::new();
        let report = scan_drive(
            temp.path(),
            DriveScanConfig {
                max_locations: 10,
                max_tracked_locations: 100,
                max_issues: 10,
                location_depth: 2,
            },
            |progress| progress_events.push(progress),
            || false,
        )
        .expect("scan drive");

        assert_eq!(report.total_files, 2);
        assert_eq!(report.total_logical_bytes, 48);
        assert_eq!(
            report
                .categories
                .iter()
                .map(|category| category.logical_bytes)
                .sum::<u64>(),
            report.total_logical_bytes
        );
        assert_eq!(report.largest_locations.len(), 1);
        assert!(progress_events.len() >= 2);
    }

    #[test]
    fn bounds_location_tracking_without_losing_drive_totals() {
        let temp = tempfile::tempdir().expect("create temp directory");
        for index in 0..50_u64 {
            let directory = temp.path().join(format!("location-{index:03}"));
            fs::create_dir(&directory).expect("create location");
            fs::write(directory.join("file.bin"), vec![0_u8; 10]).expect("write file");
        }

        let report = scan_drive(
            temp.path(),
            DriveScanConfig {
                max_locations: 2,
                max_tracked_locations: 4,
                max_issues: 10,
                location_depth: 1,
            },
            |_| {},
            || false,
        )
        .expect("scan drive");

        assert!(report.location_tracking_limit_reached);
        assert_eq!(report.largest_locations.len(), 2);
        assert_eq!(report.total_files, 50);
        assert_eq!(report.total_logical_bytes, 500);
        assert_eq!(
            report
                .categories
                .iter()
                .map(|category| category.logical_bytes)
                .sum::<u64>(),
            500
        );
    }

    #[test]
    fn includes_dot_prefixed_storage_entries() {
        let temp = tempfile::tempdir().expect("create temp directory");
        fs::write(temp.path().join(".hidden.bin"), vec![0_u8; 13]).expect("write hidden file");

        let report = scan_drive(temp.path(), DriveScanConfig::default(), |_| {}, || false)
            .expect("scan drive");

        assert_eq!(report.total_files, 1);
        assert_eq!(report.total_logical_bytes, 13);
    }

    #[test]
    fn classifies_common_content_extensions() {
        assert_eq!(
            category_from_extension(Path::new("contract.hwpx")),
            Some(StorageCategoryKind::Documents)
        );
        assert_eq!(
            category_from_extension(Path::new("recording.mkv")),
            Some(StorageCategoryKind::Videos)
        );
        assert_eq!(
            category_from_extension(Path::new("archive.7z")),
            Some(StorageCategoryKind::Archives)
        );
    }

    #[test]
    fn honours_drive_scan_cancellation() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let result = scan_drive(temp.path(), DriveScanConfig::default(), |_| {}, || true);
        assert!(matches!(result, Err(ScanError::Cancelled)));
    }
}
