use rusqlite::types::Value;
use rusqlite::{
    Connection, OptionalExtension, Statement, Transaction, TransactionBehavior, params,
    params_from_iter,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::{bounded_worker_threads, system_time_ms};

#[cfg(windows)]
mod windows_ntfs;

const INDEX_SCHEMA_VERSION: i64 = 2;
const DEFAULT_MAX_ENTRIES: usize = 2_000_000;
const DEFAULT_MAX_ISSUES: usize = 100;
const MAX_ENTRIES: usize = 5_000_000;
const MAX_ISSUES: usize = 1_000;
const MAX_SEARCH_RESULTS: usize = 250;
const MAX_QUERY_CHARS: usize = 256;
const PROGRESS_ENTRY_INTERVAL: u64 = 512;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FileCatalogConfig {
    pub max_entries: usize,
    pub max_issues: usize,
}

impl Default for FileCatalogConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_issues: DEFAULT_MAX_ISSUES,
        }
    }
}

impl FileCatalogConfig {
    fn bounded(mut self) -> Self {
        self.max_entries = self.max_entries.clamp(1, MAX_ENTRIES);
        self.max_issues = self.max_issues.min(MAX_ISSUES);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileCatalogPhase {
    Discovering,
    ApplyingChanges,
    Finalizing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogProgress {
    pub phase: FileCatalogPhase,
    pub message: String,
    pub scanned_entries: u64,
    pub indexed_entries: u64,
    pub indexed_files: u64,
    pub indexed_directories: u64,
    pub processed_bytes: u64,
    pub unreadable_entries: u64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileCatalogProvider {
    PortableWalk,
    WindowsNtfs,
}

impl FileCatalogProvider {
    fn database_value(self) -> &'static str {
        match self {
            Self::PortableWalk => "portableWalk",
            Self::WindowsNtfs => "windowsNtfs",
        }
    }

    fn from_database(value: &str) -> Self {
        match value {
            "windowsNtfs" => Self::WindowsNtfs,
            _ => Self::PortableWalk,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileCatalogRefreshMode {
    Full,
    Incremental,
}

impl FileCatalogRefreshMode {
    fn database_value(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Incremental => "incremental",
        }
    }

    fn from_database(value: &str) -> Self {
        match value {
            "incremental" => Self::Incremental,
            _ => Self::Full,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogStatus {
    pub root: String,
    pub completed_at_unix_ms: u128,
    pub duration_ms: u128,
    pub indexed_entries: u64,
    pub indexed_files: u64,
    pub indexed_directories: u64,
    pub indexed_symlinks: u64,
    pub indexed_bytes: u64,
    pub unreadable_entries: u64,
    pub entry_limit_reached: bool,
    pub provider: FileCatalogProvider,
    pub refresh_mode: FileCatalogRefreshMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogReport {
    #[serde(flatten)]
    pub status: FileCatalogStatus,
    pub scanned_entries: u64,
    pub removed_entries: u64,
    pub issues: Vec<FileCatalogIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogIssue {
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileCatalogEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

impl FileCatalogEntryKind {
    fn database_value(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }

    fn from_database(value: &str) -> Self {
        match value {
            "directory" => Self::Directory,
            "symlink" => Self::Symlink,
            "other" => Self::Other,
            _ => Self::File,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileCatalogSort {
    #[default]
    Relevance,
    Name,
    Largest,
    Modified,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogSearchRequest {
    pub query: String,
    #[serde(default)]
    pub kind: Option<FileCatalogEntryKind>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub min_bytes: Option<u64>,
    #[serde(default)]
    pub max_bytes: Option<u64>,
    #[serde(default)]
    pub sort: FileCatalogSort,
    #[serde(default = "default_search_results")]
    pub max_results: usize,
}

fn default_search_results() -> usize {
    100
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileCatalogMatchSource {
    Name,
    Path,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogSearchResult {
    pub name: String,
    pub path: String,
    pub parent: String,
    pub extension: String,
    pub kind: FileCatalogEntryKind,
    pub logical_bytes: u64,
    pub modified_at_unix_ms: Option<u128>,
    pub match_source: FileCatalogMatchSource,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCatalogSearchReport {
    pub root: String,
    pub query: String,
    pub indexed_entries: u64,
    pub search_duration_ms: u128,
    pub results_truncated: bool,
    pub results: Vec<FileCatalogSearchResult>,
}

#[derive(Debug, Error)]
pub enum FileCatalogError {
    #[error("file catalog path does not exist: {0}")]
    MissingPath(String),
    #[error("file catalog path is not a directory: {0}")]
    NotDirectory(String),
    #[error("file catalog build was cancelled")]
    Cancelled,
    #[error("enter at least one search character")]
    EmptyQuery,
    #[error("search query is longer than {MAX_QUERY_CHARS} characters")]
    QueryTooLong,
    #[error("no completed file catalog is available")]
    IndexUnavailable,
    #[error("failed to access the file catalog: {0}")]
    Index(String),
}

#[derive(Debug)]
struct CatalogRecord {
    path: String,
    name: String,
    parent: String,
    extension: String,
    kind: FileCatalogEntryKind,
    logical_bytes: u64,
    modified_at_ms: Option<u128>,
    source_record_id: Option<u64>,
    source_parent_record_id: Option<u64>,
}

trait CatalogRecordSink {
    fn set_scanned_entries(&mut self, scanned_entries: u64);
    fn push(&mut self, record: CatalogRecord) -> Result<bool, String>;
}

enum CatalogSource {
    PortableWalk,
    #[cfg(windows)]
    WindowsNtfs(windows_ntfs::NtfsSource),
}

impl CatalogSource {
    fn select(root: &Path) -> (Self, Option<FileCatalogIssue>) {
        #[cfg(windows)]
        {
            match windows_ntfs::NtfsSource::try_open(root) {
                windows_ntfs::NtfsAvailability::Ready(source) => (Self::WindowsNtfs(source), None),
                windows_ntfs::NtfsAvailability::Unavailable(message) => (
                    Self::PortableWalk,
                    Some(FileCatalogIssue {
                        path: Some(display_path(root)),
                        message,
                    }),
                ),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = root;
            (Self::PortableWalk, None)
        }
    }

    fn provider(&self) -> FileCatalogProvider {
        match self {
            Self::PortableWalk => FileCatalogProvider::PortableWalk,
            #[cfg(windows)]
            Self::WindowsNtfs(_) => FileCatalogProvider::WindowsNtfs,
        }
    }

    #[cfg(windows)]
    fn ntfs(&self) -> Option<&windows_ntfs::NtfsSource> {
        match self {
            Self::WindowsNtfs(source) => Some(source),
            Self::PortableWalk => None,
        }
    }
}

pub fn build_file_catalog<F, C>(
    root: impl AsRef<Path>,
    database_path: impl AsRef<Path>,
    config: FileCatalogConfig,
    mut on_progress: F,
    should_cancel: C,
) -> Result<FileCatalogReport, FileCatalogError>
where
    F: FnMut(FileCatalogProgress),
    C: Fn() -> bool + Sync,
{
    let started = Instant::now();
    let config = config.bounded();
    let requested_root = root.as_ref();
    if !requested_root.exists() {
        return Err(FileCatalogError::MissingPath(display_path(requested_root)));
    }
    if !requested_root.is_dir() {
        return Err(FileCatalogError::NotDirectory(display_path(requested_root)));
    }

    let root = requested_root
        .canonicalize()
        .map_err(|error| FileCatalogError::Index(error.to_string()))?;
    let root_string = display_path(&root);
    let database_path = database_path.as_ref().to_path_buf();
    let mut connection = open_index(&database_path)?;
    initialize_schema(&connection)?;
    let previous = read_meta(&connection)?;
    let (mut source, fallback_issue) = CatalogSource::select(&root);
    let mut initial_issues = fallback_issue.into_iter().collect::<Vec<_>>();

    #[cfg(windows)]
    if let (Some(ntfs), Some(previous)) = (source.ntfs(), previous.as_ref())
        && previous.can_apply_ntfs_delta(&root_string)
        && let Some(checkpoint) = previous.ntfs_checkpoint()
    {
        match ntfs.read_journal_delta(checkpoint, &should_cancel) {
            Ok(windows_ntfs::JournalDelta::Changes {
                changes,
                checkpoint,
            }) => match apply_ntfs_incremental(
                &mut connection,
                &database_path,
                &root_string,
                previous,
                ntfs,
                changes,
                checkpoint,
                config.clone(),
                &mut on_progress,
                &should_cancel,
                started,
            )? {
                IncrementalBuild::Applied(report) => return Ok(report),
                IncrementalBuild::FullRequired(message) => {
                    push_issue(
                        &mut initial_issues,
                        config.max_issues,
                        Some(root_string.clone()),
                        message,
                    );
                }
            },
            Ok(windows_ntfs::JournalDelta::FullRequired(message)) => {
                push_issue(
                    &mut initial_issues,
                    config.max_issues,
                    Some(root_string.clone()),
                    message,
                );
            }
            Err(windows_ntfs::NtfsError::Cancelled) => {
                return Err(FileCatalogError::Cancelled);
            }
            Err(error) => {
                push_issue(
                    &mut initial_issues,
                    config.max_issues,
                    Some(root_string.clone()),
                    format!("USN 변경분을 적용하지 못해 전체 MFT를 다시 읽습니다: {error}"),
                );
            }
        }
    }

    build_full_catalog(
        &root,
        &root_string,
        &database_path,
        &mut connection,
        &mut source,
        config,
        &mut on_progress,
        &should_cancel,
        initial_issues,
        started,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_full_catalog<F, C>(
    root: &Path,
    root_string: &str,
    database_path: &Path,
    connection: &mut Connection,
    source: &mut CatalogSource,
    config: FileCatalogConfig,
    on_progress: &mut F,
    should_cancel: &C,
    initial_issues: Vec<FileCatalogIssue>,
    started: Instant,
) -> Result<FileCatalogReport, FileCatalogError>
where
    F: FnMut(FileCatalogProgress),
    C: Fn() -> bool + Sync,
{
    let provider = source.provider();
    let excluded_storage_paths = catalog_storage_paths(database_path);
    on_progress(FileCatalogProgress {
        phase: FileCatalogPhase::Discovering,
        message: match provider {
            FileCatalogProvider::WindowsNtfs => {
                "NTFS MFT에서 파일명과 경로 메타데이터를 읽고 있습니다".to_owned()
            }
            FileCatalogProvider::PortableWalk => {
                "파일명과 경로 메타데이터를 확인하고 있습니다".to_owned()
            }
        },
        scanned_entries: 0,
        indexed_entries: 0,
        indexed_files: 0,
        indexed_directories: 0,
        processed_bytes: 0,
        unreadable_entries: 0,
    });

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(index_error)?;
    let previous = read_meta(&transaction)?;
    suspend_bulk_indexes(&transaction)?;
    let mut removed_entries = 0_u64;
    if previous
        .as_ref()
        .is_some_and(|meta| !same_root(&meta.root, root_string))
    {
        removed_entries = transaction
            .execute("DELETE FROM file_catalog_entries", [])
            .map_err(index_error)? as u64;
    }
    let generation = previous
        .as_ref()
        .map_or(1, |meta| meta.generation.saturating_add(1));
    let statement = prepare_upsert(&transaction)?;
    let mut writer = CatalogWriter::new(
        statement,
        generation,
        config.max_entries,
        config.max_issues,
        FileCatalogPhase::Discovering,
        match provider {
            FileCatalogProvider::WindowsNtfs => "NTFS MFT 항목을 검색 카탈로그에 저장하고 있습니다",
            FileCatalogProvider::PortableWalk => "파일명과 경로 메타데이터를 색인하고 있습니다",
        },
        on_progress,
        initial_issues,
    );

    match source {
        CatalogSource::PortableWalk => {
            enumerate_portable_walk(root, &excluded_storage_paths, &mut writer, should_cancel)?
        }
        #[cfg(windows)]
        CatalogSource::WindowsNtfs(source) => {
            match source.enumerate_full(&excluded_storage_paths, &mut writer, should_cancel) {
                Ok(stats) => {
                    writer.set_scanned_entries(stats.scanned_records);
                    if stats.malformed_records > 0 {
                        writer.add_unreadable_entries(stats.malformed_records);
                        writer.issue(
                            None,
                            format!(
                                "검증에 실패한 MFT 레코드 {}개를 건너뛰었습니다",
                                stats.malformed_records
                            ),
                        );
                    }
                }
                Err(windows_ntfs::NtfsError::Cancelled) => {
                    return Err(FileCatalogError::Cancelled);
                }
                Err(error) => {
                    return Err(FileCatalogError::Index(format!(
                        "NTFS MFT 카탈로그를 만들지 못했습니다: {error}"
                    )));
                }
            }
        }
    }
    let stats = writer.finish();

    if should_cancel() {
        return Err(FileCatalogError::Cancelled);
    }
    on_progress(FileCatalogProgress {
        phase: FileCatalogPhase::Finalizing,
        message: "사라진 항목을 제외하고 검색 색인을 확정하고 있습니다".to_owned(),
        scanned_entries: stats.scanned_entries,
        indexed_entries: stats.indexed_entries,
        indexed_files: stats.indexed_files,
        indexed_directories: stats.indexed_directories,
        processed_bytes: stats.indexed_bytes,
        unreadable_entries: stats.unreadable_entries,
    });
    removed_entries = removed_entries.saturating_add(
        transaction
            .execute(
                "DELETE FROM file_catalog_entries WHERE generation <> ?1",
                [generation],
            )
            .map_err(index_error)? as u64,
    );
    rebuild_bulk_indexes(&transaction)?;

    let completed_at = unix_time_ms();
    let duration_ms = started.elapsed().as_millis();
    let source_meta = source_meta(source);
    let status = FileCatalogStatus {
        root: root_string.to_owned(),
        completed_at_unix_ms: completed_at,
        duration_ms,
        indexed_entries: stats.indexed_entries,
        indexed_files: stats.indexed_files,
        indexed_directories: stats.indexed_directories,
        indexed_symlinks: stats.indexed_symlinks,
        indexed_bytes: stats.indexed_bytes,
        unreadable_entries: stats.unreadable_entries,
        entry_limit_reached: stats.entry_limit_reached,
        provider,
        refresh_mode: FileCatalogRefreshMode::Full,
    };
    write_meta(&transaction, &status, generation, source_meta)?;
    transaction.commit().map_err(index_error)?;
    checkpoint_index(connection)?;

    Ok(FileCatalogReport {
        status,
        scanned_entries: stats.scanned_entries,
        removed_entries,
        issues: stats.issues,
    })
}

struct CatalogWriter<'statement, 'progress> {
    upsert: Statement<'statement>,
    generation: i64,
    max_entries: usize,
    max_issues: usize,
    phase: FileCatalogPhase,
    message: &'static str,
    on_progress: &'progress mut dyn FnMut(FileCatalogProgress),
    last_progress: Instant,
    stats: CatalogWriteStats,
}

#[derive(Default)]
struct CatalogWriteStats {
    scanned_entries: u64,
    indexed_entries: u64,
    indexed_files: u64,
    indexed_directories: u64,
    indexed_symlinks: u64,
    indexed_bytes: u64,
    unreadable_entries: u64,
    entry_limit_reached: bool,
    issues: Vec<FileCatalogIssue>,
}

impl<'statement, 'progress> CatalogWriter<'statement, 'progress> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        upsert: Statement<'statement>,
        generation: i64,
        max_entries: usize,
        max_issues: usize,
        phase: FileCatalogPhase,
        message: &'static str,
        on_progress: &'progress mut dyn FnMut(FileCatalogProgress),
        issues: Vec<FileCatalogIssue>,
    ) -> Self {
        Self {
            upsert,
            generation,
            max_entries,
            max_issues,
            phase,
            message,
            on_progress,
            last_progress: Instant::now(),
            stats: CatalogWriteStats {
                issues,
                ..CatalogWriteStats::default()
            },
        }
    }

    fn bump_scanned_entries(&mut self) {
        self.stats.scanned_entries = self.stats.scanned_entries.saturating_add(1);
        self.emit_progress_if_needed(false);
    }

    fn add_unreadable_entries(&mut self, count: u64) {
        self.stats.unreadable_entries = self.stats.unreadable_entries.saturating_add(count);
    }

    fn issue(&mut self, path: Option<String>, message: String) {
        push_issue(&mut self.stats.issues, self.max_issues, path, message);
    }

    fn emit_progress_if_needed(&mut self, indexed: bool) {
        if (indexed
            && self
                .stats
                .indexed_entries
                .is_multiple_of(PROGRESS_ENTRY_INTERVAL))
            || self.last_progress.elapsed() >= Duration::from_millis(100)
        {
            (self.on_progress)(FileCatalogProgress {
                phase: self.phase,
                message: self.message.to_owned(),
                scanned_entries: self.stats.scanned_entries,
                indexed_entries: self.stats.indexed_entries,
                indexed_files: self.stats.indexed_files,
                indexed_directories: self.stats.indexed_directories,
                processed_bytes: self.stats.indexed_bytes,
                unreadable_entries: self.stats.unreadable_entries,
            });
            self.last_progress = Instant::now();
        }
    }

    fn finish(self) -> CatalogWriteStats {
        self.stats
    }
}

impl CatalogRecordSink for CatalogWriter<'_, '_> {
    fn set_scanned_entries(&mut self, scanned_entries: u64) {
        self.stats.scanned_entries = self.stats.scanned_entries.max(scanned_entries);
        self.emit_progress_if_needed(false);
    }

    fn push(&mut self, record: CatalogRecord) -> Result<bool, String> {
        if self.stats.indexed_entries >= self.max_entries as u64 {
            self.stats.entry_limit_reached = true;
            return Ok(false);
        }
        self.upsert
            .execute(params![
                record.path,
                record.name,
                record.parent,
                record.extension,
                record.kind.database_value(),
                saturating_u64_to_i64(record.logical_bytes),
                record.modified_at_ms.map(saturating_u128_to_i64),
                self.generation,
                record.source_record_id.map(saturating_u64_to_i64),
                record.source_parent_record_id.map(saturating_u64_to_i64),
            ])
            .map_err(|error| error.to_string())?;

        self.stats.indexed_entries = self.stats.indexed_entries.saturating_add(1);
        self.stats.indexed_bytes = self
            .stats
            .indexed_bytes
            .saturating_add(record.logical_bytes);
        match record.kind {
            FileCatalogEntryKind::File => {
                self.stats.indexed_files = self.stats.indexed_files.saturating_add(1)
            }
            FileCatalogEntryKind::Directory => {
                self.stats.indexed_directories = self.stats.indexed_directories.saturating_add(1)
            }
            FileCatalogEntryKind::Symlink => {
                self.stats.indexed_symlinks = self.stats.indexed_symlinks.saturating_add(1)
            }
            FileCatalogEntryKind::Other => {}
        }
        self.emit_progress_if_needed(true);
        Ok(true)
    }
}

fn prepare_upsert<'statement>(
    transaction: &'statement Transaction<'_>,
) -> Result<Statement<'statement>, FileCatalogError> {
    transaction
        .prepare(
            "INSERT INTO file_catalog_entries (
                path, name, parent, extension, kind, logical_bytes, modified_at_ms, generation,
                source_record_id, source_parent_record_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                parent = excluded.parent,
                extension = excluded.extension,
                kind = excluded.kind,
                logical_bytes = excluded.logical_bytes,
                modified_at_ms = excluded.modified_at_ms,
                generation = excluded.generation,
                source_record_id = excluded.source_record_id,
                source_parent_record_id = excluded.source_parent_record_id",
        )
        .map_err(index_error)
}

fn suspend_bulk_indexes(connection: &Connection) -> Result<(), FileCatalogError> {
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS file_catalog_ai;
             DROP TRIGGER IF EXISTS file_catalog_ad;
             DROP TRIGGER IF EXISTS file_catalog_au;
             DROP INDEX IF EXISTS file_catalog_generation_idx;
             DROP INDEX IF EXISTS file_catalog_kind_idx;
             DROP INDEX IF EXISTS file_catalog_extension_idx;
             DROP INDEX IF EXISTS file_catalog_size_idx;
             DROP INDEX IF EXISTS file_catalog_modified_idx;
             DROP INDEX IF EXISTS file_catalog_source_record_idx;",
        )
        .map_err(index_error)
}

fn rebuild_bulk_indexes(connection: &Connection) -> Result<(), FileCatalogError> {
    connection
        .execute_batch(
            "CREATE INDEX file_catalog_generation_idx
                ON file_catalog_entries(generation);
             CREATE INDEX file_catalog_kind_idx
                ON file_catalog_entries(kind);
             CREATE INDEX file_catalog_extension_idx
                ON file_catalog_entries(extension);
             CREATE INDEX file_catalog_size_idx
                ON file_catalog_entries(logical_bytes DESC);
             CREATE INDEX file_catalog_modified_idx
                ON file_catalog_entries(modified_at_ms DESC);
             CREATE INDEX file_catalog_source_record_idx
                ON file_catalog_entries(source_record_id);
             INSERT INTO file_catalog_fts(file_catalog_fts) VALUES ('rebuild');
             CREATE TRIGGER file_catalog_ai
             AFTER INSERT ON file_catalog_entries BEGIN
                INSERT INTO file_catalog_fts(rowid, name, path)
                VALUES (new.id, new.name, new.path);
             END;
             CREATE TRIGGER file_catalog_ad
             AFTER DELETE ON file_catalog_entries BEGIN
                INSERT INTO file_catalog_fts(file_catalog_fts, rowid, name, path)
                VALUES ('delete', old.id, old.name, old.path);
             END;
             CREATE TRIGGER file_catalog_au
             AFTER UPDATE OF name, path ON file_catalog_entries
             WHEN old.name IS NOT new.name OR old.path IS NOT new.path BEGIN
                INSERT INTO file_catalog_fts(file_catalog_fts, rowid, name, path)
                VALUES ('delete', old.id, old.name, old.path);
                INSERT INTO file_catalog_fts(rowid, name, path)
                VALUES (new.id, new.name, new.path);
             END;",
        )
        .map_err(index_error)
}

fn enumerate_portable_walk<C>(
    root: &Path,
    excluded_storage_paths: &[PathBuf],
    writer: &mut CatalogWriter<'_, '_>,
    should_cancel: &C,
) -> Result<(), FileCatalogError>
where
    C: Fn() -> bool + Sync,
{
    for item in jwalk::WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(bounded_worker_threads()))
    {
        if should_cancel() {
            return Err(FileCatalogError::Cancelled);
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                writer.add_unreadable_entries(1);
                writer.issue(None, error.to_string());
                continue;
            }
        };
        let path = entry.path();
        if path == root || is_catalog_storage_path(&path, excluded_storage_paths) {
            continue;
        }
        writer.bump_scanned_entries();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                writer.add_unreadable_entries(1);
                writer.issue(Some(display_path(&path)), error.to_string());
                continue;
            }
        };
        let file_type = metadata.file_type();
        let kind = if file_type.is_file() {
            FileCatalogEntryKind::File
        } else if file_type.is_dir() {
            FileCatalogEntryKind::Directory
        } else if file_type.is_symlink() {
            FileCatalogEntryKind::Symlink
        } else {
            FileCatalogEntryKind::Other
        };
        let logical_bytes = if kind == FileCatalogEntryKind::File {
            metadata.len()
        } else {
            0
        };
        let extension = if kind == FileCatalogEntryKind::Directory {
            String::new()
        } else {
            path.extension()
                .map(|value| value.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        };
        let record = CatalogRecord {
            path: display_path(&path),
            name: display_name(&path),
            parent: path.parent().map(display_path).unwrap_or_default(),
            extension,
            kind,
            logical_bytes,
            modified_at_ms: system_time_ms(metadata.modified().ok()),
            source_record_id: None,
            source_parent_record_id: None,
        };
        if !writer.push(record).map_err(FileCatalogError::Index)? {
            break;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct SourceMeta {
    volume_serial: Option<u32>,
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    root_record_id: Option<u64>,
}

fn source_meta(source: &CatalogSource) -> SourceMeta {
    match source {
        CatalogSource::PortableWalk => SourceMeta::default(),
        #[cfg(windows)]
        CatalogSource::WindowsNtfs(source) => {
            let checkpoint = source.checkpoint();
            SourceMeta {
                volume_serial: Some(source.volume_serial()),
                journal_id: checkpoint.map(|value| value.journal_id),
                next_usn: checkpoint.map(|value| value.next_usn),
                root_record_id: Some(source.root_record_id()),
            }
        }
    }
}

fn write_meta(
    connection: &Connection,
    status: &FileCatalogStatus,
    generation: i64,
    source: SourceMeta,
) -> Result<(), FileCatalogError> {
    connection
        .execute(
            "INSERT INTO file_catalog_meta (
                id, schema_version, root, completed_at_ms, duration_ms, generation,
                indexed_entries, indexed_files, indexed_directories, indexed_symlinks,
                indexed_bytes, unreadable_entries, entry_limit_reached, provider, refresh_mode,
                volume_serial, journal_id, next_usn, root_record_id
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                       ?15, ?16, ?17, ?18)
             ON CONFLICT(id) DO UPDATE SET
                schema_version = excluded.schema_version,
                root = excluded.root,
                completed_at_ms = excluded.completed_at_ms,
                duration_ms = excluded.duration_ms,
                generation = excluded.generation,
                indexed_entries = excluded.indexed_entries,
                indexed_files = excluded.indexed_files,
                indexed_directories = excluded.indexed_directories,
                indexed_symlinks = excluded.indexed_symlinks,
                indexed_bytes = excluded.indexed_bytes,
                unreadable_entries = excluded.unreadable_entries,
                entry_limit_reached = excluded.entry_limit_reached,
                provider = excluded.provider,
                refresh_mode = excluded.refresh_mode,
                volume_serial = excluded.volume_serial,
                journal_id = excluded.journal_id,
                next_usn = excluded.next_usn,
                root_record_id = excluded.root_record_id",
            params![
                INDEX_SCHEMA_VERSION,
                status.root,
                saturating_u128_to_i64(status.completed_at_unix_ms),
                saturating_u128_to_i64(status.duration_ms),
                generation,
                saturating_u64_to_i64(status.indexed_entries),
                saturating_u64_to_i64(status.indexed_files),
                saturating_u64_to_i64(status.indexed_directories),
                saturating_u64_to_i64(status.indexed_symlinks),
                saturating_u64_to_i64(status.indexed_bytes),
                saturating_u64_to_i64(status.unreadable_entries),
                i64::from(status.entry_limit_reached),
                status.provider.database_value(),
                status.refresh_mode.database_value(),
                source.volume_serial.map(i64::from),
                source.journal_id.map(|value| format!("{value:016X}")),
                source.next_usn,
                source.root_record_id.map(saturating_u64_to_i64),
            ],
        )
        .map_err(index_error)?;
    Ok(())
}

fn checkpoint_index(connection: &Connection) -> Result<(), FileCatalogError> {
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(index_error)
}

#[cfg(windows)]
enum IncrementalBuild {
    Applied(FileCatalogReport),
    FullRequired(String),
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn apply_ntfs_incremental<F, C>(
    connection: &mut Connection,
    database_path: &Path,
    root: &str,
    previous: &IndexMeta,
    source: &windows_ntfs::NtfsSource,
    changes: Vec<windows_ntfs::JournalChange>,
    checkpoint: windows_ntfs::NtfsCheckpoint,
    config: FileCatalogConfig,
    on_progress: &mut F,
    should_cancel: &C,
    started: Instant,
) -> Result<IncrementalBuild, FileCatalogError>
where
    F: FnMut(FileCatalogProgress),
    C: Fn() -> bool + Sync,
{
    if should_cancel() {
        return Err(FileCatalogError::Cancelled);
    }
    let directory_paths =
        load_ntfs_directory_paths(connection, checkpoint.root_record_id, PathBuf::from(root))?;
    let directory_ids = directory_paths.keys().copied().collect::<HashSet<_>>();
    let mut lookup = connection
        .prepare(
            "SELECT kind FROM file_catalog_entries
             WHERE source_record_id = ?1 LIMIT 1",
        )
        .map_err(index_error)?;
    let mut relevant = Vec::new();
    for change in changes {
        let indexed_kind = lookup
            .query_row([saturating_u64_to_i64(change.record_id)], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .map_err(index_error)?;
        let parent_is_inside = change
            .parent_ids
            .iter()
            .any(|parent| directory_ids.contains(parent));
        if indexed_kind.is_none() && !parent_is_inside {
            continue;
        }
        if change.directory_hint || indexed_kind.as_deref() == Some("directory") {
            return Ok(IncrementalBuild::FullRequired(
                "선택 범위 안의 폴더 구조가 바뀌어 전체 MFT 경로를 다시 계산합니다".to_owned(),
            ));
        }
        relevant.push(change);
    }
    drop(lookup);

    if relevant.len() > config.max_entries {
        return Ok(IncrementalBuild::FullRequired(
            "선택 범위의 변경 항목이 카탈로그 상한보다 많아 전체 MFT를 다시 읽습니다".to_owned(),
        ));
    }

    on_progress(FileCatalogProgress {
        phase: FileCatalogPhase::ApplyingChanges,
        message: "USN Journal 변경분만 검색 카탈로그에 반영하고 있습니다".to_owned(),
        scanned_entries: 0,
        indexed_entries: 0,
        indexed_files: 0,
        indexed_directories: 0,
        processed_bytes: 0,
        unreadable_entries: previous.unreadable_entries.max(0) as u64,
    });

    let excluded_storage_paths = catalog_storage_paths(database_path);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(index_error)?;
    transaction
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS file_catalog_changed_ids (
                id INTEGER PRIMARY KEY
             );
             DELETE FROM file_catalog_changed_ids;",
        )
        .map_err(index_error)?;
    {
        let mut insert_changed = transaction
            .prepare("INSERT OR IGNORE INTO file_catalog_changed_ids (id) VALUES (?1)")
            .map_err(index_error)?;
        for change in &relevant {
            insert_changed
                .execute([saturating_u64_to_i64(change.record_id)])
                .map_err(index_error)?;
        }
    }
    let generation = previous.generation.saturating_add(1);
    let statement = prepare_upsert(&transaction)?;
    let mut writer = CatalogWriter::new(
        statement,
        generation,
        config.max_entries,
        config.max_issues,
        FileCatalogPhase::ApplyingChanges,
        "USN 변경 파일의 최신 MFT 메타데이터를 저장하고 있습니다",
        on_progress,
        Vec::new(),
    );
    let outcome = source.enumerate_changed(
        &relevant,
        &directory_paths,
        &excluded_storage_paths,
        &mut writer,
        should_cancel,
    );
    match outcome {
        Ok(windows_ntfs::ChangedRecordsOutcome::Applied(stats)) => {
            if stats.malformed_records > 0 {
                writer.add_unreadable_entries(stats.malformed_records);
                writer.issue(
                    None,
                    format!(
                        "검증에 실패한 변경 MFT 레코드 {}개를 건너뛰었습니다",
                        stats.malformed_records
                    ),
                );
            }
        }
        Ok(windows_ntfs::ChangedRecordsOutcome::FullRequired(message)) => {
            return Ok(IncrementalBuild::FullRequired(message));
        }
        Err(windows_ntfs::NtfsError::Cancelled) => {
            return Err(FileCatalogError::Cancelled);
        }
        Err(error) => {
            return Ok(IncrementalBuild::FullRequired(format!(
                "USN 변경 파일을 읽지 못해 전체 MFT를 다시 읽습니다: {error}"
            )));
        }
    }
    let write_stats = writer.finish();
    if should_cancel() {
        return Err(FileCatalogError::Cancelled);
    }
    let removed_entries = transaction
        .execute(
            "DELETE FROM file_catalog_entries
             WHERE source_record_id IN (SELECT id FROM file_catalog_changed_ids)
               AND generation <> ?1",
            [generation],
        )
        .map_err(index_error)? as u64;
    let totals = catalog_totals(&transaction)?;
    if totals.indexed_entries > config.max_entries as u64 {
        return Ok(IncrementalBuild::FullRequired(
            "증분 반영 결과가 카탈로그 상한을 넘어 전체 범위를 다시 확인합니다".to_owned(),
        ));
    }
    on_progress(FileCatalogProgress {
        phase: FileCatalogPhase::Finalizing,
        message: "USN 변경분과 검색 색인을 확정하고 있습니다".to_owned(),
        scanned_entries: relevant.len() as u64,
        indexed_entries: write_stats.indexed_entries,
        indexed_files: write_stats.indexed_files,
        indexed_directories: write_stats.indexed_directories,
        processed_bytes: write_stats.indexed_bytes,
        unreadable_entries: write_stats.unreadable_entries,
    });

    let completed_at = unix_time_ms();
    let duration_ms = started.elapsed().as_millis();
    let status = FileCatalogStatus {
        root: root.to_owned(),
        completed_at_unix_ms: completed_at,
        duration_ms,
        indexed_entries: totals.indexed_entries,
        indexed_files: totals.indexed_files,
        indexed_directories: totals.indexed_directories,
        indexed_symlinks: totals.indexed_symlinks,
        indexed_bytes: totals.indexed_bytes,
        unreadable_entries: previous.unreadable_entries.max(0) as u64,
        entry_limit_reached: false,
        provider: FileCatalogProvider::WindowsNtfs,
        refresh_mode: FileCatalogRefreshMode::Incremental,
    };
    write_meta(
        &transaction,
        &status,
        generation,
        SourceMeta {
            volume_serial: Some(checkpoint.volume_serial),
            journal_id: Some(checkpoint.journal_id),
            next_usn: Some(checkpoint.next_usn),
            root_record_id: Some(checkpoint.root_record_id),
        },
    )?;
    transaction
        .execute("DELETE FROM file_catalog_changed_ids", [])
        .map_err(index_error)?;
    transaction.commit().map_err(index_error)?;
    checkpoint_index(connection)?;

    Ok(IncrementalBuild::Applied(FileCatalogReport {
        status,
        scanned_entries: relevant.len() as u64,
        removed_entries,
        issues: write_stats.issues,
    }))
}

#[cfg(windows)]
fn load_ntfs_directory_paths(
    connection: &Connection,
    root_record_id: u64,
    root: PathBuf,
) -> Result<HashMap<u64, PathBuf>, FileCatalogError> {
    let mut directories = HashMap::new();
    directories.insert(root_record_id, root);
    let mut statement = connection
        .prepare(
            "SELECT source_record_id, path FROM file_catalog_entries
             WHERE kind = 'directory' AND source_record_id IS NOT NULL",
        )
        .map_err(index_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(index_error)?;
    for row in rows {
        let (record_id, path) = row.map_err(index_error)?;
        if record_id >= 0 {
            directories
                .entry(record_id as u64)
                .or_insert_with(|| PathBuf::from(path));
        }
    }
    Ok(directories)
}

#[derive(Debug, Clone, Copy, Default)]
struct CatalogTotals {
    indexed_entries: u64,
    indexed_files: u64,
    indexed_directories: u64,
    indexed_symlinks: u64,
    indexed_bytes: u64,
}

fn catalog_totals(connection: &Connection) -> Result<CatalogTotals, FileCatalogError> {
    connection
        .query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(kind = 'file'), 0),
                    COALESCE(SUM(kind = 'directory'), 0),
                    COALESCE(SUM(kind = 'symlink'), 0),
                    COALESCE(SUM(logical_bytes), 0)
             FROM file_catalog_entries",
            [],
            |row| {
                Ok(CatalogTotals {
                    indexed_entries: row.get::<_, i64>(0)?.max(0) as u64,
                    indexed_files: row.get::<_, i64>(1)?.max(0) as u64,
                    indexed_directories: row.get::<_, i64>(2)?.max(0) as u64,
                    indexed_symlinks: row.get::<_, i64>(3)?.max(0) as u64,
                    indexed_bytes: row.get::<_, i64>(4)?.max(0) as u64,
                })
            },
        )
        .map_err(index_error)
}

pub fn file_catalog_status(
    database_path: impl AsRef<Path>,
) -> Result<Option<FileCatalogStatus>, FileCatalogError> {
    let database_path = database_path.as_ref();
    if !database_path.exists() {
        return Ok(None);
    }
    let connection = open_index(database_path)?;
    initialize_schema(&connection)?;
    read_meta(&connection).map(|meta| meta.map(IndexMeta::into_status))
}

pub fn search_file_catalog(
    database_path: impl AsRef<Path>,
    request: FileCatalogSearchRequest,
) -> Result<FileCatalogSearchReport, FileCatalogError> {
    let started = Instant::now();
    let query = request.query.trim();
    if query.is_empty() {
        return Err(FileCatalogError::EmptyQuery);
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(FileCatalogError::QueryTooLong);
    }

    let database_path = database_path.as_ref();
    if !database_path.exists() {
        return Err(FileCatalogError::IndexUnavailable);
    }
    let connection = open_index(database_path)?;
    initialize_schema(&connection)?;
    let meta = read_meta(&connection)?.ok_or(FileCatalogError::IndexUnavailable)?;
    let status = meta.into_status();
    let extensions = normalized_extensions(request.extensions);
    let max_results = request.max_results.clamp(1, MAX_SEARCH_RESULTS);
    let use_trigram = query.chars().count() >= 3;
    let mut values = Vec::new();

    let (from_clause, base_predicate) = if use_trigram {
        values.push(Value::Text(fts_literal(query)));
        (
            "FROM file_catalog_fts JOIN file_catalog_entries e ON e.id = file_catalog_fts.rowid",
            format!("file_catalog_fts MATCH ?{}", values.len()),
        )
    } else {
        values.push(Value::Text(format!("%{}%", escape_like(query))));
        (
            "FROM file_catalog_entries e",
            format!(
                "(e.name LIKE ?{0} ESCAPE '\\' COLLATE NOCASE OR e.path LIKE ?{0} ESCAPE '\\' COLLATE NOCASE)",
                values.len()
            ),
        )
    };

    let mut predicates = vec![base_predicate];
    if let Some(kind) = request.kind {
        values.push(Value::Text(kind.database_value().to_owned()));
        predicates.push(format!("e.kind = ?{}", values.len()));
    }
    if !extensions.is_empty() {
        let mut placeholders = Vec::with_capacity(extensions.len());
        for extension in extensions {
            values.push(Value::Text(extension));
            placeholders.push(format!("?{}", values.len()));
        }
        predicates.push(format!("e.extension IN ({})", placeholders.join(", ")));
    }
    if let Some(min_bytes) = request.min_bytes {
        values.push(Value::Integer(saturating_u64_to_i64(min_bytes)));
        predicates.push(format!("e.logical_bytes >= ?{}", values.len()));
    }
    if let Some(max_bytes) = request.max_bytes {
        values.push(Value::Integer(saturating_u64_to_i64(max_bytes)));
        predicates.push(format!("e.logical_bytes <= ?{}", values.len()));
    }

    let order_clause = match request.sort {
        FileCatalogSort::Relevance if use_trigram => {
            "ORDER BY bm25(file_catalog_fts, 4.0, 1.0), length(e.path), e.name COLLATE NOCASE"
        }
        FileCatalogSort::Relevance => "",
        FileCatalogSort::Name => "ORDER BY e.name COLLATE NOCASE, e.path COLLATE NOCASE",
        FileCatalogSort::Largest => {
            "ORDER BY e.logical_bytes DESC, e.modified_at_ms DESC, e.name COLLATE NOCASE"
        }
        FileCatalogSort::Modified => "ORDER BY e.modified_at_ms DESC, e.name COLLATE NOCASE",
    };
    values.push(Value::Integer(
        max_results.saturating_add(1).min(MAX_SEARCH_RESULTS + 1) as i64,
    ));
    let sql = format!(
        "SELECT e.name, e.path, e.parent, e.extension, e.kind,
                e.logical_bytes, e.modified_at_ms
         {from_clause}
         WHERE {}
         {order_clause}
         LIMIT ?{}",
        predicates.join(" AND "),
        values.len()
    );

    let mut statement = connection.prepare(&sql).map_err(index_error)?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
            ))
        })
        .map_err(index_error)?;
    let mut results = Vec::new();
    for row in rows {
        let (name, path, parent, extension, kind, logical_bytes, modified_at) =
            row.map_err(index_error)?;
        let match_source = if contains_case_insensitive(&name, query) {
            FileCatalogMatchSource::Name
        } else {
            FileCatalogMatchSource::Path
        };
        results.push(FileCatalogSearchResult {
            name,
            path,
            parent,
            extension,
            kind: FileCatalogEntryKind::from_database(&kind),
            logical_bytes: logical_bytes.max(0) as u64,
            modified_at_unix_ms: modified_at.map(|value| value.max(0) as u128),
            match_source,
        });
    }
    let results_truncated = results.len() > max_results;
    results.truncate(max_results);

    Ok(FileCatalogSearchReport {
        root: status.root,
        query: query.to_owned(),
        indexed_entries: status.indexed_entries,
        search_duration_ms: started.elapsed().as_millis(),
        results_truncated,
        results,
    })
}

pub fn clear_file_catalog(database_path: impl AsRef<Path>) -> Result<bool, FileCatalogError> {
    let database_path = database_path.as_ref();
    if !database_path.exists() {
        return Ok(false);
    }
    let connection = open_index(database_path)?;
    initialize_schema(&connection)?;
    connection
        .execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM file_catalog_entries;
             DELETE FROM file_catalog_meta;
             COMMIT;
             PRAGMA wal_checkpoint(TRUNCATE);
             VACUUM;",
        )
        .map_err(index_error)?;
    Ok(true)
}

fn open_index(path: &Path) -> Result<Connection, FileCatalogError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| FileCatalogError::Index(error.to_string()))?;
    }
    let connection = Connection::open(path).map_err(index_error)?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(index_error)?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA secure_delete = ON;
             PRAGMA temp_store = MEMORY;",
        )
        .map_err(index_error)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> Result<(), FileCatalogError> {
    let schema_version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(index_error)?;
    if !(0..=INDEX_SCHEMA_VERSION).contains(&schema_version) {
        return Err(FileCatalogError::Index(format!(
            "unsupported file catalog schema version {schema_version}"
        )));
    }
    if schema_version == 1 {
        connection
            .execute_batch(
                "ALTER TABLE file_catalog_meta
                    ADD COLUMN refresh_mode TEXT NOT NULL DEFAULT 'full';
                 ALTER TABLE file_catalog_meta
                    ADD COLUMN volume_serial INTEGER;
                 ALTER TABLE file_catalog_meta
                    ADD COLUMN journal_id TEXT;
                 ALTER TABLE file_catalog_meta
                    ADD COLUMN next_usn INTEGER;
                 ALTER TABLE file_catalog_meta
                    ADD COLUMN root_record_id INTEGER;
                 ALTER TABLE file_catalog_entries
                    ADD COLUMN source_record_id INTEGER;
                 ALTER TABLE file_catalog_entries
                    ADD COLUMN source_parent_record_id INTEGER;
                 UPDATE file_catalog_meta SET schema_version = 2;
                 CREATE INDEX IF NOT EXISTS file_catalog_source_record_idx
                    ON file_catalog_entries(source_record_id);
                 PRAGMA user_version = 2;",
            )
            .map_err(index_error)?;
    }
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS file_catalog_meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                schema_version INTEGER NOT NULL,
                root TEXT NOT NULL,
                completed_at_ms INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                generation INTEGER NOT NULL,
                indexed_entries INTEGER NOT NULL,
                indexed_files INTEGER NOT NULL,
                indexed_directories INTEGER NOT NULL,
                indexed_symlinks INTEGER NOT NULL,
                indexed_bytes INTEGER NOT NULL,
                unreadable_entries INTEGER NOT NULL,
                entry_limit_reached INTEGER NOT NULL,
                provider TEXT NOT NULL,
                refresh_mode TEXT NOT NULL DEFAULT 'full',
                volume_serial INTEGER,
                journal_id TEXT,
                next_usn INTEGER,
                root_record_id INTEGER
             );
             CREATE TABLE IF NOT EXISTS file_catalog_entries (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                parent TEXT NOT NULL,
                extension TEXT NOT NULL,
                kind TEXT NOT NULL,
                logical_bytes INTEGER NOT NULL,
                modified_at_ms INTEGER,
                generation INTEGER NOT NULL,
                source_record_id INTEGER,
                source_parent_record_id INTEGER
             );
             CREATE INDEX IF NOT EXISTS file_catalog_generation_idx
                ON file_catalog_entries(generation);
             CREATE INDEX IF NOT EXISTS file_catalog_kind_idx
                ON file_catalog_entries(kind);
             CREATE INDEX IF NOT EXISTS file_catalog_extension_idx
                ON file_catalog_entries(extension);
             CREATE INDEX IF NOT EXISTS file_catalog_size_idx
                ON file_catalog_entries(logical_bytes DESC);
             CREATE INDEX IF NOT EXISTS file_catalog_modified_idx
                ON file_catalog_entries(modified_at_ms DESC);
             CREATE INDEX IF NOT EXISTS file_catalog_source_record_idx
                ON file_catalog_entries(source_record_id);
             CREATE VIRTUAL TABLE IF NOT EXISTS file_catalog_fts USING fts5(
                name,
                path,
                content = 'file_catalog_entries',
                content_rowid = 'id',
                tokenize = 'trigram'
             );
             CREATE TRIGGER IF NOT EXISTS file_catalog_ai
             AFTER INSERT ON file_catalog_entries BEGIN
                INSERT INTO file_catalog_fts(rowid, name, path)
                VALUES (new.id, new.name, new.path);
             END;
             CREATE TRIGGER IF NOT EXISTS file_catalog_ad
             AFTER DELETE ON file_catalog_entries BEGIN
                INSERT INTO file_catalog_fts(file_catalog_fts, rowid, name, path)
                VALUES ('delete', old.id, old.name, old.path);
             END;
             CREATE TRIGGER IF NOT EXISTS file_catalog_au
             AFTER UPDATE OF name, path ON file_catalog_entries
             WHEN old.name IS NOT new.name OR old.path IS NOT new.path BEGIN
                INSERT INTO file_catalog_fts(file_catalog_fts, rowid, name, path)
                VALUES ('delete', old.id, old.name, old.path);
                INSERT INTO file_catalog_fts(rowid, name, path)
                VALUES (new.id, new.name, new.path);
             END;
             PRAGMA user_version = 2;",
        )
        .map_err(index_error)
}

#[derive(Debug, Clone)]
struct IndexMeta {
    root: String,
    completed_at_ms: i64,
    duration_ms: i64,
    generation: i64,
    indexed_entries: i64,
    indexed_files: i64,
    indexed_directories: i64,
    indexed_symlinks: i64,
    indexed_bytes: i64,
    unreadable_entries: i64,
    entry_limit_reached: i64,
    provider: String,
    refresh_mode: String,
    volume_serial: Option<i64>,
    journal_id: Option<String>,
    next_usn: Option<i64>,
    root_record_id: Option<i64>,
}

impl IndexMeta {
    fn into_status(self) -> FileCatalogStatus {
        FileCatalogStatus {
            root: self.root,
            completed_at_unix_ms: self.completed_at_ms.max(0) as u128,
            duration_ms: self.duration_ms.max(0) as u128,
            indexed_entries: self.indexed_entries.max(0) as u64,
            indexed_files: self.indexed_files.max(0) as u64,
            indexed_directories: self.indexed_directories.max(0) as u64,
            indexed_symlinks: self.indexed_symlinks.max(0) as u64,
            indexed_bytes: self.indexed_bytes.max(0) as u64,
            unreadable_entries: self.unreadable_entries.max(0) as u64,
            entry_limit_reached: self.entry_limit_reached != 0,
            provider: FileCatalogProvider::from_database(&self.provider),
            refresh_mode: FileCatalogRefreshMode::from_database(&self.refresh_mode),
        }
    }

    #[cfg(windows)]
    fn can_apply_ntfs_delta(&self, root: &str) -> bool {
        FileCatalogProvider::from_database(&self.provider) == FileCatalogProvider::WindowsNtfs
            && self.entry_limit_reached == 0
            && same_root(&self.root, root)
    }

    #[cfg(windows)]
    fn ntfs_checkpoint(&self) -> Option<windows_ntfs::NtfsCheckpoint> {
        let volume_serial = u32::try_from(self.volume_serial?).ok()?;
        let journal_id = u64::from_str_radix(self.journal_id.as_deref()?, 16).ok()?;
        let root_record_id = u64::try_from(self.root_record_id?).ok()?;
        Some(windows_ntfs::NtfsCheckpoint {
            volume_serial,
            journal_id,
            next_usn: self.next_usn?,
            root_record_id,
        })
    }
}

fn read_meta(connection: &Connection) -> Result<Option<IndexMeta>, FileCatalogError> {
    connection
        .query_row(
            "SELECT root, completed_at_ms, duration_ms, generation,
                    indexed_entries, indexed_files, indexed_directories, indexed_symlinks,
                    indexed_bytes, unreadable_entries, entry_limit_reached, provider,
                    refresh_mode, volume_serial, journal_id, next_usn, root_record_id
             FROM file_catalog_meta WHERE id = 1 AND schema_version = ?1",
            [INDEX_SCHEMA_VERSION],
            |row| {
                Ok(IndexMeta {
                    root: row.get(0)?,
                    completed_at_ms: row.get(1)?,
                    duration_ms: row.get(2)?,
                    generation: row.get(3)?,
                    indexed_entries: row.get(4)?,
                    indexed_files: row.get(5)?,
                    indexed_directories: row.get(6)?,
                    indexed_symlinks: row.get(7)?,
                    indexed_bytes: row.get(8)?,
                    unreadable_entries: row.get(9)?,
                    entry_limit_reached: row.get(10)?,
                    provider: row.get(11)?,
                    refresh_mode: row.get(12)?,
                    volume_serial: row.get(13)?,
                    journal_id: row.get(14)?,
                    next_usn: row.get(15)?,
                    root_record_id: row.get(16)?,
                })
            },
        )
        .optional()
        .map_err(index_error)
}

fn normalized_extensions(extensions: Vec<String>) -> Vec<String> {
    let mut extensions = extensions
        .into_iter()
        .map(|extension| {
            extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase()
        })
        .filter(|extension| {
            !extension.is_empty()
                && extension.len() <= 32
                && extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
        .collect::<Vec<_>>();
    extensions.sort_unstable();
    extensions.dedup();
    extensions.truncate(32);
    extensions
}

fn fts_literal(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

fn escape_like(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn contains_case_insensitive(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(&query.to_lowercase())
}

fn catalog_storage_paths(database_path: &Path) -> Vec<PathBuf> {
    let database_path = database_path
        .canonicalize()
        .unwrap_or_else(|_| database_path.to_path_buf());
    let database_string = database_path.to_string_lossy();
    vec![
        database_path.clone(),
        PathBuf::from(format!("{database_string}-wal")),
        PathBuf::from(format!("{database_string}-shm")),
    ]
}

fn is_catalog_storage_path(path: &Path, storage_paths: &[PathBuf]) -> bool {
    storage_paths
        .iter()
        .any(|storage_path| paths_equal(path, storage_path))
}

#[cfg(windows)]
fn paths_equal(left: &Path, right: &Path) -> bool {
    display_path(left).eq_ignore_ascii_case(&display_path(right))
}

#[cfg(not(windows))]
fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
}

fn push_issue(
    issues: &mut Vec<FileCatalogIssue>,
    max_issues: usize,
    path: Option<String>,
    message: String,
) {
    if issues.len() < max_issues {
        issues.push(FileCatalogIssue { path, message });
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| display_path(path))
}

#[cfg(windows)]
fn display_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    path.strip_prefix(r"\\?\UNC\")
        .map(|path| format!(r"\\{path}"))
        .or_else(|| path.strip_prefix(r"\\?\").map(str::to_owned))
        .unwrap_or_else(|| path.into_owned())
}

#[cfg(not(windows))]
fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(windows)]
fn same_root(left: &str, right: &str) -> bool {
    left.replace('/', "\\")
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(right.replace('/', "\\").trim_end_matches('\\'))
}

#[cfg(not(windows))]
fn same_root(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn saturating_u64_to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn saturating_u128_to_i64(value: u128) -> i64 {
    value.min(i64::MAX as u128) as i64
}

fn index_error(error: rusqlite::Error) -> FileCatalogError {
    FileCatalogError::Index(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::tempdir;

    fn build(root: &Path, database: &Path) -> FileCatalogReport {
        build_file_catalog(
            root,
            database,
            FileCatalogConfig::default(),
            |_| {},
            || false,
        )
        .expect("build file catalog")
    }

    fn request(query: &str) -> FileCatalogSearchRequest {
        FileCatalogSearchRequest {
            query: query.to_owned(),
            kind: None,
            extensions: Vec::new(),
            min_bytes: None,
            max_bytes: None,
            sort: FileCatalogSort::Relevance,
            max_results: 100,
        }
    }

    #[test]
    fn indexes_files_directories_and_searches_korean_names_and_paths() {
        let temporary = tempdir().expect("tempdir");
        let root = temporary.path().join("root");
        let nested = root.join("회의자료");
        fs::create_dir_all(&nested).expect("create directories");
        fs::write(nested.join("분기보고서.txt"), b"metadata only").expect("write report");
        fs::write(root.join("notes.md"), b"notes").expect("write notes");
        let database = temporary.path().join("catalog.sqlite3");

        let report = build(&root, &database);
        assert_eq!(report.status.indexed_files, 2);
        assert_eq!(report.status.indexed_directories, 1);

        let by_name = search_file_catalog(&database, request("분기보고서")).expect("name search");
        assert_eq!(by_name.results.len(), 1);
        assert_eq!(
            by_name.results[0].match_source,
            FileCatalogMatchSource::Name
        );

        let mut by_path_request = request("회의자료");
        by_path_request.kind = Some(FileCatalogEntryKind::File);
        let by_path = search_file_catalog(&database, by_path_request).expect("path search");
        assert_eq!(by_path.results.len(), 1);
        assert_eq!(
            by_path.results[0].match_source,
            FileCatalogMatchSource::Path
        );
    }

    #[test]
    fn filters_extensions_sizes_and_sort_order() {
        let temporary = tempdir().expect("tempdir");
        let root = temporary.path().join("root");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("report-small.txt"), vec![1_u8; 8]).expect("write small");
        fs::write(root.join("report-large.log"), vec![2_u8; 64]).expect("write large");
        let database = temporary.path().join("catalog.sqlite3");
        build(&root, &database);

        let mut extension_request = request("report");
        extension_request.extensions = vec![".txt".to_owned()];
        let extension_result =
            search_file_catalog(&database, extension_request).expect("extension search");
        assert_eq!(extension_result.results.len(), 1);
        assert_eq!(extension_result.results[0].extension, "txt");

        let mut size_request = request("report");
        size_request.min_bytes = Some(16);
        size_request.sort = FileCatalogSort::Largest;
        let size_result = search_file_catalog(&database, size_request).expect("size search");
        assert_eq!(size_result.results.len(), 1);
        assert_eq!(size_result.results[0].name, "report-large.log");
    }

    #[test]
    fn refresh_removes_missing_entries_and_cancellation_preserves_last_catalog() {
        let temporary = tempdir().expect("tempdir");
        let root = temporary.path().join("root");
        fs::create_dir_all(&root).expect("create root");
        let first = root.join("first.txt");
        fs::write(&first, b"first").expect("write first");
        let database = temporary.path().join("catalog.sqlite3");
        let initial = build(&root, &database);

        fs::remove_file(&first).expect("remove first");
        fs::write(root.join("second.txt"), b"second").expect("write second");
        let cancelled = AtomicBool::new(true);
        let error = build_file_catalog(
            &root,
            &database,
            FileCatalogConfig::default(),
            |_| {},
            || cancelled.load(Ordering::Acquire),
        )
        .expect_err("cancel refresh");
        assert!(matches!(error, FileCatalogError::Cancelled));
        let status = file_catalog_status(&database)
            .expect("status after cancel")
            .expect("completed catalog");
        assert_eq!(
            status.completed_at_unix_ms,
            initial.status.completed_at_unix_ms
        );
        assert_eq!(
            search_file_catalog(&database, request("first"))
                .expect("old search")
                .results
                .len(),
            1
        );
        let connection = Connection::open(&database).expect("inspect rolled back catalog");
        let index_count = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name IN (
                    'file_catalog_generation_idx',
                    'file_catalog_kind_idx',
                    'file_catalog_extension_idx',
                    'file_catalog_size_idx',
                    'file_catalog_modified_idx',
                    'file_catalog_source_record_idx'
                 )",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count restored indexes");
        let trigger_count = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'trigger' AND name IN (
                    'file_catalog_ai', 'file_catalog_ad', 'file_catalog_au'
                 )",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count restored triggers");
        assert_eq!(index_count, 6);
        assert_eq!(trigger_count, 3);
        drop(connection);

        let refreshed = build(&root, &database);
        assert_eq!(refreshed.removed_entries, 1);
        assert!(
            search_file_catalog(&database, request("first"))
                .expect("removed search")
                .results
                .is_empty()
        );
        assert_eq!(
            search_file_catalog(&database, request("second"))
                .expect("new search")
                .results
                .len(),
            1
        );
    }

    #[test]
    fn entry_limit_is_explicit_and_catalog_can_be_cleared() {
        let temporary = tempdir().expect("tempdir");
        let root = temporary.path().join("root");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("one.txt"), b"one").expect("write one");
        fs::write(root.join("two.txt"), b"two").expect("write two");
        let database = temporary.path().join("catalog.sqlite3");

        let report = build_file_catalog(
            &root,
            &database,
            FileCatalogConfig {
                max_entries: 1,
                max_issues: 10,
            },
            |_| {},
            || false,
        )
        .expect("limited build");
        assert!(report.status.entry_limit_reached);
        assert_eq!(report.status.indexed_entries, 1);

        assert!(clear_file_catalog(&database).expect("clear catalog"));
        assert!(
            file_catalog_status(&database)
                .expect("empty status")
                .is_none()
        );
        assert!(matches!(
            search_file_catalog(&database, request("one")),
            Err(FileCatalogError::IndexUnavailable)
        ));
    }

    #[test]
    fn migrates_the_v1_catalog_without_losing_completed_status() {
        let temporary = tempdir().expect("tempdir");
        let database = temporary.path().join("catalog.sqlite3");
        let connection = Connection::open(&database).expect("open v1 database");
        connection
            .execute_batch(
                "CREATE TABLE file_catalog_meta (
                    id INTEGER PRIMARY KEY,
                    schema_version INTEGER NOT NULL,
                    root TEXT NOT NULL,
                    completed_at_ms INTEGER NOT NULL,
                    duration_ms INTEGER NOT NULL,
                    generation INTEGER NOT NULL,
                    indexed_entries INTEGER NOT NULL,
                    indexed_files INTEGER NOT NULL,
                    indexed_directories INTEGER NOT NULL,
                    indexed_symlinks INTEGER NOT NULL,
                    indexed_bytes INTEGER NOT NULL,
                    unreadable_entries INTEGER NOT NULL,
                    entry_limit_reached INTEGER NOT NULL,
                    provider TEXT NOT NULL
                 );
                 CREATE TABLE file_catalog_entries (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    parent TEXT NOT NULL,
                    extension TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    logical_bytes INTEGER NOT NULL,
                    modified_at_ms INTEGER,
                    generation INTEGER NOT NULL
                 );
                 INSERT INTO file_catalog_meta VALUES (
                    1, 1, 'C:\\fixture', 1000, 20, 1, 0, 0, 0, 0, 0, 0, 0, 'portableWalk'
                 );
                 PRAGMA user_version = 1;",
            )
            .expect("create v1 schema");
        drop(connection);

        let status = file_catalog_status(&database)
            .expect("migrate status")
            .expect("completed status");
        assert_eq!(status.provider, FileCatalogProvider::PortableWalk);
        assert_eq!(status.refresh_mode, FileCatalogRefreshMode::Full);
        let connection = Connection::open(&database).expect("reopen migrated database");
        let version = connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .expect("read schema version");
        assert_eq!(version, INDEX_SCHEMA_VERSION);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires an elevated Windows process and reads the current NTFS volume"]
    fn real_ntfs_provider_builds_a_searchable_bounded_catalog() {
        let current = std::env::current_dir().expect("current directory");
        let current_display = display_path(&current);
        let drive = current_display
            .chars()
            .next()
            .expect("current drive letter")
            .to_ascii_uppercase();
        let root = PathBuf::from(format!("{drive}:\\"));
        let temporary = tempdir().expect("tempdir");
        let database = temporary.path().join("ntfs-catalog.sqlite3");

        let report = build_file_catalog(
            &root,
            &database,
            FileCatalogConfig {
                max_entries: 1_000,
                max_issues: 20,
            },
            |_| {},
            || false,
        )
        .expect("build real NTFS catalog");
        assert_eq!(report.status.provider, FileCatalogProvider::WindowsNtfs);
        assert_eq!(report.status.refresh_mode, FileCatalogRefreshMode::Full);
        assert_eq!(report.status.indexed_entries, 1_000);
        assert!(report.status.entry_limit_reached);
        assert!(
            !search_file_catalog(&database, request("$MFT"))
                .expect("search MFT metadata file")
                .results
                .is_empty()
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires elevation, free temporary space, and an idle writable NTFS volume"]
    fn real_ntfs_catalog_applies_a_new_file_from_the_usn_journal() {
        let mut candidates = ('C'..='Z')
            .filter_map(|drive| {
                let root = PathBuf::from(format!("{drive}:\\"));
                match windows_ntfs::NtfsSource::try_open(&root) {
                    windows_ntfs::NtfsAvailability::Ready(source) => {
                        Some((source.record_count(), root))
                    }
                    windows_ntfs::NtfsAvailability::Unavailable(_) => None,
                }
            })
            .filter(|(records, _)| *records <= MAX_ENTRIES as u64)
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(records, _)| *records);
        let root = candidates
            .into_iter()
            .find_map(|(_, root)| {
                tempfile::Builder::new()
                    .prefix("bloomsweepy-usn-probe-")
                    .suffix(".tmp")
                    .tempfile_in(&root)
                    .ok()
                    .map(|probe| {
                        drop(probe);
                        root
                    })
            })
            .expect("writable NTFS volume with a bounded MFT");
        let temporary = tempdir().expect("tempdir");
        let database = temporary.path().join("ntfs-usn-catalog.sqlite3");

        let initial = build_file_catalog(
            &root,
            &database,
            FileCatalogConfig {
                max_entries: MAX_ENTRIES,
                max_issues: 20,
            },
            |_| {},
            || false,
        )
        .expect("initial full MFT catalog");
        assert_eq!(initial.status.provider, FileCatalogProvider::WindowsNtfs);
        assert!(!initial.status.entry_limit_reached);

        let mut changed_file = tempfile::Builder::new()
            .prefix("bloomsweepy-usn-live-")
            .suffix(".txt")
            .tempfile_in(&root)
            .expect("create live USN fixture");
        changed_file
            .write_all(b"USN incremental fixture")
            .expect("write live fixture");
        changed_file.flush().expect("flush live fixture");
        let live_name = changed_file
            .path()
            .file_name()
            .expect("live fixture name")
            .to_string_lossy()
            .into_owned();

        let refreshed = build_file_catalog(
            &root,
            &database,
            FileCatalogConfig {
                max_entries: MAX_ENTRIES,
                max_issues: 20,
            },
            |_| {},
            || false,
        )
        .expect("apply USN delta");
        assert_eq!(
            refreshed.status.refresh_mode,
            FileCatalogRefreshMode::Incremental
        );
        assert!(
            !search_file_catalog(&database, request(&live_name))
                .expect("search USN-created file")
                .results
                .is_empty()
        );
    }
}
