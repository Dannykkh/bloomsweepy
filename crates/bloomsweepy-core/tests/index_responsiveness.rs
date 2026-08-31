use bloomsweepy_core::{
    DocumentIndexConfig, DocumentIndexPhase, DocumentSearchError, DocumentSearchRequest,
    FileCatalogConfig, FileCatalogError, FileCatalogPhase, FileCatalogSearchRequest,
    FileCatalogSort, build_document_index, build_file_catalog, document_index_status,
    file_catalog_status, search_document_index, search_file_catalog,
};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

const READ_LATENCY_LIMIT: Duration = Duration::from_millis(500);
const WORKER_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn document_refresh_keeps_completed_snapshot_readable_and_cancellable() {
    let temporary = tempfile::tempdir().expect("create document fixture");
    let root = temporary.path().join("documents");
    let database = temporary.path().join("document-index.sqlite3");
    fs::create_dir(&root).expect("create document root");
    fs::write(root.join("stable.txt"), "stable snapshot token").expect("write stable document");

    let initial = build_document_index(
        &root,
        &database,
        DocumentIndexConfig::default(),
        |_| {},
        || false,
    )
    .expect("build initial document index");
    fs::write(root.join("new.txt"), "uncommitted snapshot token").expect("write new document");

    let cancellation = Arc::new(AtomicBool::new(false));
    let worker_cancellation = Arc::clone(&cancellation);
    let worker_root = root.clone();
    let worker_database = database.clone();
    let (paused_tx, paused_rx) = mpsc::sync_channel(0);
    let (resume_tx, resume_rx) = mpsc::sync_channel(0);
    let worker = thread::spawn(move || {
        let mut paused = false;
        build_document_index(
            worker_root,
            worker_database,
            DocumentIndexConfig::default(),
            move |progress| {
                if !paused && matches!(progress.phase, DocumentIndexPhase::Finalizing) {
                    paused = true;
                    paused_tx.send(()).expect("signal document finalizing");
                    resume_rx
                        .recv_timeout(WORKER_TIMEOUT)
                        .expect("resume document finalizing");
                }
            },
            move || worker_cancellation.load(Ordering::Acquire),
        )
    });
    paused_rx
        .recv_timeout(WORKER_TIMEOUT)
        .expect("document refresh reached finalizing");

    let status_started = Instant::now();
    let visible_status = document_index_status(&database)
        .expect("read document status during refresh")
        .expect("completed document status during refresh");
    assert_read_responsive("document status", status_started);
    assert_eq!(
        visible_status.indexed_documents,
        initial.status.indexed_documents
    );

    let search_started = Instant::now();
    let stable_results = search_document_index(
        &database,
        DocumentSearchRequest {
            query: "stable snapshot".to_owned(),
            extensions: Vec::new(),
            max_results: 10,
        },
    )
    .expect("search completed document snapshot");
    assert_read_responsive("document search", search_started);
    assert_eq!(stable_results.total_matches, 1);

    cancellation.store(true, Ordering::Release);
    resume_tx
        .send(())
        .expect("resume cancelled document refresh");
    let refresh_error = worker
        .join()
        .expect("join document refresh")
        .expect_err("finalizing document refresh should cancel");
    assert!(matches!(refresh_error, DocumentSearchError::Cancelled));

    let final_status = document_index_status(&database)
        .expect("read document status after cancellation")
        .expect("completed document status after cancellation");
    assert_eq!(
        final_status.indexed_documents,
        initial.status.indexed_documents
    );
    assert_eq!(
        search_document_index(
            &database,
            DocumentSearchRequest {
                query: "uncommitted snapshot".to_owned(),
                extensions: Vec::new(),
                max_results: 10,
            },
        )
        .expect("search rolled back document snapshot")
        .total_matches,
        0
    );
}

#[test]
fn file_catalog_refresh_keeps_completed_snapshot_readable_and_cancellable() {
    let temporary = tempfile::tempdir().expect("create catalog fixture");
    let root = temporary.path().join("catalog");
    let database = temporary.path().join("file-catalog.sqlite3");
    fs::create_dir(&root).expect("create catalog root");
    fs::write(root.join("stable-entry.txt"), b"stable").expect("write stable catalog entry");

    let initial = build_file_catalog(
        &root,
        &database,
        FileCatalogConfig::default(),
        |_| {},
        || false,
    )
    .expect("build initial file catalog");
    fs::write(root.join("new-entry.txt"), b"new").expect("write new catalog entry");

    let cancellation = Arc::new(AtomicBool::new(false));
    let worker_cancellation = Arc::clone(&cancellation);
    let worker_root = root.clone();
    let worker_database = database.clone();
    let (paused_tx, paused_rx) = mpsc::sync_channel(0);
    let (resume_tx, resume_rx) = mpsc::sync_channel(0);
    let worker = thread::spawn(move || {
        let mut paused = false;
        build_file_catalog(
            worker_root,
            worker_database,
            FileCatalogConfig::default(),
            move |progress| {
                if !paused && matches!(progress.phase, FileCatalogPhase::Finalizing) {
                    paused = true;
                    paused_tx.send(()).expect("signal catalog finalizing");
                    resume_rx
                        .recv_timeout(WORKER_TIMEOUT)
                        .expect("resume catalog finalizing");
                }
            },
            move || worker_cancellation.load(Ordering::Acquire),
        )
    });
    paused_rx
        .recv_timeout(WORKER_TIMEOUT)
        .expect("catalog refresh reached finalizing");

    let status_started = Instant::now();
    let visible_status = file_catalog_status(&database)
        .expect("read catalog status during refresh")
        .expect("completed catalog status during refresh");
    assert_read_responsive("catalog status", status_started);
    assert_eq!(
        visible_status.indexed_entries,
        initial.status.indexed_entries
    );

    let search_started = Instant::now();
    let stable_results = search_file_catalog(&database, catalog_request("stable-entry"))
        .expect("search completed catalog snapshot");
    assert_read_responsive("catalog search", search_started);
    assert_eq!(stable_results.results.len(), 1);

    cancellation.store(true, Ordering::Release);
    resume_tx
        .send(())
        .expect("resume cancelled catalog refresh");
    let refresh_error = worker
        .join()
        .expect("join catalog refresh")
        .expect_err("finalizing catalog refresh should cancel");
    assert!(matches!(refresh_error, FileCatalogError::Cancelled));

    let final_status = file_catalog_status(&database)
        .expect("read catalog status after cancellation")
        .expect("completed catalog status after cancellation");
    assert_eq!(final_status.indexed_entries, initial.status.indexed_entries);
    assert!(
        search_file_catalog(&database, catalog_request("new-entry"))
            .expect("search rolled back catalog snapshot")
            .results
            .is_empty()
    );
}

fn catalog_request(query: &str) -> FileCatalogSearchRequest {
    FileCatalogSearchRequest {
        query: query.to_owned(),
        kind: None,
        extensions: Vec::new(),
        min_bytes: None,
        max_bytes: None,
        timezone_offset_minutes: 0,
        sort: FileCatalogSort::Relevance,
        max_results: 10,
    }
}

fn assert_read_responsive(operation: &str, started: Instant) {
    let elapsed = started.elapsed();
    println!("{operation} latency while writer is active: {elapsed:?}");
    assert!(
        elapsed < READ_LATENCY_LIMIT,
        "{operation} took {elapsed:?} while a writer transaction was active"
    );
}
