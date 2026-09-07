//! User-initiated local inspection, not a general program launcher.
//! No file content is read. OS dispatch uses a path, so the final metadata check
//! narrows (but cannot atomically eliminate) concurrent filesystem replacement.
use std::fs::{self, Metadata};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

static INSPECTION_LOCK: Mutex<()> = Mutex::new(());
const MAX_PATH_BYTES: usize = 32_768;
const MAX_PATH_COMPONENTS: usize = 256;

// Keep in sync with the presentation-only fileInspectionPolicy.ts allowlist.
const DOCUMENT_EXTENSIONS: &[&str] = &[
    "aac", "avi", "avif", "bmp", "csv", "doc", "docx", "epub", "flac", "gif", "heic", "heif",
    "hwp", "hwpx", "jpeg", "jpg", "key", "log", "m4a", "m4v", "markdown", "md", "mkv", "mobi",
    "mov", "mp3", "mp4", "mpeg", "mpg", "numbers", "odp", "ods", "odt", "ogg", "opus", "pages",
    "pdf", "png", "ppt", "pptx", "rtf", "tif", "tiff", "tsv", "txt", "wav", "webm", "webp", "xls",
    "xlsx",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Inspection {
    Document,
    Directory,
    Reveal,
}

#[derive(Debug, PartialEq, Eq)]
struct MetadataStamp {
    path: PathBuf,
    length: u64,
    modified: Option<SystemTime>,
    readonly: bool,
    is_directory: bool,
    is_file: bool,
    is_link: bool,
    is_finder_alias: bool,
    #[cfg(unix)]
    unix_identity: (u64, u64, i64, i64, u32),
    #[cfg(windows)]
    windows_identity: (u32, u64, u64),
}

impl MetadataStamp {
    fn new(path: PathBuf, metadata: &Metadata, is_finder_alias: bool) -> Self {
        Self {
            path,
            length: metadata.len(),
            modified: metadata.modified().ok(),
            readonly: metadata.permissions().readonly(),
            is_directory: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_link: metadata.file_type().is_symlink() || is_reparse_point(metadata),
            is_finder_alias,
            #[cfg(unix)]
            unix_identity: {
                use std::os::unix::fs::MetadataExt;
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.ctime(),
                    metadata.ctime_nsec(),
                    metadata.mode(),
                )
            },
            #[cfg(windows)]
            windows_identity: {
                use std::os::windows::fs::MetadataExt;
                (
                    metadata.file_attributes(),
                    metadata.creation_time(),
                    metadata.last_write_time(),
                )
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct InspectionPlan {
    kind: Inspection,
    stamps: Vec<MetadataStamp>,
}

/// Pure OS destination selection. A link/alias leaf is never given to the
/// opener's reveal API, which canonicalizes Unix paths and follows the link.
#[derive(Debug, PartialEq, Eq)]
enum OsDispatch {
    OpenDocument(PathBuf),
    OpenDirectory(PathBuf),
    RevealItem(PathBuf),
}

fn dispatch_plan(plan: &InspectionPlan, force_reveal: bool) -> Result<OsDispatch, String> {
    let leaf = plan
        .stamps
        .last()
        .ok_or("The inspected path has no metadata.")?;
    if leaf.is_link || leaf.is_finder_alias {
        let parent = plan
            .stamps
            .iter()
            .rev()
            .nth(1)
            .filter(|parent| parent.is_directory && !parent.is_link && !parent.is_finder_alias)
            .filter(|parent| leaf.path.parent() == Some(parent.path.as_path()))
            .ok_or("The link's local parent folder could not be verified.")?;
        return Ok(OsDispatch::OpenDirectory(parent.path.clone()));
    }
    if force_reveal || plan.kind == Inspection::Reveal {
        return Ok(OsDispatch::RevealItem(leaf.path.clone()));
    }
    Ok(match plan.kind {
        Inspection::Document => OsDispatch::OpenDocument(leaf.path.clone()),
        Inspection::Directory => OsDispatch::OpenDirectory(leaf.path.clone()),
        Inspection::Reveal => unreachable!("reveal handled above"),
    })
}

#[cfg(target_os = "macos")]
fn finder_alias(path: &Path) -> Result<bool, String> {
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        fn getxattr(
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: usize,
            position: u32,
            options: c_int,
        ) -> isize;
    }
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| "Invalid local path.")?;
    let mut info = [0_u8; 32];
    // SDK sys/xattr.h: XATTR_NOFOLLOW=1. Never read an alias's data/resource
    // fork or ask Finder to resolve it. Finder.h FileInfo flags are at byte 8.
    let length = unsafe {
        getxattr(
            path.as_ptr(),
            c"com.apple.FinderInfo".as_ptr(),
            info.as_mut_ptr().cast(),
            info.len(),
            0,
            1,
        )
    };
    if length < 0 {
        let error = std::io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(93) => Ok(false), // SDK sys/errno.h: ENOATTR, no FinderInfo.
            Some(45) => Err("Finder alias metadata is unavailable on this filesystem.".into()), // ENOTSUP
            _ => Err(format!(
                "Finder alias metadata could not be checked: {error}"
            )),
        };
    }
    if length != info.len() as isize {
        return Err("Finder alias metadata has an unexpected size.".into());
    }
    Ok(u16::from_be_bytes([info[8], info[9]]) & 0x8000 != 0) // kIsAlias
}

#[cfg(not(target_os = "macos"))]
fn finder_alias(_path: &Path) -> Result<bool, String> {
    Ok(false)
}

fn validate_path_text(path: &str) -> Result<&Path, String> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.chars().any(char::is_control)
    {
        return Err("Only a local absolute file or folder path can be inspected.".into());
    }
    // Rust scans return canonical verbatim disk paths on Windows. Strip only
    // that exact local-disk prefix, then apply all ordinary Win32 safety checks
    // to the same path used for metadata validation and OS dispatch.
    #[cfg(windows)]
    let path = path
        .strip_prefix(r"\\?\")
        .filter(|local| {
            let bytes = local.as_bytes();
            bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'\\'
        })
        .unwrap_or(path);
    if path.starts_with("//") || path.starts_with("\\\\") {
        return Err("Only a local absolute file or folder path can be inspected.".into());
    }
    let local = Path::new(path);
    if !local.is_absolute()
        || local.components().count() > MAX_PATH_COMPONENTS
        || local
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("Relative paths and parent traversal cannot be inspected.".into());
    }
    #[cfg(windows)]
    {
        // Exclude UNC/device namespaces, alternate streams and Win32 name aliases.
        if !matches!(local.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), std::path::Prefix::Disk(_)))
            || path.get(2..).is_some_and(|suffix| suffix.contains(':'))
            || local.components().any(|part| {
                matches!(part, Component::Normal(name)
                if windows_name_alias(&name.to_string_lossy()))
            })
        {
            return Err("Network, device and alternate-stream paths cannot be inspected.".into());
        }
    }
    Ok(local)
}

#[cfg(windows)]
fn windows_name_alias(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").to_ascii_lowercase();
    name.ends_with(['.', ' '])
        || matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
        || stem
            .strip_prefix("com")
            .or_else(|| stem.strip_prefix("lpt"))
            .is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

fn executable_package(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    [
        "app",
        "appex",
        "application",
        "action",
        "bundle",
        "framework",
        "kext",
        "mdimporter",
        "mpkg",
        "pkg",
        "plugin",
        "prefpane",
        "qlgenerator",
        "saver",
        "service",
        "vst",
        "vst3",
        "wdgt",
        "workflow",
        "xpc",
    ]
    .iter()
    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

fn is_reparse_point(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

fn executable(metadata: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        false
    }
}

fn document_extension(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if name.contains(':') || name.starts_with('.') && !name[1..].contains('.') {
        return false;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    DOCUMENT_EXTENSIONS
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
}

fn inspection_plan(path: &str) -> Result<InspectionPlan, String> {
    let path = validate_path_text(path)?;
    // Lexical exclusion is intentionally before even symlink_metadata.
    if bloomsweepy_core::is_cloud_storage_path(path) {
        return Err("Cloud storage is excluded. Choose a local file or folder.".into());
    }
    let mut cursor = PathBuf::new();
    let mut stamps = Vec::new();
    let mut within_package = false;
    let parts: Vec<_> = path.components().collect();
    let mut kind = Inspection::Reveal;
    for (index, component) in parts.iter().enumerate() {
        cursor.push(component.as_os_str());
        // A Windows drive prefix alone is relative; wait for the root separator.
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&cursor)
            .map_err(|error| format!("The item is missing or inaccessible: {error}"))?;
        if bloomsweepy_core::is_online_only_metadata(&metadata) {
            return Err("Online-only items cannot be opened without downloading them.".into());
        }
        let leaf = index + 1 == parts.len();
        let link = metadata.file_type().is_symlink() || is_reparse_point(&metadata);
        if !leaf && (link || !metadata.is_dir()) {
            return Err("A parent folder is a link or is no longer a regular folder.".into());
        }
        // Finder aliases look like regular files, including when renamed .pdf.
        // This is one bounded, no-follow metadata read, not a content inspection.
        let alias = leaf && metadata.is_file() && !link && finder_alias(&cursor)?;
        within_package |= executable_package(&cursor);
        if leaf {
            kind = if link || alias || within_package {
                Inspection::Reveal
            } else if metadata.is_dir() {
                Inspection::Directory
            } else if metadata.is_file() && !executable(&metadata) && document_extension(path) {
                Inspection::Document
            } else {
                Inspection::Reveal
            };
        }
        stamps.push(MetadataStamp::new(cursor.clone(), &metadata, alias));
    }
    Ok(InspectionPlan { kind, stamps })
}

fn revalidate(path: &str, original: &InspectionPlan) -> Result<(), String> {
    if &inspection_plan(path)? != original {
        return Err("The item or one of its parent folders changed. Please try again.".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn inspect_local_path(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
) -> Result<&'static str, String> {
    inspect_with_mode(window, app, path, false).await
}

#[tauri::command]
pub(crate) async fn reveal_local_path(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
) -> Result<(), String> {
    inspect_with_mode(window, app, path, true).await.map(|_| ())
}

async fn inspect_with_mode(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
    force_reveal: bool,
) -> Result<&'static str, String> {
    if window.label() != "main" {
        return Err("File inspection is available only in the main application window.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = INSPECTION_LOCK
            .try_lock()
            .map_err(|_| "Another item is being opened.")?;
        let plan = inspection_plan(&path)?;
        let dispatch = dispatch_plan(&plan, force_reveal)?;
        revalidate(&path, &plan)?;
        // Dispatch the same normalized lexical path that was inspected. A raw
        // trailing slash could otherwise cause the OS to dereference a leaf link.
        match dispatch {
            OsDispatch::RevealItem(path) => {
                app.opener()
                    .reveal_item_in_dir(path)
                    .map_err(|error| error.to_string())?;
            }
            OsDispatch::OpenDocument(path) => {
                let path = path
                    .to_str()
                    .ok_or("The local path cannot be represented.")?;
                app.opener()
                    .open_path(path, None::<&str>)
                    .map_err(|error| error.to_string())?;
            }
            OsDispatch::OpenDirectory(path) => {
                // Finder is fixed and trusted. Default opening could execute a
                // custom directory package not covered by the suffix list above.
                #[cfg(target_os = "macos")]
                let viewer = Some("/System/Library/CoreServices/Finder.app");
                #[cfg(not(target_os = "macos"))]
                let viewer = None::<&str>;
                let path = path
                    .to_str()
                    .ok_or("The local path cannot be represented.")?;
                app.opener()
                    .open_path(path, viewer)
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(if force_reveal || plan.kind == Inspection::Reveal {
            "revealed"
        } else {
            "opened"
        })
    })
    .await
    .map_err(|error| format!("File inspection could not finish: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        // /var is a macOS OS alias; test only the canonical, local fixture root.
        let canonical = temp.path().canonicalize().unwrap();
        let root = validate_path_text(canonical.to_str().unwrap())
            .unwrap()
            .to_path_buf();
        (temp, root)
    }

    fn decision(path: &Path) -> Inspection {
        inspection_plan(path.to_str().unwrap()).unwrap().kind
    }

    #[test]
    fn local_documents_and_normal_directories_are_inspectable_without_reading_content() {
        let (_temp, root) = fixture();
        for extension in DOCUMENT_EXTENSIONS {
            let path = root.join(format!("document.{extension}"));
            fs::write(&path, b"synthetic test content only").unwrap();
            assert_eq!(decision(&path), Inspection::Document, "{extension}");
        }
        assert_eq!(decision(&root), Inspection::Directory);
    }

    #[test]
    fn executable_and_ambiguous_formats_only_reveal() {
        let (_temp, root) = fixture();
        for name in [
            "README",
            ".pdf",
            "image.pdf.exe",
            "run.command",
            "run.bat",
            "run.lnk",
            "file.txt:payload",
        ] {
            #[cfg(windows)]
            if name.contains(':') {
                continue;
            }
            let path = root.join(name);
            fs::write(&path, b"synthetic").unwrap();
            assert_eq!(decision(&path), Inspection::Reveal, "{name}");
        }
        let app = root.join("Example.APP");
        fs::create_dir(&app).unwrap();
        let nested = app.join("manual.pdf");
        fs::write(&nested, b"synthetic").unwrap();
        assert_eq!(decision(&app), Inspection::Reveal);
        assert_eq!(decision(&nested), Inspection::Reveal);
    }

    #[test]
    fn invalid_cloud_and_missing_paths_do_not_reach_dispatch() {
        for path in [
            "",
            "relative.txt",
            "https://example.com/a.pdf",
            "file:///tmp/a.pdf",
            "//server/share/a.pdf",
            "\\\\server\\share\\a.pdf",
            "/tmp/../a.pdf",
            "/tmp/a\0.pdf",
        ] {
            assert!(validate_path_text(path).is_err(), "{path}");
        }
        let (_temp, root) = fixture();
        assert!(inspection_plan(root.join("missing.pdf").to_str().unwrap()).is_err());
        let cloud = root.join("Users/test/Library/CloudStorage/provider/absent.pdf");
        assert!(
            inspection_plan(cloud.to_str().unwrap())
                .unwrap_err()
                .contains("Cloud storage")
        );
    }

    #[test]
    fn changed_metadata_invalidates_the_plan() {
        let (_temp, root) = fixture();
        let file = root.join("note.txt");
        fs::write(&file, b"before").unwrap();
        let plan = inspection_plan(file.to_str().unwrap()).unwrap();
        fs::write(&file, b"changed length").unwrap();
        assert!(revalidate(file.to_str().unwrap(), &plan).is_err());
    }

    #[test]
    fn explicit_reveal_never_uses_the_document_opener() {
        let (_temp, root) = fixture();
        let file = root.join("note.pdf");
        fs::write(&file, b"synthetic").unwrap();
        let plan = inspection_plan(file.to_str().unwrap()).unwrap();
        assert_eq!(
            dispatch_plan(&plan, false).unwrap(),
            OsDispatch::OpenDocument(file.clone())
        );
        assert_eq!(
            dispatch_plan(&plan, true).unwrap(),
            OsDispatch::RevealItem(file)
        );
        let plan = inspection_plan(root.to_str().unwrap()).unwrap();
        assert_eq!(
            dispatch_plan(&plan, false).unwrap(),
            OsDispatch::OpenDirectory(root.clone())
        );
        assert_eq!(
            dispatch_plan(&plan, true).unwrap(),
            OsDispatch::RevealItem(root)
        );
    }

    #[cfg(unix)]
    #[test]
    fn dangling_and_cloud_target_links_dispatch_only_to_their_verified_local_parent() {
        use std::os::unix::fs::symlink;
        let (_temp, root) = fixture();
        for (name, target) in [
            ("dangling.pdf", root.join("missing.pdf")),
            (
                "cloud.pdf",
                root.join("Users/test/Library/CloudStorage/provider/private.pdf"),
            ),
        ] {
            let link = root.join(name);
            symlink(target, &link).unwrap();
            let plan = inspection_plan(link.to_str().unwrap()).unwrap();
            assert_eq!(plan.kind, Inspection::Reveal);
            for force_reveal in [false, true] {
                assert_eq!(
                    dispatch_plan(&plan, force_reveal).unwrap(),
                    OsDispatch::OpenDirectory(root.clone())
                );
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn finder_alias_renamed_as_pdf_dispatches_only_to_parent_and_invalidates_when_flags_change() {
        use std::ffi::{CString, c_char, c_int, c_void};
        use std::os::unix::ffi::OsStrExt;
        unsafe extern "C" {
            fn setxattr(
                path: *const c_char,
                name: *const c_char,
                value: *const c_void,
                size: usize,
                position: u32,
                options: c_int,
            ) -> c_int;
        }
        let (_temp, root) = fixture();
        let alias = root.join("renamed-alias.pdf");
        fs::write(
            &alias,
            b"synthetic alias metadata fixture; no alias content resolved",
        )
        .unwrap();
        let path = CString::new(alias.as_os_str().as_bytes()).unwrap();
        let mut info = [0_u8; 32];
        info[8] = 0x80; // Finder.h kIsAlias, big-endian FinderInfo flags.
        let set_flags = |info: &[u8; 32]| {
            let result = unsafe {
                setxattr(
                    path.as_ptr(),
                    c"com.apple.FinderInfo".as_ptr(),
                    info.as_ptr().cast(),
                    info.len(),
                    0,
                    1,
                )
            };
            assert_eq!(result, 0, "{}", std::io::Error::last_os_error());
        };
        set_flags(&info);
        let plan = inspection_plan(alias.to_str().unwrap()).unwrap();
        assert_eq!(plan.kind, Inspection::Reveal);
        assert!(plan.stamps.last().unwrap().is_finder_alias);
        assert_eq!(
            dispatch_plan(&plan, false).unwrap(),
            OsDispatch::OpenDirectory(root.clone())
        );
        assert_eq!(
            dispatch_plan(&plan, true).unwrap(),
            OsDispatch::OpenDirectory(root)
        );
        info[8] = 0;
        set_flags(&info);
        assert!(revalidate(alias.to_str().unwrap(), &plan).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn executable_bits_and_leaf_links_reveal_but_link_parents_reject() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let (_temp, root) = fixture();
        let file = root.join("script.txt");
        fs::write(&file, b"synthetic, never executed").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(decision(&file), Inspection::Reveal);
        let link = root.join("shortcut.pdf");
        symlink(&file, &link).unwrap();
        assert_eq!(decision(&link), Inspection::Reveal);
        let linked_root = root.join("alias");
        symlink(&root, &linked_root).unwrap();
        assert!(inspection_plan(linked_root.join("script.txt").to_str().unwrap()).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_canonical_local_disk_paths_share_the_checked_dispatch_path() {
        let (_temp, root) = fixture();
        let file = root.join("note.pdf");
        fs::write(&file, b"synthetic").unwrap();
        let canonical = file.canonicalize().unwrap();
        let plan = inspection_plan(canonical.to_str().unwrap()).unwrap();
        assert_eq!(plan.kind, Inspection::Document);
        assert_eq!(
            dispatch_plan(&plan, false).unwrap(),
            OsDispatch::OpenDocument(file.clone())
        );
        assert_eq!(
            dispatch_plan(&plan, true).unwrap(),
            OsDispatch::RevealItem(file)
        );
        revalidate(canonical.to_str().unwrap(), &plan).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_device_alternate_stream_and_alias_paths_are_rejected() {
        for path in [
            r"C:\Files\a.txt:payload",
            r"\\?\C:\Files\a.txt:payload",
            r"\\?\C:\Files\a.txt.",
            r"\\?\C:\Files\NUL.txt",
            r"\\?\C:\Files\..\a.txt",
            r"\\?\UNC\server\share\a.txt",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\a.txt",
            r"\\?\C:Files\a.txt",
            r"\\.\C:\Files\a.txt",
            r"C:\Files\a.txt.",
            r"C:\Files\a.txt ",
            r"C:Files\a.txt",
            r"C:\Files\NUL.txt",
            r"C:\Files\COM1.txt",
        ] {
            assert!(validate_path_text(path).is_err(), "{path}");
        }
    }
}
