use encoding_rs::{EUC_KR, WINDOWS_1252};
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, params_from_iter};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use zip::ZipArchive;

use crate::{bounded_worker_threads, open_read_shared, system_time_ms};

const INDEX_SCHEMA_VERSION: i64 = 1;
const DEFAULT_MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_MAX_EXTRACTED_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_MAX_DOCUMENTS: usize = 100_000;
const DEFAULT_MAX_ISSUES: usize = 100;
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: usize = 16 * 1024 * 1024;
const MAX_DOCUMENTS: usize = 500_000;
const MAX_ISSUES: usize = 1_000;
const MAX_SEARCH_RESULTS: usize = 250;
const MAX_QUERY_CHARS: usize = 256;
const READ_CHUNK_BYTES: usize = 256 * 1024;
const PROGRESS_FILE_INTERVAL: u64 = 256;
const SNIPPET_OPEN: char = '\u{e000}';
const SNIPPET_CLOSE: char = '\u{e001}';

const PLAIN_TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "rst", "log", "csv", "tsv", "json", "jsonl", "xml", "yaml", "yml",
    "toml", "ini", "cfg", "conf", "sql", "html", "htm", "css", "js", "jsx", "ts", "tsx", "py",
    "rs", "go", "java", "c", "h", "cpp", "hpp", "cs", "swift", "kt", "kts", "sh", "ps1", "bat",
    "cmd",
];

const ARCHIVE_ENTRY_MULTIPLIER: usize = 2;
const ARCHIVE_TOTAL_MULTIPLIER: usize = 8;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DocumentIndexConfig {
    pub max_file_bytes: u64,
    pub max_extracted_bytes: usize,
    pub max_documents: usize,
    pub max_issues: usize,
}

impl Default for DocumentIndexConfig {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_extracted_bytes: DEFAULT_MAX_EXTRACTED_BYTES,
            max_documents: DEFAULT_MAX_DOCUMENTS,
            max_issues: DEFAULT_MAX_ISSUES,
        }
    }
}

impl DocumentIndexConfig {
    fn bounded(mut self) -> Self {
        self.max_file_bytes = self.max_file_bytes.clamp(1, MAX_FILE_BYTES);
        self.max_extracted_bytes = self.max_extracted_bytes.clamp(1, MAX_EXTRACTED_BYTES);
        self.max_documents = self.max_documents.min(MAX_DOCUMENTS);
        self.max_issues = self.max_issues.min(MAX_ISSUES);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentIndexPhase {
    Discovering,
    Indexing,
    Finalizing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIndexProgress {
    pub phase: DocumentIndexPhase,
    pub message: String,
    pub scanned_files: u64,
    pub candidate_documents: u64,
    pub indexed_documents: u64,
    pub reused_documents: u64,
    pub processed_bytes: u64,
    pub skipped_documents: u64,
    pub unreadable_entries: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIndexStatus {
    pub root: String,
    pub completed_at_unix_ms: u128,
    pub duration_ms: u128,
    pub indexed_documents: u64,
    pub indexed_bytes: u64,
    pub supported_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIndexReport {
    #[serde(flatten)]
    pub status: DocumentIndexStatus,
    pub scanned_files: u64,
    pub candidate_documents: u64,
    pub updated_documents: u64,
    pub reused_documents: u64,
    pub removed_documents: u64,
    pub skipped_documents: u64,
    pub unsupported_documents: u64,
    pub unreadable_entries: u64,
    pub document_limit_reached: bool,
    pub issues: Vec<DocumentIndexIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIndexIssue {
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSearchRequest {
    pub query: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default = "default_search_results")]
    pub max_results: usize,
}

fn default_search_results() -> usize {
    100
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSearchReport {
    pub root: String,
    pub query: String,
    pub searched_documents: u64,
    pub total_matches: u64,
    pub results_truncated: bool,
    pub results: Vec<DocumentSearchResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSearchResult {
    pub name: String,
    pub path: String,
    pub extension: String,
    pub format: DocumentFormat,
    pub logical_bytes: u64,
    pub modified_at_unix_ms: Option<u128>,
    pub match_source: DocumentMatchSource,
    pub snippet: Vec<DocumentSnippetPart>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSnippetPart {
    pub text: String,
    pub highlighted: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DocumentFormat {
    PlainText,
    Pdf,
    Word,
    Spreadsheet,
    Presentation,
    Hwpx,
}

impl DocumentFormat {
    fn database_value(self) -> &'static str {
        match self {
            Self::PlainText => "plainText",
            Self::Pdf => "pdf",
            Self::Word => "word",
            Self::Spreadsheet => "spreadsheet",
            Self::Presentation => "presentation",
            Self::Hwpx => "hwpx",
        }
    }

    fn from_database(value: &str) -> Self {
        match value {
            "pdf" => Self::Pdf,
            "word" => Self::Word,
            "spreadsheet" => Self::Spreadsheet,
            "presentation" => Self::Presentation,
            "hwpx" => Self::Hwpx,
            _ => Self::PlainText,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DocumentMatchSource {
    Content,
    Name,
    Path,
}

#[derive(Debug, Error)]
pub enum DocumentSearchError {
    #[error("document search path does not exist: {0}")]
    MissingPath(String),
    #[error("document search path is not a directory: {0}")]
    NotDirectory(String),
    #[error("document indexing was cancelled")]
    Cancelled,
    #[error("enter at least one search character")]
    EmptyQuery,
    #[error("search query is longer than {MAX_QUERY_CHARS} characters")]
    QueryTooLong,
    #[error("no completed document index is available")]
    IndexUnavailable,
    #[error("failed to access the document index: {0}")]
    Index(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateKind {
    Supported(DocumentFormat),
    UnsupportedHwp,
    Other,
}

#[derive(Debug)]
enum ExtractionError {
    Cancelled,
    Unreadable(String),
    NoText(String),
}

impl ExtractionError {
    fn message(&self) -> Option<&str> {
        match self {
            Self::Cancelled => None,
            Self::Unreadable(message) | Self::NoText(message) => Some(message),
        }
    }

    fn is_unreadable(&self) -> bool {
        matches!(self, Self::Unreadable(_))
    }
}

pub fn build_document_index<F, C>(
    root: impl AsRef<Path>,
    database_path: impl AsRef<Path>,
    config: DocumentIndexConfig,
    mut on_progress: F,
    should_cancel: C,
) -> Result<DocumentIndexReport, DocumentSearchError>
where
    F: FnMut(DocumentIndexProgress),
    C: Fn() -> bool + Sync,
{
    let started = Instant::now();
    let config = config.bounded();
    let requested_root = root.as_ref();
    if !requested_root.exists() {
        return Err(DocumentSearchError::MissingPath(
            requested_root.to_string_lossy().into_owned(),
        ));
    }
    if !requested_root.is_dir() {
        return Err(DocumentSearchError::NotDirectory(
            requested_root.to_string_lossy().into_owned(),
        ));
    }

    let root = requested_root
        .canonicalize()
        .map_err(|error| DocumentSearchError::Index(error.to_string()))?;
    let root_string = display_path(&root);
    let mut connection = open_index(database_path.as_ref())?;
    initialize_schema(&connection)?;

    on_progress(DocumentIndexProgress {
        phase: DocumentIndexPhase::Discovering,
        message: "검색할 문서와 기존 색인을 대조하고 있습니다".to_owned(),
        scanned_files: 0,
        candidate_documents: 0,
        indexed_documents: 0,
        reused_documents: 0,
        processed_bytes: 0,
        skipped_documents: 0,
        unreadable_entries: 0,
    });

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(index_error)?;
    let previous = read_meta(&transaction)?;
    if previous
        .as_ref()
        .is_some_and(|meta| !same_root(&meta.root, &root_string))
    {
        transaction
            .execute("DELETE FROM documents", [])
            .map_err(index_error)?;
    }
    let generation = previous
        .as_ref()
        .map_or(1_i64, |meta| meta.generation.saturating_add(1));

    let mut scanned_files = 0_u64;
    let mut candidate_documents = 0_u64;
    let mut indexed_documents = 0_u64;
    let mut updated_documents = 0_u64;
    let mut reused_documents = 0_u64;
    let mut processed_bytes = 0_u64;
    let mut skipped_documents = 0_u64;
    let mut unsupported_documents = 0_u64;
    let mut unreadable_entries = 0_u64;
    let mut document_limit_reached = false;
    let mut issues = Vec::new();
    let mut last_detailed_progress = Instant::now();

    for item in jwalk::WalkDir::new(&root)
        .follow_links(false)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(bounded_worker_threads()))
    {
        if should_cancel() {
            return Err(DocumentSearchError::Cancelled);
        }

        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                unreadable_entries = unreadable_entries.saturating_add(1);
                push_issue(&mut issues, config.max_issues, None, error.to_string());
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        scanned_files = scanned_files.saturating_add(1);
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_lowercase())
            .unwrap_or_default();
        let format = match candidate_kind(&extension) {
            CandidateKind::Other => {
                emit_periodic_progress(
                    &mut on_progress,
                    DocumentIndexProgress {
                        phase: DocumentIndexPhase::Discovering,
                        message: "문서 파일과 변경 시각을 확인하고 있습니다".to_owned(),
                        scanned_files,
                        candidate_documents,
                        indexed_documents,
                        reused_documents,
                        processed_bytes,
                        skipped_documents,
                        unreadable_entries,
                    },
                );
                continue;
            }
            CandidateKind::UnsupportedHwp => {
                candidate_documents = candidate_documents.saturating_add(1);
                unsupported_documents = unsupported_documents.saturating_add(1);
                skipped_documents = skipped_documents.saturating_add(1);
                emit_periodic_progress(
                    &mut on_progress,
                    DocumentIndexProgress {
                        phase: DocumentIndexPhase::Discovering,
                        message: "문서 파일과 변경 시각을 확인하고 있습니다".to_owned(),
                        scanned_files,
                        candidate_documents,
                        indexed_documents,
                        reused_documents,
                        processed_bytes,
                        skipped_documents,
                        unreadable_entries,
                    },
                );
                continue;
            }
            CandidateKind::Supported(format) => format,
        };
        candidate_documents = candidate_documents.saturating_add(1);

        let Some(_) = path.to_str() else {
            skipped_documents = skipped_documents.saturating_add(1);
            push_issue(
                &mut issues,
                config.max_issues,
                None,
                "운영체제 문자열로 표현할 수 없는 문서 경로를 건너뛰었습니다".to_owned(),
            );
            continue;
        };
        let path_string = display_path(&path);
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                unreadable_entries = unreadable_entries.saturating_add(1);
                skipped_documents = skipped_documents.saturating_add(1);
                push_issue(
                    &mut issues,
                    config.max_issues,
                    Some(path_string),
                    error.to_string(),
                );
                continue;
            }
        };
        let logical_bytes = metadata.len();
        processed_bytes = processed_bytes.saturating_add(logical_bytes);
        let modified_at_unix_ms = system_time_ms(metadata.modified().ok());
        let modified_at_i64 = modified_at_unix_ms.map(saturating_u128_to_i64);

        if indexed_documents as usize >= config.max_documents {
            document_limit_reached = true;
            skipped_documents = skipped_documents.saturating_add(1);
            continue;
        }
        if logical_bytes > config.max_file_bytes {
            skipped_documents = skipped_documents.saturating_add(1);
            continue;
        }

        let existing = transaction
            .query_row(
                "SELECT logical_bytes, modified_at_ms, format FROM documents WHERE path = ?1",
                [&path_string],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(index_error)?;
        if existing
            .as_ref()
            .is_some_and(|(bytes, modified, stored_format)| {
                *bytes == saturating_u64_to_i64(logical_bytes)
                    && *modified == modified_at_i64
                    && stored_format == format.database_value()
            })
        {
            transaction
                .execute(
                    "UPDATE documents SET generation = ?1 WHERE path = ?2",
                    params![generation, path_string],
                )
                .map_err(index_error)?;
            indexed_documents = indexed_documents.saturating_add(1);
            reused_documents = reused_documents.saturating_add(1);
            emit_periodic_progress(
                &mut on_progress,
                DocumentIndexProgress {
                    phase: DocumentIndexPhase::Discovering,
                    message: "문서 파일과 변경 시각을 확인하고 있습니다".to_owned(),
                    scanned_files,
                    candidate_documents,
                    indexed_documents,
                    reused_documents,
                    processed_bytes,
                    skipped_documents,
                    unreadable_entries,
                },
            );
            continue;
        }

        if last_detailed_progress.elapsed() >= Duration::from_millis(100) {
            on_progress(DocumentIndexProgress {
                phase: DocumentIndexPhase::Indexing,
                message: format!("{} 내용을 읽고 있습니다", display_name(&path)),
                scanned_files,
                candidate_documents,
                indexed_documents,
                reused_documents,
                processed_bytes,
                skipped_documents,
                unreadable_entries,
            });
            last_detailed_progress = Instant::now();
        }
        let content = match extract_document(&path, format, &config, &should_cancel) {
            Ok(content) => content,
            Err(ExtractionError::Cancelled) => return Err(DocumentSearchError::Cancelled),
            Err(error) => {
                if error.is_unreadable() {
                    unreadable_entries = unreadable_entries.saturating_add(1);
                }
                skipped_documents = skipped_documents.saturating_add(1);
                if let Some(message) = error.message() {
                    push_issue(
                        &mut issues,
                        config.max_issues,
                        Some(path_string),
                        message.to_owned(),
                    );
                }
                continue;
            }
        };

        let name = display_name(&path);
        transaction
            .execute(
                "INSERT INTO documents (
                    path, name, extension, format, logical_bytes, modified_at_ms, content, generation
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(path) DO UPDATE SET
                    name = excluded.name,
                    extension = excluded.extension,
                    format = excluded.format,
                    logical_bytes = excluded.logical_bytes,
                    modified_at_ms = excluded.modified_at_ms,
                    content = excluded.content,
                    generation = excluded.generation",
                params![
                    path_string,
                    name,
                    extension,
                    format.database_value(),
                    saturating_u64_to_i64(logical_bytes),
                    modified_at_i64,
                    content,
                    generation,
                ],
            )
            .map_err(index_error)?;
        indexed_documents = indexed_documents.saturating_add(1);
        updated_documents = updated_documents.saturating_add(1);
    }

    if should_cancel() {
        return Err(DocumentSearchError::Cancelled);
    }
    on_progress(DocumentIndexProgress {
        phase: DocumentIndexPhase::Finalizing,
        message: "변경되거나 사라진 문서를 정리하고 있습니다".to_owned(),
        scanned_files,
        candidate_documents,
        indexed_documents,
        reused_documents,
        processed_bytes,
        skipped_documents,
        unreadable_entries,
    });

    let removed_documents = transaction
        .execute("DELETE FROM documents WHERE generation <> ?1", [generation])
        .map_err(index_error)? as u64;
    let (indexed_documents, indexed_bytes) = transaction
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(logical_bytes), 0) FROM documents",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(index_error)?;
    let completed_at_unix_ms = unix_time_ms();
    let duration_ms = started.elapsed().as_millis();
    transaction
        .execute(
            "INSERT INTO index_meta (
                id, schema_version, root, completed_at_ms, duration_ms, generation
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                schema_version = excluded.schema_version,
                root = excluded.root,
                completed_at_ms = excluded.completed_at_ms,
                duration_ms = excluded.duration_ms,
                generation = excluded.generation",
            params![
                INDEX_SCHEMA_VERSION,
                root_string,
                saturating_u128_to_i64(completed_at_unix_ms),
                saturating_u128_to_i64(duration_ms),
                generation,
            ],
        )
        .map_err(index_error)?;
    transaction.commit().map_err(index_error)?;
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(index_error)?;

    Ok(DocumentIndexReport {
        status: DocumentIndexStatus {
            root: root_string,
            completed_at_unix_ms,
            duration_ms,
            indexed_documents: indexed_documents.max(0) as u64,
            indexed_bytes: indexed_bytes.max(0) as u64,
            supported_extensions: supported_extensions(),
        },
        scanned_files,
        candidate_documents,
        updated_documents,
        reused_documents,
        removed_documents,
        skipped_documents,
        unsupported_documents,
        unreadable_entries,
        document_limit_reached,
        issues,
    })
}

pub fn document_index_status(
    database_path: impl AsRef<Path>,
) -> Result<Option<DocumentIndexStatus>, DocumentSearchError> {
    let database_path = database_path.as_ref();
    if !database_path.exists() {
        return Ok(None);
    }
    let connection = open_index(database_path)?;
    initialize_schema(&connection)?;
    let Some(meta) = read_meta(&connection)? else {
        return Ok(None);
    };
    let (documents, bytes) = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(logical_bytes), 0) FROM documents",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(index_error)?;
    Ok(Some(DocumentIndexStatus {
        root: meta.root,
        completed_at_unix_ms: meta.completed_at_ms.max(0) as u128,
        duration_ms: meta.duration_ms.max(0) as u128,
        indexed_documents: documents.max(0) as u64,
        indexed_bytes: bytes.max(0) as u64,
        supported_extensions: supported_extensions(),
    }))
}

pub fn search_document_index(
    database_path: impl AsRef<Path>,
    request: DocumentSearchRequest,
) -> Result<DocumentSearchReport, DocumentSearchError> {
    let query = request.query.trim();
    if query.is_empty() {
        return Err(DocumentSearchError::EmptyQuery);
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(DocumentSearchError::QueryTooLong);
    }

    let database_path = database_path.as_ref();
    if !database_path.exists() {
        return Err(DocumentSearchError::IndexUnavailable);
    }
    let connection = open_index(database_path)?;
    initialize_schema(&connection)?;
    let meta = read_meta(&connection)?.ok_or(DocumentSearchError::IndexUnavailable)?;
    let searched_documents = connection
        .query_row("SELECT COUNT(*) FROM documents", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(index_error)?
        .max(0) as u64;
    let extensions = normalized_extensions(request.extensions);
    let max_results = request.max_results.clamp(1, MAX_SEARCH_RESULTS);
    let use_trigram = query.chars().count() >= 3;

    let mut values = Vec::new();
    let base_predicate = if use_trigram {
        values.push(Value::Text(fts_literal(query)));
        "documents_fts MATCH ?1".to_owned()
    } else {
        values.push(Value::Text(format!("%{}%", escape_like(query))));
        "(d.name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
          OR d.path LIKE ?1 ESCAPE '\\' COLLATE NOCASE
          OR d.content LIKE ?1 ESCAPE '\\' COLLATE NOCASE)"
            .to_owned()
    };
    let extension_clause = extension_clause(&extensions, values.len() + 1);
    for extension in &extensions {
        values.push(Value::Text(extension.clone()));
    }

    let from_clause = if use_trigram {
        "FROM documents_fts JOIN documents d ON d.id = documents_fts.rowid"
    } else {
        "FROM documents d"
    };
    let count_sql =
        format!("SELECT COUNT(*) {from_clause} WHERE {base_predicate}{extension_clause}");
    let total_matches = connection
        .query_row(&count_sql, params_from_iter(values.iter()), |row| {
            row.get::<_, i64>(0)
        })
        .map_err(index_error)?
        .max(0) as u64;

    let snippet_expression = if use_trigram {
        format!("snippet(documents_fts, 2, '{SNIPPET_OPEN}', '{SNIPPET_CLOSE}', ' … ', 28)")
    } else {
        "d.content".to_owned()
    };
    let order_clause = if use_trigram {
        "ORDER BY bm25(documents_fts, 3.0, 1.2, 1.0), d.modified_at_ms DESC"
    } else {
        "ORDER BY d.modified_at_ms DESC, d.name COLLATE NOCASE"
    };
    let limit_parameter = values.len() + 1;
    let result_sql = format!(
        "SELECT d.name, d.path, d.extension, d.format, d.logical_bytes,
                d.modified_at_ms, {snippet_expression}
         {from_clause}
         WHERE {base_predicate}{extension_clause}
         {order_clause}
         LIMIT ?{limit_parameter}"
    );
    values.push(Value::Integer(max_results as i64));

    let mut statement = connection.prepare(&result_sql).map_err(index_error)?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(index_error)?;
    let mut results = Vec::new();
    for row in rows {
        let (name, path, extension, format, bytes, modified_at, raw_snippet) =
            row.map_err(index_error)?;
        let name_matches = contains_case_insensitive(&name, query);
        let path_matches = contains_case_insensitive(&path, query);
        let mut snippet = if use_trigram {
            parse_marked_snippet(&raw_snippet)
        } else {
            direct_snippet(&raw_snippet, query)
        };
        let content_matches = snippet.iter().any(|part| part.highlighted);
        let match_source = if content_matches {
            DocumentMatchSource::Content
        } else if name_matches {
            DocumentMatchSource::Name
        } else if path_matches {
            DocumentMatchSource::Path
        } else {
            DocumentMatchSource::Content
        };
        if snippet.is_empty() || !content_matches {
            snippet = fallback_snippet(match_source, &name, &path, query);
        }
        results.push(DocumentSearchResult {
            name,
            path,
            extension,
            format: DocumentFormat::from_database(&format),
            logical_bytes: bytes.max(0) as u64,
            modified_at_unix_ms: modified_at.map(|value| value.max(0) as u128),
            match_source,
            snippet,
        });
    }

    Ok(DocumentSearchReport {
        root: meta.root,
        query: query.to_owned(),
        searched_documents,
        total_matches,
        results_truncated: total_matches > results.len() as u64,
        results,
    })
}

fn open_index(path: &Path) -> Result<Connection, DocumentSearchError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| DocumentSearchError::Index(error.to_string()))?;
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

fn initialize_schema(connection: &Connection) -> Result<(), DocumentSearchError> {
    let schema_version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(index_error)?;
    if schema_version != 0 && schema_version != INDEX_SCHEMA_VERSION {
        return Err(DocumentSearchError::Index(format!(
            "unsupported index schema version {schema_version}"
        )));
    }
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS index_meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                schema_version INTEGER NOT NULL,
                root TEXT NOT NULL,
                completed_at_ms INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                generation INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                extension TEXT NOT NULL,
                format TEXT NOT NULL,
                logical_bytes INTEGER NOT NULL,
                modified_at_ms INTEGER,
                content TEXT NOT NULL,
                generation INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS documents_generation_idx ON documents(generation);
             CREATE INDEX IF NOT EXISTS documents_extension_idx ON documents(extension);
             CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                name,
                path,
                content,
                content = 'documents',
                content_rowid = 'id',
                tokenize = 'trigram'
             );
             CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO documents_fts(rowid, name, path, content)
                VALUES (new.id, new.name, new.path, new.content);
             END;
             CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, name, path, content)
                VALUES ('delete', old.id, old.name, old.path, old.content);
             END;
             CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE OF name, path, content ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, name, path, content)
                VALUES ('delete', old.id, old.name, old.path, old.content);
                INSERT INTO documents_fts(rowid, name, path, content)
                VALUES (new.id, new.name, new.path, new.content);
             END;
             PRAGMA user_version = 1;",
        )
        .map_err(index_error)
}

#[derive(Debug)]
struct IndexMeta {
    root: String,
    completed_at_ms: i64,
    duration_ms: i64,
    generation: i64,
}

fn read_meta(connection: &Connection) -> Result<Option<IndexMeta>, DocumentSearchError> {
    connection
        .query_row(
            "SELECT root, completed_at_ms, duration_ms, generation
             FROM index_meta WHERE id = 1 AND schema_version = ?1",
            [INDEX_SCHEMA_VERSION],
            |row| {
                Ok(IndexMeta {
                    root: row.get(0)?,
                    completed_at_ms: row.get(1)?,
                    duration_ms: row.get(2)?,
                    generation: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(index_error)
}

fn candidate_kind(extension: &str) -> CandidateKind {
    if PLAIN_TEXT_EXTENSIONS.contains(&extension) {
        return CandidateKind::Supported(DocumentFormat::PlainText);
    }
    match extension {
        "pdf" => CandidateKind::Supported(DocumentFormat::Pdf),
        "docx" => CandidateKind::Supported(DocumentFormat::Word),
        "xlsx" => CandidateKind::Supported(DocumentFormat::Spreadsheet),
        "pptx" => CandidateKind::Supported(DocumentFormat::Presentation),
        "hwpx" => CandidateKind::Supported(DocumentFormat::Hwpx),
        "hwp" => CandidateKind::UnsupportedHwp,
        _ => CandidateKind::Other,
    }
}

fn extract_document<C>(
    path: &Path,
    format: DocumentFormat,
    config: &DocumentIndexConfig,
    should_cancel: &C,
) -> Result<String, ExtractionError>
where
    C: Fn() -> bool,
{
    let text = match format {
        DocumentFormat::PlainText => {
            let bytes = read_file_limited(path, config.max_file_bytes, should_cancel)?;
            decode_text(&bytes)?
        }
        DocumentFormat::Pdf => {
            let bytes = read_file_limited(path, config.max_file_bytes, should_cancel)?;
            if should_cancel() {
                return Err(ExtractionError::Cancelled);
            }
            pdf_extract::extract_text_from_mem(&bytes).map_err(|error| {
                ExtractionError::Unreadable(format!("PDF 텍스트를 읽지 못했습니다: {error}"))
            })?
        }
        DocumentFormat::Word
        | DocumentFormat::Spreadsheet
        | DocumentFormat::Presentation
        | DocumentFormat::Hwpx => extract_archive_text(path, format, config, should_cancel)?,
    };
    let normalized = normalize_text(&text, config.max_extracted_bytes);
    if normalized.trim().is_empty() {
        let message = if format == DocumentFormat::Pdf {
            "텍스트 계층이 없는 PDF입니다. OCR은 아직 수행하지 않습니다"
        } else {
            "검색할 수 있는 텍스트가 없습니다"
        };
        return Err(ExtractionError::NoText(message.to_owned()));
    }
    Ok(normalized)
}

fn read_file_limited<C>(
    path: &Path,
    max_bytes: u64,
    should_cancel: &C,
) -> Result<Vec<u8>, ExtractionError>
where
    C: Fn() -> bool,
{
    let mut file =
        open_read_shared(path).map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
    let mut output = Vec::with_capacity((max_bytes as usize).min(READ_CHUNK_BYTES));
    let mut buffer = vec![0_u8; READ_CHUNK_BYTES];
    loop {
        if should_cancel() {
            return Err(ExtractionError::Cancelled);
        }
        let read = file
            .read(&mut buffer)
            .map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) > max_bytes as usize {
            return Err(ExtractionError::Unreadable(
                "문서가 설정된 읽기 상한을 초과했습니다".to_owned(),
            ));
        }
        output.extend_from_slice(&buffer[..read]);
    }
    Ok(output)
}

fn decode_text(bytes: &[u8]) -> Result<String, ExtractionError> {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(bytes.to_vec())
            .map_err(|error| ExtractionError::Unreadable(error.to_string()));
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        return decode_utf16(bytes, true);
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xfe, 0xff]) {
        return decode_utf16(bytes, false);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.to_owned());
    }

    let even_zeroes = bytes.iter().step_by(2).filter(|byte| **byte == 0).count();
    let odd_zeroes = bytes
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|byte| **byte == 0)
        .count();
    let pairs = bytes.len() / 2;
    if pairs > 0 && odd_zeroes > pairs / 3 {
        return decode_utf16(bytes, true);
    }
    if pairs > 0 && even_zeroes > pairs / 3 {
        return decode_utf16(bytes, false);
    }

    let (korean, _, korean_had_errors) = EUC_KR.decode(bytes);
    if !korean_had_errors && korean.chars().any(is_hangul) {
        return Ok(korean.into_owned());
    }
    let null_bytes = bytes.iter().filter(|byte| **byte == 0).count();
    if null_bytes <= bytes.len() / 100 {
        let (western, _, western_had_errors) = WINDOWS_1252.decode(bytes);
        if !western_had_errors {
            return Ok(western.into_owned());
        }
    }
    Err(ExtractionError::Unreadable(
        "UTF-8, UTF-16, CP949 또는 Windows ANSI 텍스트로 해석할 수 없습니다".to_owned(),
    ))
}

fn is_hangul(character: char) -> bool {
    matches!(character as u32, 0x1100..=0x11ff | 0x3130..=0x318f | 0xac00..=0xd7af)
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Result<String, ExtractionError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(ExtractionError::Unreadable(
            "손상된 UTF-16 텍스트입니다".to_owned(),
        ));
    }
    let (pairs, remainder) = bytes.as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    let units = pairs.iter().map(|pair| {
        if little_endian {
            u16::from_le_bytes([pair[0], pair[1]])
        } else {
            u16::from_be_bytes([pair[0], pair[1]])
        }
    });
    String::from_utf16(&units.collect::<Vec<_>>())
        .map_err(|error| ExtractionError::Unreadable(error.to_string()))
}

fn extract_archive_text<C>(
    path: &Path,
    format: DocumentFormat,
    config: &DocumentIndexConfig,
    should_cancel: &C,
) -> Result<String, ExtractionError>
where
    C: Fn() -> bool,
{
    let file =
        open_read_shared(path).map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        ExtractionError::Unreadable(format!("문서 압축 구조를 읽지 못했습니다: {error}"))
    })?;
    let entry_limit = config
        .max_extracted_bytes
        .saturating_mul(ARCHIVE_ENTRY_MULTIPLIER)
        .max(64 * 1024);
    let total_limit = config
        .max_extracted_bytes
        .saturating_mul(ARCHIVE_TOTAL_MULTIPLIER)
        .max(256 * 1024);
    let mut total_xml_bytes = 0_usize;
    let mut output = String::new();

    for index in 0..archive.len() {
        if should_cancel() {
            return Err(ExtractionError::Cancelled);
        }
        let mut entry = archive.by_index(index).map_err(|error| {
            ExtractionError::Unreadable(format!("문서 내부 항목을 읽지 못했습니다: {error}"))
        })?;
        let name = entry.name().replace('\\', "/").to_lowercase();
        if entry.encrypted() || !archive_entry_is_searchable(format, &name) {
            continue;
        }
        let entry_size = entry.size() as usize;
        if entry_size > entry_limit || total_xml_bytes.saturating_add(entry_size) > total_limit {
            continue;
        }
        let bytes = read_stream_limited(&mut entry, entry_limit, should_cancel)?;
        total_xml_bytes = total_xml_bytes.saturating_add(bytes.len());
        let xml = decode_text(&bytes)?;
        append_xml_text(&xml, format, &mut output, config.max_extracted_bytes)?;
        if output.len() >= config.max_extracted_bytes {
            break;
        }
    }
    Ok(output)
}

fn read_stream_limited<R, C>(
    reader: &mut R,
    max_bytes: usize,
    should_cancel: &C,
) -> Result<Vec<u8>, ExtractionError>
where
    R: Read,
    C: Fn() -> bool,
{
    let mut output = Vec::with_capacity(max_bytes.min(READ_CHUNK_BYTES));
    let mut buffer = vec![0_u8; READ_CHUNK_BYTES.min(max_bytes.max(1))];
    loop {
        if should_cancel() {
            return Err(ExtractionError::Cancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) > max_bytes {
            return Err(ExtractionError::Unreadable(
                "문서 내부 항목이 압축 해제 상한을 초과했습니다".to_owned(),
            ));
        }
        output.extend_from_slice(&buffer[..read]);
    }
    Ok(output)
}

fn archive_entry_is_searchable(format: DocumentFormat, name: &str) -> bool {
    match format {
        DocumentFormat::Word => {
            name == "word/document.xml"
                || name.starts_with("word/header") && name.ends_with(".xml")
                || name.starts_with("word/footer") && name.ends_with(".xml")
                || matches!(
                    name,
                    "word/footnotes.xml"
                        | "word/endnotes.xml"
                        | "word/comments.xml"
                        | "word/glossary/document.xml"
                )
        }
        DocumentFormat::Spreadsheet => {
            name == "xl/sharedstrings.xml"
                || name.starts_with("xl/worksheets/") && name.ends_with(".xml")
        }
        DocumentFormat::Presentation => {
            (name.starts_with("ppt/slides/") || name.starts_with("ppt/notesslides/"))
                && name.ends_with(".xml")
        }
        DocumentFormat::Hwpx => {
            name.starts_with("contents/section") && name.ends_with(".xml")
                || name == "preview/prvtext.txt"
        }
        _ => false,
    }
}

fn append_xml_text(
    xml: &str,
    format: DocumentFormat,
    output: &mut String,
    max_bytes: usize,
) -> Result<(), ExtractionError> {
    if format == DocumentFormat::Hwpx && !xml.trim_start().starts_with('<') {
        push_bounded(output, xml, max_bytes);
        return Ok(());
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut capture_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let local = element.local_name();
                capture_text = xml_text_element(format, local.as_ref());
                if xml_break_element(local.as_ref()) {
                    push_separator(output, max_bytes);
                }
            }
            Ok(Event::Empty(element)) => {
                if xml_break_element(element.local_name().as_ref()) {
                    push_separator(output, max_bytes);
                }
            }
            Ok(Event::Text(text)) if capture_text => {
                let decoded = text
                    .decode()
                    .map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
                let decoded = unescape(&decoded)
                    .map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
                push_bounded(output, &decoded, max_bytes);
            }
            Ok(Event::CData(text)) if capture_text => {
                let decoded = text
                    .decode()
                    .map_err(|error| ExtractionError::Unreadable(error.to_string()))?;
                push_bounded(output, &decoded, max_bytes);
            }
            Ok(Event::End(_)) => capture_text = false,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(ExtractionError::Unreadable(format!(
                    "문서 XML을 읽지 못했습니다: {error}"
                )));
            }
        }
        if output.len() >= max_bytes {
            break;
        }
    }
    Ok(())
}

fn xml_text_element(format: DocumentFormat, local_name: &[u8]) -> bool {
    match format {
        DocumentFormat::Spreadsheet => local_name == b"t" || local_name == b"v",
        DocumentFormat::Word | DocumentFormat::Presentation | DocumentFormat::Hwpx => {
            local_name == b"t"
        }
        _ => false,
    }
}

fn xml_break_element(local_name: &[u8]) -> bool {
    matches!(local_name, b"p" | b"tr" | b"row" | b"br" | b"tab")
}

fn push_separator(output: &mut String, max_bytes: usize) {
    if !output.is_empty() && output.len() < max_bytes && !output.ends_with(' ') {
        output.push(' ');
    }
}

fn push_bounded(output: &mut String, text: &str, max_bytes: usize) {
    if output.len() >= max_bytes || text.is_empty() {
        return;
    }
    if !output.is_empty()
        && !output.ends_with(char::is_whitespace)
        && !text.starts_with(char::is_whitespace)
        && output.len() < max_bytes
    {
        output.push(' ');
    }
    let remaining = max_bytes.saturating_sub(output.len());
    let mut end = remaining.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&text[..end]);
}

fn normalize_text(text: &str, max_bytes: usize) -> String {
    let mut normalized = String::with_capacity(text.len().min(max_bytes));
    let mut pending_space = false;
    for character in text.chars() {
        if character.is_whitespace() || character.is_control() {
            pending_space = !normalized.is_empty();
            continue;
        }
        if pending_space && normalized.len() < max_bytes {
            normalized.push(' ');
        }
        pending_space = false;
        if normalized.len().saturating_add(character.len_utf8()) > max_bytes {
            break;
        }
        normalized.push(character);
    }
    normalized
}

fn emit_periodic_progress<F>(on_progress: &mut F, progress: DocumentIndexProgress)
where
    F: FnMut(DocumentIndexProgress),
{
    if !progress
        .scanned_files
        .is_multiple_of(PROGRESS_FILE_INTERVAL)
    {
        return;
    }
    on_progress(progress);
}

fn supported_extensions() -> Vec<String> {
    PLAIN_TEXT_EXTENSIONS
        .iter()
        .copied()
        .chain(["pdf", "docx", "xlsx", "pptx", "hwpx"])
        .map(str::to_owned)
        .collect()
}

fn normalized_extensions(extensions: Vec<String>) -> Vec<String> {
    let mut extensions: Vec<String> = extensions
        .into_iter()
        .map(|extension| extension.trim().trim_start_matches('.').to_lowercase())
        .filter(|extension| {
            !extension.is_empty()
                && extension.len() <= 16
                && extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
        .collect();
    extensions.sort_unstable();
    extensions.dedup();
    extensions.truncate(32);
    extensions
}

fn extension_clause(extensions: &[String], first_parameter: usize) -> String {
    if extensions.is_empty() {
        return String::new();
    }
    let placeholders = (0..extensions.len())
        .map(|offset| format!("?{}", first_parameter + offset))
        .collect::<Vec<_>>()
        .join(", ");
    format!(" AND d.extension IN ({placeholders})")
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

fn parse_marked_snippet(snippet: &str) -> Vec<DocumentSnippetPart> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    let mut highlighted = false;
    for character in snippet.chars() {
        if character == SNIPPET_OPEN || character == SNIPPET_CLOSE {
            if !buffer.is_empty() {
                parts.push(DocumentSnippetPart {
                    text: std::mem::take(&mut buffer),
                    highlighted,
                });
            }
            highlighted = character == SNIPPET_OPEN;
        } else {
            buffer.push(character);
        }
    }
    if !buffer.is_empty() {
        parts.push(DocumentSnippetPart {
            text: buffer,
            highlighted,
        });
    }
    parts
}

fn direct_snippet(content: &str, query: &str) -> Vec<DocumentSnippetPart> {
    let lower_content = content.to_lowercase();
    let lower_query = query.to_lowercase();
    let Some(position) = lower_content.find(&lower_query) else {
        return Vec::new();
    };
    if lower_content.len() != content.len() || lower_query.len() != query.len() {
        return vec![DocumentSnippetPart {
            text: truncate_chars(content, 180),
            highlighted: false,
        }];
    }
    let start = floor_char_boundary(content, position.saturating_sub(72));
    let match_end = (position + query.len()).min(content.len());
    let end = ceil_char_boundary(content, (match_end + 108).min(content.len()));
    let mut parts = Vec::new();
    if start > 0 {
        parts.push(DocumentSnippetPart {
            text: "… ".to_owned(),
            highlighted: false,
        });
    }
    if start < position {
        parts.push(DocumentSnippetPart {
            text: content[start..position].to_owned(),
            highlighted: false,
        });
    }
    parts.push(DocumentSnippetPart {
        text: content[position..match_end].to_owned(),
        highlighted: true,
    });
    if match_end < end {
        parts.push(DocumentSnippetPart {
            text: content[match_end..end].to_owned(),
            highlighted: false,
        });
    }
    if end < content.len() {
        parts.push(DocumentSnippetPart {
            text: " …".to_owned(),
            highlighted: false,
        });
    }
    parts
}

fn fallback_snippet(
    source: DocumentMatchSource,
    name: &str,
    path: &str,
    query: &str,
) -> Vec<DocumentSnippetPart> {
    let value = match source {
        DocumentMatchSource::Name => name,
        DocumentMatchSource::Path => path,
        DocumentMatchSource::Content => "내용에서 일치했지만 미리보기를 만들 수 없습니다",
    };
    if source == DocumentMatchSource::Content {
        return vec![DocumentSnippetPart {
            text: value.to_owned(),
            highlighted: false,
        }];
    }
    direct_snippet(value, query)
}

fn contains_case_insensitive(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(&query.to_lowercase())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        output.push('…');
    }
    output
}

fn floor_char_boundary(value: &str, mut index: usize) -> usize {
    index = index.min(value.len());
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(value: &str, mut index: usize) -> usize {
    index = index.min(value.len());
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn push_issue(
    issues: &mut Vec<DocumentIndexIssue>,
    max_issues: usize,
    path: Option<String>,
    message: String,
) {
    if issues.len() < max_issues {
        issues.push(DocumentIndexIssue { path, message });
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

fn index_error(error: rusqlite::Error) -> DocumentSearchError {
    DocumentSearchError::Index(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    #[test]
    fn indexes_plain_text_searches_korean_and_reuses_unchanged_documents() {
        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        fs::write(
            documents.join("contract.md"),
            "프로젝트 계약 변경 내용과 저장공간 분석 결과",
        )
        .expect("write document");
        fs::write(documents.join("other.txt"), "전혀 다른 문서").expect("write other");

        let first = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build first index");
        assert_eq!(first.status.indexed_documents, 2);
        assert_eq!(first.updated_documents, 2);

        let result = search_document_index(
            &database,
            DocumentSearchRequest {
                query: "계약 변경".to_owned(),
                extensions: Vec::new(),
                max_results: 20,
            },
        )
        .expect("search index");
        assert_eq!(result.total_matches, 1);
        assert_eq!(result.results[0].name, "contract.md");
        assert_eq!(result.results[0].match_source, DocumentMatchSource::Content);
        assert!(
            result.results[0]
                .snippet
                .iter()
                .any(|part| part.highlighted)
        );

        let filtered_out = search_document_index(
            &database,
            DocumentSearchRequest {
                query: "계약 변경".to_owned(),
                extensions: vec!["pdf".to_owned()],
                max_results: 20,
            },
        )
        .expect("search filtered index");
        assert_eq!(filtered_out.total_matches, 0);

        let short_query = search_document_index(
            &database,
            DocumentSearchRequest {
                query: "계약".to_owned(),
                extensions: vec!["md".to_owned()],
                max_results: 20,
            },
        )
        .expect("search with two-character fallback");
        assert_eq!(short_query.total_matches, 1);
        assert!(
            short_query.results[0]
                .snippet
                .iter()
                .any(|part| part.highlighted)
        );

        let second = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build incremental index");
        assert_eq!(second.updated_documents, 0);
        assert_eq!(second.reused_documents, 2);
        assert_eq!(
            document_index_status(&database)
                .expect("load status")
                .expect("status")
                .indexed_documents,
            2
        );
    }

    #[test]
    fn extracts_docx_and_hwpx_text_from_bounded_archives() {
        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        write_archive(
            &documents.join("brief.docx"),
            "word/document.xml",
            r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>분기 보고서 핵심</w:t></w:r></w:p></w:body></w:document>"#,
        );
        write_archive(
            &documents.join("plan.hwpx"),
            "Contents/section0.xml",
            r#"<hs:sec xmlns:hs="h" xmlns:hp="p"><hp:p><hp:run><hp:t>한글 계획 문서</hp:t></hp:run></hp:p></hs:sec>"#,
        );

        let report = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build archive index");
        assert_eq!(report.status.indexed_documents, 2);

        for (query, expected) in [("분기 보고서", "brief.docx"), ("한글 계획", "plan.hwpx")]
        {
            let result = search_document_index(
                &database,
                DocumentSearchRequest {
                    query: query.to_owned(),
                    extensions: Vec::new(),
                    max_results: 20,
                },
            )
            .expect("search archive index");
            assert_eq!(result.total_matches, 1);
            assert_eq!(result.results[0].name, expected);
        }
    }

    #[test]
    fn indexes_windows_cp949_plain_text() {
        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        let (encoded, _, had_errors) = EUC_KR.encode("윈도우 한글 문서 검색");
        assert!(!had_errors);
        fs::write(documents.join("legacy-encoding.txt"), encoded.as_ref())
            .expect("write CP949 fixture");

        build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build CP949 index");
        let result = search_document_index(
            &database,
            DocumentSearchRequest {
                query: "한글 문서".to_owned(),
                extensions: Vec::new(),
                max_results: 20,
            },
        )
        .expect("search CP949 index");
        assert_eq!(result.total_matches, 1);
    }

    #[test]
    fn indexes_text_layer_from_pdf_without_ocr() {
        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        write_simple_pdf(
            &documents.join("searchable.pdf"),
            "Searchable contract change summary",
        );

        let report = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build PDF index");
        assert_eq!(report.status.indexed_documents, 1);
        let result = search_document_index(
            &database,
            DocumentSearchRequest {
                query: "contract change".to_owned(),
                extensions: vec!["pdf".to_owned()],
                max_results: 20,
            },
        )
        .expect("search PDF index");
        assert_eq!(result.total_matches, 1);
        assert_eq!(result.results[0].format, DocumentFormat::Pdf);
    }

    #[test]
    fn cancellation_rolls_back_an_incomplete_refresh() {
        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        fs::write(documents.join("stable.txt"), "기존 색인 내용").expect("write stable");
        build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build initial index");
        fs::write(documents.join("new.txt"), "취소 뒤 남으면 안 되는 내용").expect("write new");

        let cancelled = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || true,
        );
        assert!(matches!(cancelled, Err(DocumentSearchError::Cancelled)));

        let status = document_index_status(&database)
            .expect("read previous status")
            .expect("previous status");
        assert_eq!(status.indexed_documents, 1);
    }

    #[test]
    fn legacy_hwp_is_reported_without_being_indexed() {
        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        fs::write(documents.join("legacy.hwp"), b"HWP Document File").expect("write hwp");

        let report = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("build index");
        assert_eq!(report.unsupported_documents, 1);
        assert_eq!(report.status.indexed_documents, 0);
    }

    #[cfg(windows)]
    #[test]
    fn exclusively_locked_document_is_skipped_without_blocking() {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        let fixture = tempfile::tempdir().expect("fixture");
        let database = fixture.path().join("index.sqlite3");
        let documents = fixture.path().join("documents");
        fs::create_dir(&documents).expect("document root");
        let path = documents.join("locked.txt");
        fs::write(&path, "잠긴 문서 내용").expect("write locked fixture");
        let _exclusive = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&path)
            .expect("lock fixture exclusively");

        let report = build_document_index(
            &documents,
            &database,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .expect("complete index around locked file");
        assert_eq!(report.status.indexed_documents, 0);
        assert_eq!(report.unreadable_entries, 1);
        assert_eq!(report.skipped_documents, 1);
    }

    fn write_archive(path: &Path, entry_name: &str, content: &str) {
        let file = fs::File::create(path).expect("create archive");
        let mut archive = ZipWriter::new(file);
        archive
            .start_file(entry_name, SimpleFileOptions::default())
            .expect("start archive entry");
        archive
            .write_all(content.as_bytes())
            .expect("write archive");
        archive.finish().expect("finish archive");
    }

    fn write_simple_pdf(path: &Path, text: &str) {
        let content = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0_usize];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object)
                .expect("write PDF object");
        }
        let xref = pdf.len();
        write!(&mut pdf, "xref\n0 {}\n", objects.len() + 1).expect("write PDF xref");
        writeln!(&mut pdf, "0000000000 65535 f ").expect("write PDF free entry");
        for offset in offsets.iter().skip(1) {
            writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write PDF offset");
        }
        write!(
            &mut pdf,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .expect("write PDF trailer");
        fs::write(path, pdf).expect("write PDF fixture");
    }
}
