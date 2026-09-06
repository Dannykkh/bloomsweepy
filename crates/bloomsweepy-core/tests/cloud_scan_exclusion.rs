use bloomsweepy_core::{
    DirectoryScanConfig, DocumentIndexConfig, DocumentSearchRequest, DriveScanConfig,
    FileCatalogConfig, FileCatalogSearchRequest, FileCatalogSort, ScanConfig, build_document_index,
    build_file_catalog, scan_directory_level, scan_drive, scan_path, search_document_index,
    search_file_catalog,
};
use std::fs;
use std::path::Path;

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn scan_config() -> ScanConfig {
    ScanConfig {
        min_large_file_bytes: 1,
        min_duplicate_file_bytes: 1,
        ..ScanConfig::default()
    }
}

#[test]
fn every_scan_excludes_cloud_subtrees_but_keeps_local_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("disk");
    let local = b"local-marker duplicate fixture";
    for name in ["first.txt", "second.txt"] {
        write(&root.join("Users/test/Documents").join(name), local);
    }
    for cloud in [
        "Users/test/Library/CloudStorage/GoogleDrive-test/My Drive",
        "Users/test/Library/CloudStorage/UnknownProvider",
        "Users/test/Library/Mobile Documents/com~apple~CloudDocs",
        "Users/other/Library/CloudStorage/Dropbox",
        "System/Volumes/Data/Users/test/Library/CloudStorage/OneDrive",
        "Users/test/Google Drive",
        "Users/test/OneDrive - Company",
        "Users/test/Dropbox (Personal)",
        "Volumes/GoogleDrive-test@example.com",
    ] {
        // Even locally downloaded cloud files must not be counted or hashed.
        write(&root.join(cloud).join("third.txt"), local);
        write(&root.join(cloud).join("secret.txt"), b"cloud-only-marker");
    }
    let scan = scan_path(&root, scan_config(), |_| {}, || false).unwrap();
    assert_eq!(scan.total_files, 2);
    assert_eq!(scan.total_logical_bytes, 2 * local.len() as u64);
    assert_eq!(scan.large_files.len(), 2);
    assert_eq!(scan.duplicate_groups.len(), 1);
    assert_eq!(scan.duplicate_groups[0].files.len(), 2);
    assert_eq!(scan.unreadable_entries, 0);

    let drive = scan_drive(&root, DriveScanConfig::default(), |_| {}, || false).unwrap();
    assert_eq!(drive.total_files, 2);
    assert_eq!(drive.total_logical_bytes, scan.total_logical_bytes);
    let map =
        scan_directory_level(&root, DirectoryScanConfig::default(), |_| {}, || false).unwrap();
    assert_eq!(map.total_files, 2);
    assert_eq!(map.total_logical_bytes, scan.total_logical_bytes);
    // Parents containing only excluded children must not become "empty folders".
    assert!(map.empty_directories.is_empty());

    let catalog_db = temp.path().join("catalog.sqlite3");
    let catalog = build_file_catalog(
        &root,
        &catalog_db,
        FileCatalogConfig::default(),
        |_| {},
        || false,
    )
    .unwrap();
    assert_eq!(catalog.status.indexed_files, 2);
    let found = search_file_catalog(
        &catalog_db,
        FileCatalogSearchRequest {
            query: "secret".into(),
            kind: None,
            extensions: vec![],
            min_bytes: None,
            max_bytes: None,
            timezone_offset_minutes: 0,
            sort: FileCatalogSort::Relevance,
            max_results: 100,
        },
    )
    .unwrap();
    assert!(found.results.is_empty());

    let documents_db = temp.path().join("documents.sqlite3");
    let documents = build_document_index(
        &root,
        &documents_db,
        DocumentIndexConfig::default(),
        |_| {},
        || false,
    )
    .unwrap();
    assert_eq!(documents.status.indexed_documents, 2);
    let found = search_document_index(
        &documents_db,
        DocumentSearchRequest {
            query: "cloud-only-marker".into(),
            extensions: vec![],
            max_results: 100,
        },
    )
    .unwrap();
    assert_eq!(found.total_matches, 0);
}

fn assert_all_reject(root: &Path, database: &Path) {
    let expected = "클라우드 동기화 폴더";
    assert!(
        scan_path(root, scan_config(), |_| {}, || false)
            .unwrap_err()
            .to_string()
            .contains(expected)
    );
    assert!(
        scan_drive(root, DriveScanConfig::default(), |_| {}, || false)
            .unwrap_err()
            .to_string()
            .contains(expected)
    );
    assert!(
        scan_directory_level(root, DirectoryScanConfig::default(), |_| {}, || false)
            .unwrap_err()
            .to_string()
            .contains(expected)
    );
    assert!(
        build_file_catalog(
            root,
            database,
            FileCatalogConfig::default(),
            |_| {},
            || false
        )
        .unwrap_err()
        .to_string()
        .contains(expected)
    );
    assert!(
        build_document_index(
            root,
            database,
            DocumentIndexConfig::default(),
            |_| {},
            || false
        )
        .unwrap_err()
        .to_string()
        .contains(expected)
    );
    assert!(!database.exists(), "blocked roots must not create an index");
}

#[test]
fn explicit_cloud_roots_are_rejected_before_access_or_database_creation() {
    let temp = tempfile::tempdir().unwrap();
    let cloud = temp.path().join("Users/test/Library/CloudStorage/provider");
    assert_all_reject(&cloud, &temp.path().join("not-created.sqlite3"));
}

#[cfg(unix)]
#[test]
fn direct_symlink_to_cloud_cannot_bypass_root_policy() {
    let temp = tempfile::tempdir().unwrap();
    let cloud = temp
        .path()
        .join("Users/test/Library/Mobile Documents/com~apple~CloudDocs");
    fs::create_dir_all(&cloud).unwrap();
    let alias = temp.path().join("innocent-alias");
    std::os::unix::fs::symlink(&cloud, &alias).unwrap();
    assert_all_reject(&alias, &temp.path().join("not-created.sqlite3"));
}

#[test]
fn refreshing_indexes_removes_previously_visible_files_moved_into_cloud() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("disk");
    write(&root.join("local.txt"), b"moving-marker");
    let catalog_db = temp.path().join("catalog.sqlite3");
    let documents_db = temp.path().join("documents.sqlite3");
    for moved in [false, true] {
        if moved {
            let cloud = root.join("Users/test/Library/CloudStorage/provider");
            fs::create_dir_all(&cloud).unwrap();
            fs::rename(root.join("local.txt"), cloud.join("local.txt")).unwrap();
        }
        let catalog = build_file_catalog(
            &root,
            &catalog_db,
            FileCatalogConfig::default(),
            |_| {},
            || false,
        )
        .unwrap();
        let documents = build_document_index(
            &root,
            &documents_db,
            DocumentIndexConfig::default(),
            |_| {},
            || false,
        )
        .unwrap();
        assert_eq!(catalog.status.indexed_files, u64::from(!moved));
        assert_eq!(documents.status.indexed_documents, u64::from(!moved));
    }
}
