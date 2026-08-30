use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod actions;
mod cleanup;
mod directory;
mod document_search;
mod drive;
mod file_catalog;

pub use actions::{
    ActionValidationError, VerifiedTrashItem, revalidate_verified_trash_item,
    validate_cleanup_trash_candidate, validate_duplicate_trash_selection,
};

pub use cleanup::{
    CleanupCandidate, CleanupCandidateKind, CleanupConfidence, CleanupRootSpec, CleanupScanConfig,
    CleanupScanProgress, CleanupScanReport, scan_cleanup_candidates,
};

pub use directory::{
    DirectoryNode, DirectoryScanConfig, DirectoryScanProgress, DirectoryScanReport, EmptyDirectory,
    scan_directory_level,
};
pub use document_search::{
    DocumentFormat, DocumentIndexConfig, DocumentIndexIssue, DocumentIndexPhase,
    DocumentIndexProgress, DocumentIndexReport, DocumentIndexStatus, DocumentMatchSource,
    DocumentSearchError, DocumentSearchReport, DocumentSearchRequest, DocumentSearchResult,
    DocumentSnippetPart, build_document_index, document_index_status, search_document_index,
};
pub use drive::{
    DriveScanConfig, DriveScanPhase, DriveScanProgress, DriveScanReport, StorageCategory,
    StorageCategoryKind, StorageLocation, scan_drive,
};
pub use file_catalog::{
    FileCatalogConfig, FileCatalogEntryKind, FileCatalogError, FileCatalogIssue,
    FileCatalogMatchSource, FileCatalogPhase, FileCatalogProgress, FileCatalogProvider,
    FileCatalogRefreshMode, FileCatalogReport, FileCatalogSearchReport, FileCatalogSearchRequest,
    FileCatalogSearchResult, FileCatalogSort, FileCatalogStatus, build_file_catalog,
    clear_file_catalog, file_catalog_status, search_file_catalog,
};

const HASH_CHUNK_BYTES: usize = 64 * 1024;
const COMPARE_CHUNK_BYTES: usize = 256 * 1024;
const PROGRESS_FILE_INTERVAL: u64 = 512;
const SAMPLE_BATCH_SIZE: usize = 512;
const FULL_HASH_BATCH_SIZE: usize = 16;
const MAX_WALK_WORKERS: usize = 4;
const MAX_HASH_WORKERS: usize = 2;
const MAX_LARGE_FILE_RESULTS: usize = 10_000;
const MAX_DUPLICATE_GROUP_RESULTS: usize = 10_000;
const MAX_DUPLICATE_CANDIDATES: usize = 250_000;
const MAX_ISSUES: usize = 1_000;
const MAX_TRACKED_HARD_LINK_IDENTITIES: usize = 250_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScanConfig {
    pub min_large_file_bytes: u64,
    pub min_duplicate_file_bytes: u64,
    pub max_large_files: usize,
    pub max_duplicate_groups: usize,
    pub max_duplicate_candidates: usize,
    pub max_issues: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            min_large_file_bytes: 100 * 1024 * 1024,
            min_duplicate_file_bytes: 1024 * 1024,
            max_large_files: 250,
            max_duplicate_groups: 100,
            max_duplicate_candidates: 250_000,
            max_issues: 50,
        }
    }
}

impl ScanConfig {
    fn bounded(mut self) -> Self {
        self.max_large_files = self.max_large_files.min(MAX_LARGE_FILE_RESULTS);
        self.max_duplicate_groups = self.max_duplicate_groups.min(MAX_DUPLICATE_GROUP_RESULTS);
        self.max_duplicate_candidates = self.max_duplicate_candidates.min(MAX_DUPLICATE_CANDIDATES);
        self.max_issues = self.max_issues.min(MAX_ISSUES);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub message: String,
    pub processed_files: u64,
    pub processed_bytes: u64,
    pub fraction: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanPhase {
    Discovering,
    Sampling,
    Verifying,
    Finalizing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub root: String,
    pub completed_at_unix_ms: u128,
    pub duration_ms: u128,
    pub total_files: u64,
    pub total_logical_bytes: u64,
    pub hard_links_skipped: u64,
    pub hard_link_identity_limit_reached: bool,
    pub unreadable_entries: u64,
    pub candidate_limit_reached: bool,
    pub large_files: Vec<FileEntry>,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub duplicate_waste_bytes: u64,
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub logical_bytes: u64,
    pub modified_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub content_hash: String,
    pub each_file_bytes: u64,
    pub wasted_bytes: u64,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanIssue {
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("scan path does not exist: {0}")]
    MissingPath(String),
    #[error("scan path is not a directory: {0}")]
    NotDirectory(String),
    #[error("scan was cancelled")]
    Cancelled,
    #[error("failed to access scan path: {0}")]
    Access(String),
}

#[derive(Debug, Clone)]
struct Candidate {
    entry: FileEntry,
    path: PathBuf,
    modified_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileIdentity {
    device: u64,
    index: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileObjectIdentity {
    identity: FileIdentity,
    links: u64,
}

pub fn scan_path<F, C>(
    root: impl AsRef<Path>,
    config: ScanConfig,
    mut on_progress: F,
    should_cancel: C,
) -> Result<ScanReport, ScanError>
where
    F: FnMut(ScanProgress),
    C: Fn() -> bool + Sync,
{
    let started = Instant::now();
    let config = config.bounded();
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

    on_progress(ScanProgress {
        phase: ScanPhase::Discovering,
        message: "파일과 폴더를 확인하고 있습니다".to_owned(),
        processed_files: 0,
        processed_bytes: 0,
        fraction: None,
    });

    let mut total_files = 0_u64;
    let mut total_logical_bytes = 0_u64;
    let mut hard_links_skipped = 0_u64;
    let mut hard_link_identity_limit_reached = false;
    let mut unreadable_entries = 0_u64;
    let mut issues = Vec::new();
    let mut seen_files = HashSet::new();
    let mut large_files = Vec::new();
    let mut size_groups: HashMap<u64, Vec<Candidate>> = HashMap::new();
    let mut duplicate_candidates = 0_usize;
    let mut candidate_limit_reached = false;

    for item in jwalk::WalkDir::new(&root)
        .follow_links(false)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(bounded_worker_threads()))
    {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }

        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                unreadable_entries += 1;
                push_issue(&mut issues, config.max_issues, None, error.to_string());
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
                unreadable_entries += 1;
                push_issue(
                    &mut issues,
                    config.max_issues,
                    Some(path.to_string_lossy().into_owned()),
                    error.to_string(),
                );
                continue;
            }
        };

        let mut duplicate_safe = true;
        if let Some(identity) = file_identity(&path, &metadata) {
            if seen_files.contains(&identity) {
                hard_links_skipped += 1;
                continue;
            }
            if seen_files.len() < MAX_TRACKED_HARD_LINK_IDENTITIES {
                seen_files.insert(identity);
            } else {
                hard_link_identity_limit_reached = true;
                duplicate_safe = false;
            }
        }

        let logical_bytes = metadata.len();
        total_files += 1;
        total_logical_bytes = total_logical_bytes.saturating_add(logical_bytes);

        let file_entry = FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            logical_bytes,
            modified_at_unix_ms: system_time_ms(metadata.modified().ok()),
        };

        if logical_bytes >= config.min_large_file_bytes {
            push_bounded_large_file(&mut large_files, file_entry.clone(), config.max_large_files);
        }

        if duplicate_safe && logical_bytes >= config.min_duplicate_file_bytes {
            if duplicate_candidates < config.max_duplicate_candidates {
                size_groups
                    .entry(logical_bytes)
                    .or_default()
                    .push(Candidate {
                        entry: file_entry,
                        path,
                        modified_at: metadata.modified().ok(),
                    });
                duplicate_candidates += 1;
            } else {
                candidate_limit_reached = true;
            }
        }

        if total_files.is_multiple_of(PROGRESS_FILE_INTERVAL) {
            on_progress(ScanProgress {
                phase: ScanPhase::Discovering,
                message: format!("{total_files}개 파일을 확인했습니다"),
                processed_files: total_files,
                processed_bytes: total_logical_bytes,
                fraction: None,
            });
        }
    }

    if hard_link_identity_limit_reached {
        push_issue(
            &mut issues,
            config.max_issues,
            None,
            "하드링크 파일이 너무 많아 이후 항목을 중복 분석에서 제외했습니다".to_owned(),
        );
    }

    sort_large_files(&mut large_files);
    large_files.truncate(config.max_large_files);

    let candidates: Vec<Candidate> = size_groups
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .collect();

    on_progress(ScanProgress {
        phase: ScanPhase::Sampling,
        message: format!(
            "{}개 중복 후보를 빠르게 비교하고 있습니다",
            candidates.len()
        ),
        processed_files: total_files,
        processed_bytes: total_logical_bytes,
        fraction: Some(0.58),
    });

    let hash_pool = if candidates.is_empty() {
        None
    } else {
        Some(
            rayon::ThreadPoolBuilder::new()
                .num_threads(bounded_hash_worker_threads())
                .thread_name(|index| format!("bloomsweepy-hash-{index}"))
                .build()
                .map_err(|error| ScanError::Access(error.to_string()))?,
        )
    };
    let mut sample_groups: HashMap<(u64, [u8; 32]), Vec<Candidate>> = HashMap::new();
    let sample_total = candidates.len();
    let mut sampled_count = 0_usize;
    let mut candidate_iter = candidates.into_iter();
    loop {
        let batch: Vec<Candidate> = candidate_iter.by_ref().take(SAMPLE_BATCH_SIZE).collect();
        if batch.is_empty() {
            break;
        }
        let sampled: Vec<(Candidate, io::Result<[u8; 32]>)> = hash_pool
            .as_ref()
            .expect("candidate batches require a hash worker pool")
            .install(|| {
                batch
                    .into_par_iter()
                    .map(|candidate| {
                        let result = quick_signature_candidate(&candidate, &should_cancel);
                        (candidate, result)
                    })
                    .collect()
            });
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        sampled_count = sampled_count.saturating_add(sampled.len());
        for (candidate, result) in sampled {
            match result {
                Ok(signature) => sample_groups
                    .entry((candidate.entry.logical_bytes, signature))
                    .or_default()
                    .push(candidate),
                Err(error) => {
                    unreadable_entries += 1;
                    push_issue(
                        &mut issues,
                        config.max_issues,
                        Some(candidate.entry.path.clone()),
                        error.to_string(),
                    );
                }
            }
        }
        on_progress(ScanProgress {
            phase: ScanPhase::Sampling,
            message: format!("{sampled_count}/{sample_total}개 중복 후보를 빠르게 비교했습니다"),
            processed_files: total_files,
            processed_bytes: total_logical_bytes,
            fraction: phase_fraction(0.58, 0.74, sampled_count, sample_total),
        });
    }

    let full_hash_candidates: Vec<Candidate> = sample_groups
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .collect();

    on_progress(ScanProgress {
        phase: ScanPhase::Verifying,
        message: format!(
            "{}개 후보의 전체 내용을 검증하고 있습니다",
            full_hash_candidates.len()
        ),
        processed_files: total_files,
        processed_bytes: total_logical_bytes,
        fraction: Some(0.76),
    });

    let mut full_hash_groups: HashMap<(u64, [u8; 32]), Vec<Candidate>> = HashMap::new();
    let full_hash_total = full_hash_candidates.len();
    let mut hashed_count = 0_usize;
    let mut full_hash_iter = full_hash_candidates.into_iter();
    loop {
        let batch: Vec<Candidate> = full_hash_iter.by_ref().take(FULL_HASH_BATCH_SIZE).collect();
        if batch.is_empty() {
            break;
        }
        let hashed: Vec<(Candidate, io::Result<[u8; 32]>)> = hash_pool
            .as_ref()
            .expect("full-hash batches require a hash worker pool")
            .install(|| {
                batch
                    .into_par_iter()
                    .map(|candidate| {
                        let result = full_hash_candidate(&candidate, &should_cancel);
                        (candidate, result)
                    })
                    .collect()
            });
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        hashed_count = hashed_count.saturating_add(hashed.len());
        for (candidate, result) in hashed {
            match result {
                Ok(hash) => full_hash_groups
                    .entry((candidate.entry.logical_bytes, hash))
                    .or_default()
                    .push(candidate),
                Err(error) => {
                    unreadable_entries += 1;
                    push_issue(
                        &mut issues,
                        config.max_issues,
                        Some(candidate.entry.path.clone()),
                        error.to_string(),
                    );
                }
            }
        }
        on_progress(ScanProgress {
            phase: ScanPhase::Verifying,
            message: format!("{hashed_count}/{full_hash_total}개 후보의 전체 내용을 검증했습니다"),
            processed_files: total_files,
            processed_bytes: total_logical_bytes,
            fraction: phase_fraction(0.76, 0.93, hashed_count, full_hash_total),
        });
    }
    drop(hash_pool);

    on_progress(ScanProgress {
        phase: ScanPhase::Finalizing,
        message: "중복 파일을 최종 확인하고 있습니다".to_owned(),
        processed_files: total_files,
        processed_bytes: total_logical_bytes,
        fraction: Some(0.94),
    });

    let mut duplicate_groups = Vec::new();
    for ((each_file_bytes, hash), group) in full_hash_groups {
        if group.len() < 2 {
            continue;
        }

        for exact_group in
            partition_exact_matches(group, &should_cancel, &mut issues, config.max_issues)?
        {
            if exact_group.len() < 2 {
                continue;
            }

            let mut files: Vec<FileEntry> = exact_group
                .into_iter()
                .map(|candidate| candidate.entry)
                .collect();
            files.sort_unstable_by(|left, right| left.path.cmp(&right.path));
            let wasted_bytes = each_file_bytes.saturating_mul((files.len() - 1) as u64);

            duplicate_groups.push(DuplicateGroup {
                content_hash: blake3::Hash::from_bytes(hash).to_hex().to_string(),
                each_file_bytes,
                wasted_bytes,
                files,
            });
        }
    }

    duplicate_groups.sort_unstable_by(|left, right| {
        right
            .wasted_bytes
            .cmp(&left.wasted_bytes)
            .then_with(|| left.content_hash.cmp(&right.content_hash))
    });
    duplicate_groups.truncate(config.max_duplicate_groups);
    let duplicate_waste_bytes = duplicate_groups.iter().fold(0_u64, |total, group| {
        total.saturating_add(group.wasted_bytes)
    });

    Ok(ScanReport {
        root: root.to_string_lossy().into_owned(),
        completed_at_unix_ms: system_time_ms(Some(SystemTime::now())).unwrap_or_default(),
        duration_ms: started.elapsed().as_millis(),
        total_files,
        total_logical_bytes,
        hard_links_skipped,
        hard_link_identity_limit_reached,
        unreadable_entries,
        candidate_limit_reached,
        large_files,
        duplicate_groups,
        duplicate_waste_bytes,
        issues,
    })
}

fn partition_exact_matches<C>(
    candidates: Vec<Candidate>,
    should_cancel: &C,
    issues: &mut Vec<ScanIssue>,
    max_issues: usize,
) -> Result<Vec<Vec<Candidate>>, ScanError>
where
    C: Fn() -> bool + Sync,
{
    let mut partitions: Vec<Vec<Candidate>> = Vec::new();

    'candidate: for candidate in candidates {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }

        for partition in &mut partitions {
            match files_equal(&partition[0], &candidate, should_cancel) {
                Ok(true) => {
                    partition.push(candidate);
                    continue 'candidate;
                }
                Ok(false) => {}
                Err(error) => {
                    if error.kind() == io::ErrorKind::Interrupted {
                        return Err(ScanError::Cancelled);
                    }
                    push_issue(
                        issues,
                        max_issues,
                        Some(candidate.entry.path.clone()),
                        error.to_string(),
                    );
                    continue 'candidate;
                }
            }
        }

        partitions.push(vec![candidate]);
    }

    Ok(partitions)
}

fn quick_signature_candidate<C>(candidate: &Candidate, should_cancel: &C) -> io::Result<[u8; 32]>
where
    C: Fn() -> bool,
{
    validate_candidate_snapshot(candidate)?;
    let signature = quick_signature(
        &candidate.path,
        candidate.entry.logical_bytes,
        should_cancel,
    )?;
    validate_candidate_snapshot(candidate)?;
    Ok(signature)
}

fn quick_signature<C>(path: &Path, size: u64, should_cancel: &C) -> io::Result<[u8; 32]>
where
    C: Fn() -> bool,
{
    check_cancelled(should_cancel)?;
    let mut file = open_read_shared(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&size.to_le_bytes());

    let mut head = vec![0_u8; size.min(HASH_CHUNK_BYTES as u64) as usize];
    file.read_exact(&mut head)?;
    hasher.update(&head);

    if size > (HASH_CHUNK_BYTES * 2) as u64 {
        check_cancelled(should_cancel)?;
        file.seek(SeekFrom::End(-(HASH_CHUNK_BYTES as i64)))?;
        let mut tail = vec![0_u8; HASH_CHUNK_BYTES];
        file.read_exact(&mut tail)?;
        hasher.update(&tail);
    }

    Ok(*hasher.finalize().as_bytes())
}

fn full_hash_candidate<C>(candidate: &Candidate, should_cancel: &C) -> io::Result<[u8; 32]>
where
    C: Fn() -> bool,
{
    validate_candidate_snapshot(candidate)?;
    let hash = full_hash(&candidate.path, should_cancel)?;
    validate_candidate_snapshot(candidate)?;
    Ok(hash)
}

fn full_hash<C>(path: &Path, should_cancel: &C) -> io::Result<[u8; 32]>
where
    C: Fn() -> bool,
{
    let mut file = open_read_shared(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; COMPARE_CHUNK_BYTES];

    loop {
        check_cancelled(should_cancel)?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(*hasher.finalize().as_bytes())
}

fn files_equal<C>(left: &Candidate, right: &Candidate, should_cancel: &C) -> io::Result<bool>
where
    C: Fn() -> bool,
{
    validate_candidate_snapshot(left)?;
    validate_candidate_snapshot(right)?;
    let mut left_file = open_read_shared(&left.path)?;
    let mut right_file = open_read_shared(&right.path)?;
    let mut left_buffer = vec![0_u8; COMPARE_CHUNK_BYTES];
    let mut right_buffer = vec![0_u8; COMPARE_CHUNK_BYTES];

    loop {
        check_cancelled(should_cancel)?;
        let left_read = left_file.read(&mut left_buffer)?;
        let right_read = right_file.read(&mut right_buffer)?;

        if left_read != right_read {
            return Ok(false);
        }
        if left_read == 0 {
            validate_candidate_snapshot(left)?;
            validate_candidate_snapshot(right)?;
            return Ok(true);
        }
        if left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
    }
}

fn validate_candidate_snapshot(candidate: &Candidate) -> io::Result<()> {
    let metadata = fs::symlink_metadata(&candidate.path)?;
    let unchanged = metadata.file_type().is_file()
        && metadata.len() == candidate.entry.logical_bytes
        && metadata.modified().ok() == candidate.modified_at;
    if unchanged {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "스캔 도중 파일이 변경되어 중복 분석에서 제외했습니다",
        ))
    }
}

fn check_cancelled<C>(should_cancel: &C) -> io::Result<()>
where
    C: Fn() -> bool,
{
    if should_cancel() {
        Err(io::Error::new(io::ErrorKind::Interrupted, "scan cancelled"))
    } else {
        Ok(())
    }
}

fn open_read_shared(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
    }
    options.open(path)
}

fn bounded_worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, MAX_WALK_WORKERS)
}

fn bounded_hash_worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, MAX_HASH_WORKERS)
}

fn phase_fraction(start: f64, end: f64, completed: usize, total: usize) -> Option<f64> {
    if total == 0 {
        return Some(end);
    }
    let ratio = completed.min(total) as f64 / total as f64;
    Some(start + (end - start) * ratio)
}

fn sort_large_files(files: &mut [FileEntry]) {
    files.sort_unstable_by(|left, right| {
        right
            .logical_bytes
            .cmp(&left.logical_bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn push_bounded_large_file(files: &mut Vec<FileEntry>, entry: FileEntry, limit: usize) {
    if limit == 0 {
        return;
    }
    files.push(entry);
    if files.len() > limit.saturating_mul(2) {
        sort_large_files(files);
        files.truncate(limit);
    }
}

fn push_issue(
    issues: &mut Vec<ScanIssue>,
    max_issues: usize,
    path: Option<String>,
    message: String,
) {
    if issues.len() < max_issues {
        issues.push(ScanIssue { path, message });
    }
}

fn system_time_ms(value: Option<SystemTime>) -> Option<u128> {
    value.and_then(|time| {
        time.duration_since(UNIX_EPOCH)
            .ok()
            .map(|value| value.as_millis())
    })
}

#[cfg(unix)]
fn file_object_identity(_path: &Path, metadata: &Metadata) -> Option<FileObjectIdentity> {
    use std::os::unix::fs::MetadataExt;

    Some(FileObjectIdentity {
        identity: FileIdentity {
            device: metadata.dev(),
            index: metadata.ino(),
        },
        links: metadata.nlink(),
    })
}

#[cfg(windows)]
fn file_object_identity(path: &Path, _metadata: &Metadata) -> Option<FileObjectIdentity> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        GetFileInformationByHandle,
    };

    let mut options = OpenOptions::new();
    options
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let handle = options.open(path).ok()?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    let success =
        unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, &mut information) };
    if success == 0 {
        return None;
    }

    Some(FileObjectIdentity {
        identity: FileIdentity {
            device: information.dwVolumeSerialNumber as u64,
            index: ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
        },
        links: information.nNumberOfLinks as u64,
    })
}

#[cfg(not(any(unix, windows)))]
fn file_object_identity(_path: &Path, _metadata: &Metadata) -> Option<FileObjectIdentity> {
    None
}

fn file_identity(path: &Path, metadata: &Metadata) -> Option<FileIdentity> {
    file_object_identity(path, metadata)
        .filter(|identity| identity.links > 1)
        .map(|identity| identity.identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn write_file(path: &Path, bytes: &[u8]) {
        let mut file = File::create(path).expect("create test file");
        file.write_all(bytes).expect("write test file");
    }

    fn test_config() -> ScanConfig {
        ScanConfig {
            min_large_file_bytes: 1,
            min_duplicate_file_bytes: 1,
            max_large_files: 100,
            max_duplicate_groups: 100,
            max_duplicate_candidates: 1_000,
            max_issues: 20,
        }
    }

    #[test]
    fn verifies_full_content_before_grouping_duplicates() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let original = vec![b'a'; HASH_CHUNK_BYTES * 3];
        let mut different_middle = original.clone();
        different_middle[HASH_CHUNK_BYTES + 7] = b'b';
        write_file(&temp.path().join("same-a.bin"), &original);
        write_file(&temp.path().join("same-b.bin"), &original);
        write_file(&temp.path().join("different.bin"), &different_middle);

        let report = scan_path(temp.path(), test_config(), |_| {}, || false).expect("scan");

        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(report.duplicate_groups[0].files.len(), 2);
        assert_eq!(
            report.duplicate_groups[0].wasted_bytes,
            (HASH_CHUNK_BYTES * 3) as u64
        );
    }

    #[test]
    fn does_not_count_hard_links_as_reclaimable_duplicates() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let original = temp.path().join("original.bin");
        let linked = temp.path().join("linked.bin");
        write_file(&original, b"shared-allocation");
        fs::hard_link(&original, &linked).expect("create hard link");

        let report = scan_path(temp.path(), test_config(), |_| {}, || false).expect("scan");

        assert!(report.duplicate_groups.is_empty());
        assert_eq!(report.hard_links_skipped, 1);
    }

    #[test]
    fn honours_cancellation_before_work_starts() {
        let temp = tempfile::tempdir().expect("create temp directory");
        write_file(&temp.path().join("file.bin"), b"content");

        let result = scan_path(temp.path(), test_config(), |_| {}, || true);

        assert!(matches!(result, Err(ScanError::Cancelled)));
    }

    #[test]
    fn bounds_large_file_working_set_before_final_sort() {
        let mut files = Vec::new();
        for index in 0..10_000_u64 {
            push_bounded_large_file(
                &mut files,
                FileEntry {
                    name: format!("{index}.bin"),
                    path: format!("/{index}.bin"),
                    logical_bytes: index,
                    modified_at_unix_ms: None,
                },
                32,
            );
            assert!(files.len() <= 65);
        }

        sort_large_files(&mut files);
        files.truncate(32);
        assert_eq!(files.len(), 32);
        assert_eq!(files[0].logical_bytes, 9_999);
        assert_eq!(files[31].logical_bytes, 9_968);
    }

    #[test]
    fn full_hash_checks_cancellation_between_chunks() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("large.bin");
        write_file(&path, &vec![b'x'; COMPARE_CHUNK_BYTES * 3]);
        let checks = AtomicUsize::new(0);

        let result = full_hash(&path, &|| checks.fetch_add(1, Ordering::SeqCst) >= 1);

        assert_eq!(
            result.expect_err("hash should be cancelled").kind(),
            io::ErrorKind::Interrupted
        );
        assert!(checks.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn rejects_a_file_that_changes_after_discovery() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let path = temp.path().join("changing.bin");
        write_file(&path, b"before");
        let metadata = fs::metadata(&path).expect("read metadata");
        let candidate = Candidate {
            entry: FileEntry {
                name: "changing.bin".to_owned(),
                path: path.to_string_lossy().into_owned(),
                logical_bytes: metadata.len(),
                modified_at_unix_ms: system_time_ms(metadata.modified().ok()),
            },
            path: path.clone(),
            modified_at: metadata.modified().ok(),
        };
        write_file(&path, b"after-and-longer");

        let error = validate_candidate_snapshot(&candidate).expect_err("file changed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[cfg(windows)]
    #[test]
    fn shared_read_handle_does_not_block_rename() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let original = temp.path().join("original.bin");
        let renamed = temp.path().join("renamed.bin");
        write_file(&original, b"content");
        let handle = open_read_shared(&original).expect("open shared scanner handle");

        fs::rename(&original, &renamed).expect("rename while scanner handle is open");

        drop(handle);
        assert!(renamed.exists());
    }

    #[cfg(windows)]
    #[test]
    fn exclusively_locked_file_is_reported_and_skipped() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().expect("create temp directory");
        let locked_path = temp.path().join("locked.bin");
        let readable_path = temp.path().join("readable.bin");
        write_file(&locked_path, b"same-size");
        write_file(&readable_path, b"different");
        let locked = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&locked_path)
            .expect("hold exclusive file lock");

        let report = scan_path(temp.path(), test_config(), |_| {}, || false).expect("scan");

        drop(locked);
        assert!(report.unreadable_entries >= 1);
        assert!(report.issues.iter().any(|issue| {
            issue
                .path
                .as_deref()
                .is_some_and(|path| path.ends_with("locked.bin"))
        }));
    }
}
