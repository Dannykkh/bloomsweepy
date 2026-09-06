#![cfg(target_os = "macos")]

use bloomsweepy_core::{
    DirectoryScanConfig, DocumentIndexConfig, DocumentSearchRequest, FileCatalogConfig,
    FileCatalogSearchRequest, FileCatalogSort, ScanConfig, ScanError, build_document_index,
    build_file_catalog, scan_directory_level, scan_path, search_document_index,
    search_file_catalog,
};
use std::fs::{self, File};
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

// A bounded native regression, not an all-day soak or a physical 8 GiB read test.
// Everything lives in a test-owned temporary directory; no user data is scanned.
#[test]
fn repeated_native_scans_preserve_results_and_bound_resources() {
    let cycles = std::env::var("BROOMSWEEPY_MAC_SOAK_CYCLES")
        .map(|value| {
            value
                .parse::<usize>()
                .expect("soak cycles must be an integer")
        })
        .unwrap_or(20);
    assert!((1..=2000).contains(&cycles), "soak cycles must be 1..=2000");
    let temp = tempfile::tempdir().expect("create native scan fixture");
    let root = temp.path().join("files");
    fs::create_dir(&root).unwrap();
    for directory in 0..32 {
        let child = root.join(format!("폴더-{directory:02}"));
        fs::create_dir(&child).unwrap();
        for index in 0..128 {
            fs::write(child.join(format!("entry-{index:03}.dat")), b"fixture\n").unwrap();
        }
    }
    let catalog = temp.path().join("catalog.sqlite3");
    for _ in 0..3 {
        run_file_cycle(&root, &catalog);
    }
    let baseline = resources();
    let started = Instant::now();
    for _ in 0..cycles {
        run_file_cycle(&root, &catalog);
    }
    assert_bounded(
        &format!("4096 files, {cycles} scan/map/catalog cycles"),
        baseline,
        resources(),
    );
    println!("file cycles elapsed: {:?}", started.elapsed());

    // Cancellation must stop real traversal, and a later scan must still succeed.
    let cancelled = AtomicBool::new(false);
    let result = scan_path(
        &root,
        scan_config(),
        |progress| {
            if progress.processed_files >= 512 {
                cancelled.store(true, Ordering::Release);
            }
        },
        || cancelled.load(Ordering::Acquire),
    );
    assert!(matches!(result, Err(ScanError::Cancelled)));
    run_file_cycle(&root, &catalog);

    let documents = temp.path().join("documents");
    fs::create_dir(&documents).unwrap();
    for index in 0..128 {
        fs::write(
            documents.join(format!("document-{index:03}.txt")),
            format!("반복검증 검색표식 document {index}"),
        )
        .unwrap();
    }
    let document_db = temp.path().join("documents.sqlite3");
    for _ in 0..3 {
        run_document_cycle(&documents, &document_db);
    }
    let baseline = resources();
    for _ in 0..cycles {
        run_document_cycle(&documents, &document_db);
    }
    assert_bounded(
        &format!("128 documents, {cycles} index/search cycles"),
        baseline,
        resources(),
    );

    verify_sparse_file_and_link_boundaries(temp.path());
}

fn scan_config() -> ScanConfig {
    ScanConfig {
        min_large_file_bytes: 1,
        // The separate duplicate fixture below covers content comparison.
        min_duplicate_file_bytes: 1024,
        max_large_files: 32,
        max_duplicate_groups: 32,
        max_duplicate_candidates: 1_000,
        max_issues: 32,
    }
}

fn run_file_cycle(root: &Path, database: &Path) {
    let scan = scan_path(root, scan_config(), |_| {}, || false).unwrap();
    assert_eq!(scan.total_files, 4096);
    assert_eq!(scan.large_files.len(), 32);
    let map = scan_directory_level(root, DirectoryScanConfig::default(), |_| {}, || false).unwrap();
    assert_eq!(map.total_files, 4096);
    assert_eq!(map.total_logical_bytes, 4096 * 8);
    assert_eq!(map.children.len(), 32);
    assert_eq!(map.unreadable_entries, 0);
    let report = build_file_catalog(
        root,
        database,
        FileCatalogConfig::default(),
        |_| {},
        || false,
    )
    .unwrap();
    assert_eq!(report.status.indexed_files, 4096);
    let found = search_file_catalog(
        database,
        FileCatalogSearchRequest {
            query: "entry-007".into(),
            kind: None,
            extensions: Vec::new(),
            min_bytes: None,
            max_bytes: None,
            timezone_offset_minutes: 0,
            sort: FileCatalogSort::Relevance,
            max_results: 100,
        },
    )
    .unwrap();
    assert_eq!(found.results.len(), 32);
}

fn run_document_cycle(root: &Path, database: &Path) {
    let report = build_document_index(
        root,
        database,
        DocumentIndexConfig::default(),
        |_| {},
        || false,
    )
    .unwrap();
    assert_eq!(report.status.indexed_documents, 128);
    let found = search_document_index(
        database,
        DocumentSearchRequest {
            query: "반복검증 검색표식".into(),
            extensions: Vec::new(),
            max_results: 200,
        },
    )
    .unwrap();
    assert_eq!(found.total_matches, 128);
}

fn verify_sparse_file_and_link_boundaries(parent: &Path) {
    let root = parent.join("sparse");
    fs::create_dir(&root).unwrap();
    let path = root.join("8-gib-sparse.bin");
    let file = File::create(&path).unwrap();
    let logical_bytes = 8 * 1024 * 1024 * 1024;
    file.set_len(logical_bytes).unwrap();
    assert!(file.metadata().unwrap().blocks() * 512 < 1024 * 1024);
    drop(file);
    symlink(&root, root.join("cycle-link")).unwrap();
    let map =
        scan_directory_level(&root, DirectoryScanConfig::default(), |_| {}, || false).unwrap();
    assert_eq!(map.total_files, 1);
    assert_eq!(map.total_logical_bytes, logical_bytes);
    assert_eq!(map.children.len(), 1);

    let duplicates = parent.join("duplicates");
    fs::create_dir(&duplicates).unwrap();
    for index in 0..16 {
        fs::write(
            duplicates.join(format!("copy-{index}.bin")),
            vec![b'x'; 256 * 1024],
        )
        .unwrap();
    }
    for _ in 0..20 {
        let scan = scan_path(&duplicates, scan_config(), |_| {}, || false).unwrap();
        assert_eq!(scan.total_files, 16);
        assert_eq!(scan.duplicate_groups.len(), 1);
    }
    println!("8 GiB sparse logical size, symlink cycle, cancellation/retry, duplicate scans: pass");
}

#[derive(Clone, Copy, Debug)]
struct Resources {
    descriptors: usize,
    resident_kib: u64,
}

fn resources() -> Resources {
    let descriptors = fs::read_dir("/dev/fd").unwrap().count();
    let output = Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let resident_kib = String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    Resources {
        descriptors,
        resident_kib,
    }
}

fn assert_bounded(label: &str, before: Resources, after: Resources) {
    println!("{label}: {before:?} -> {after:?}");
    assert!(
        after.descriptors <= before.descriptors + 2,
        "open descriptors grew: {label}"
    );
    // Warmed allocator/SQLite caches need not immediately return every page to macOS.
    assert!(
        after.resident_kib <= before.resident_kib + 64 * 1024,
        "RSS grew over 64 MiB: {label}"
    );
}
