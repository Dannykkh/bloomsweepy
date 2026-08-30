#![cfg(windows)]

use bloomsweepy_core::{
    DocumentIndexConfig, DocumentSearchRequest, FileCatalogConfig, FileCatalogSearchRequest,
    FileCatalogSort, ScanConfig, build_document_index, build_file_catalog, scan_path,
    search_document_index, search_file_catalog,
};
use std::fs;
use std::mem::size_of;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetProcessHandleCount,
};

#[test]
fn repeated_scans_release_file_handles_threads_and_working_memory() {
    let temp = tempfile::tempdir().expect("create temp directory");
    let contents = vec![b'x'; 256 * 1024];
    for index in 0..16 {
        fs::write(
            temp.path().join(format!("duplicate-{index:02}.bin")),
            &contents,
        )
        .expect("write duplicate fixture");
    }

    run_scan(temp.path());
    thread::sleep(Duration::from_millis(25));
    let baseline_handles = process_handle_count();
    let baseline_threads = process_thread_count();
    let baseline_private_bytes = process_private_bytes();

    for _ in 0..20 {
        run_scan(temp.path());
    }
    thread::sleep(Duration::from_millis(50));

    let final_handles = process_handle_count();
    let final_threads = process_thread_count();
    let final_private_bytes = process_private_bytes();

    println!(
        "handles {baseline_handles}->{final_handles}, threads {baseline_threads}->{final_threads}, private_bytes {baseline_private_bytes}->{final_private_bytes}"
    );

    assert!(
        final_handles <= baseline_handles.saturating_add(2),
        "handle count grew from {baseline_handles} to {final_handles}"
    );
    assert!(
        final_threads <= baseline_threads.saturating_add(1),
        "thread count grew from {baseline_threads} to {final_threads}"
    );
    assert!(
        final_private_bytes <= baseline_private_bytes.saturating_add(16 * 1024 * 1024),
        "private bytes grew from {baseline_private_bytes} to {final_private_bytes}"
    );

    let document_root = temp.path().join("documents");
    fs::create_dir(&document_root).expect("create document fixture");
    for index in 0..16 {
        fs::write(
            document_root.join(format!("document-{index:02}.txt")),
            format!("반복 문서 색인 안정성 검색어 {index}"),
        )
        .expect("write document fixture");
    }
    let database = temp.path().join("document-index.sqlite3");
    run_document_index(&document_root, &database);
    thread::sleep(Duration::from_millis(25));
    let document_baseline_handles = process_handle_count();
    let document_baseline_threads = process_thread_count();
    let document_baseline_private_bytes = process_private_bytes();

    for _ in 0..20 {
        run_document_index(&document_root, &database);
    }
    thread::sleep(Duration::from_millis(50));

    let document_final_handles = process_handle_count();
    let document_final_threads = process_thread_count();
    let document_final_private_bytes = process_private_bytes();
    println!(
        "document handles {document_baseline_handles}->{document_final_handles}, threads {document_baseline_threads}->{document_final_threads}, private_bytes {document_baseline_private_bytes}->{document_final_private_bytes}"
    );
    assert!(
        document_final_handles <= document_baseline_handles.saturating_add(2),
        "document index handle count grew from {document_baseline_handles} to {document_final_handles}"
    );
    assert!(
        document_final_threads <= document_baseline_threads.saturating_add(1),
        "document index thread count grew from {document_baseline_threads} to {document_final_threads}"
    );
    assert!(
        document_final_private_bytes
            <= document_baseline_private_bytes.saturating_add(16 * 1024 * 1024),
        "document index private bytes grew from {document_baseline_private_bytes} to {document_final_private_bytes}"
    );

    let catalog_root = temp.path().join("catalog-files");
    fs::create_dir(&catalog_root).expect("create file catalog fixture");
    for index in 0..64 {
        fs::write(
            catalog_root.join(format!("catalog-stability-{index:02}.dat")),
            format!("fixture {index}"),
        )
        .expect("write file catalog fixture");
    }
    let catalog_database = temp.path().join("file-catalog.sqlite3");
    run_file_catalog(&catalog_root, &catalog_database);
    thread::sleep(Duration::from_millis(25));
    let catalog_baseline_handles = process_handle_count();
    let catalog_baseline_threads = process_thread_count();
    let catalog_baseline_private_bytes = process_private_bytes();

    for _ in 0..20 {
        run_file_catalog(&catalog_root, &catalog_database);
    }
    thread::sleep(Duration::from_millis(50));

    let catalog_final_handles = process_handle_count();
    let catalog_final_threads = process_thread_count();
    let catalog_final_private_bytes = process_private_bytes();
    println!(
        "file catalog handles {catalog_baseline_handles}->{catalog_final_handles}, threads {catalog_baseline_threads}->{catalog_final_threads}, private_bytes {catalog_baseline_private_bytes}->{catalog_final_private_bytes}"
    );
    assert!(
        catalog_final_handles <= catalog_baseline_handles.saturating_add(2),
        "file catalog handle count grew from {catalog_baseline_handles} to {catalog_final_handles}"
    );
    assert!(
        catalog_final_threads <= catalog_baseline_threads.saturating_add(1),
        "file catalog thread count grew from {catalog_baseline_threads} to {catalog_final_threads}"
    );
    assert!(
        catalog_final_private_bytes
            <= catalog_baseline_private_bytes.saturating_add(16 * 1024 * 1024),
        "file catalog private bytes grew from {catalog_baseline_private_bytes} to {catalog_final_private_bytes}"
    );
}

fn run_scan(root: &std::path::Path) {
    let report = scan_path(
        root,
        ScanConfig {
            min_large_file_bytes: 1,
            min_duplicate_file_bytes: 1,
            max_large_files: 32,
            max_duplicate_groups: 32,
            max_duplicate_candidates: 1_000,
            max_issues: 32,
        },
        |_| {},
        || false,
    )
    .expect("repeat scan");
    assert_eq!(report.total_files, 16);
    assert_eq!(report.duplicate_groups.len(), 1);
}

fn run_document_index(root: &std::path::Path, database: &std::path::Path) {
    let report = build_document_index(
        root,
        database,
        DocumentIndexConfig::default(),
        |_| {},
        || false,
    )
    .expect("repeat document index");
    assert_eq!(report.status.indexed_documents, 16);

    let search = search_document_index(
        database,
        DocumentSearchRequest {
            query: "안정성 검색어".to_owned(),
            extensions: Vec::new(),
            max_results: 32,
        },
    )
    .expect("repeat document search");
    assert_eq!(search.total_matches, 16);
}

fn run_file_catalog(root: &std::path::Path, database: &std::path::Path) {
    let report = build_file_catalog(
        root,
        database,
        FileCatalogConfig::default(),
        |_| {},
        || false,
    )
    .expect("repeat file catalog build");
    assert_eq!(report.status.indexed_files, 64);

    let search = search_file_catalog(
        database,
        FileCatalogSearchRequest {
            query: "catalog-stability".to_owned(),
            kind: None,
            extensions: Vec::new(),
            min_bytes: None,
            max_bytes: None,
            sort: FileCatalogSort::Relevance,
            max_results: 100,
        },
    )
    .expect("repeat file catalog search");
    assert_eq!(search.results.len(), 64);
}

fn process_handle_count() -> u32 {
    let mut count = 0_u32;
    let success = unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) };
    assert_ne!(success, 0, "GetProcessHandleCount failed");
    count
}

fn process_thread_count() -> u32 {
    let process_id = unsafe { GetCurrentProcessId() };
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    assert_ne!(snapshot, INVALID_HANDLE_VALUE, "thread snapshot failed");

    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..THREADENTRY32::default()
    };
    let mut count = 0_u32;
    let mut has_entry = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while has_entry {
        if entry.th32OwnerProcessID == process_id {
            count = count.saturating_add(1);
        }
        has_entry = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe {
        CloseHandle(snapshot);
    }
    count
}

fn process_private_bytes() -> usize {
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..PROCESS_MEMORY_COUNTERS_EX::default()
    };
    let success = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        )
    };
    assert_ne!(success, 0, "K32GetProcessMemoryInfo failed");
    counters.PrivateUsage
}
