//! Local-only scanning: prune cloud roots before opening their directory stream.
//! This is independent of the dashboard's mounted-volume visibility filter.

use std::fs;
#[cfg(any(target_os = "macos", windows))]
use std::io;
use std::path::Path;

use crate::ScanError;

pub(crate) const CLOUD_EXCLUDED_MESSAGE: &str =
    "클라우드 동기화 폴더와 온라인 전용 항목은 검사하지 않습니다. 로컬 폴더를 선택해 주세요";

pub(crate) fn is_cloud_path(path: &Path) -> bool {
    let raw = path.to_string_lossy();
    let windows_path = raw.starts_with("\\\\") || raw.as_bytes().get(1) == Some(&b':');
    let normalized = if windows_path {
        raw.replace('\\', "/").to_lowercase()
    } else {
        raw.to_lowercase()
    };
    let parts: Vec<_> = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();

    for (index, part) in parts.iter().enumerate() {
        // All File Provider accounts, including providers we do not know by name.
        // Also works through the /System/Volumes/Data alias and relocated homes.
        if index > 0
            && parts[index - 1] == "library"
            && matches!(*part, "cloudstorage" | "mobile documents")
        {
            return true;
        }
        // Older clients keep sync roots directly in a user's home or /Volumes.
        let home_child = index >= 2 && matches!(parts[index - 2], "users" | "home");
        let mounted_volume = index > 0 && parts[index - 1] == "volumes";
        if (home_child || mounted_volume) && is_provider_root_name(part) {
            return true;
        }
    }
    false
}

fn is_provider_root_name(name: &str) -> bool {
    [
        "google drive",
        "googledrive",
        "googledrivefs",
        "drivefs",
        "onedrive",
        "dropbox",
        "icloud drive",
        "iclouddrive",
        "pcloud",
        "pcloud drive",
        "box",
        "box sync",
    ]
    .iter()
    .any(|provider| {
        name == *provider
            || name.strip_prefix(provider).is_some_and(|suffix| {
                (matches!(*provider, "googledrive" | "onedrive") && suffix.starts_with('-'))
                    || suffix.starts_with(" - ")
                    || (suffix.starts_with(" (") && suffix.ends_with(')'))
            })
    }) || (name.contains('@') && name.contains(" - google drive"))
}

/// Lexical check comes first: do not stat or canonicalize a known cloud path.
pub(crate) fn ensure_local_path(path: &Path) -> Result<(), String> {
    if is_cloud_path(path)
        || fs::symlink_metadata(path).is_ok_and(|metadata| is_online_only(&metadata))
    {
        return Err(CLOUD_EXCLUDED_MESSAGE.to_owned());
    }
    Ok(())
}

/// Reject UNC paths lexically before inspecting any filesystem metadata.
pub(crate) fn reject_network_path(path: &Path) -> Result<(), ScanError> {
    let raw = path.as_os_str().to_string_lossy();
    if (raw.starts_with("\\\\") && !raw.starts_with("\\\\?\\"))
        || raw.to_ascii_lowercase().starts_with("\\\\?\\unc\\")
    {
        return Err(ScanError::Access(
            "네트워크 폴더는 로컬 검색 대상으로 사용할 수 없습니다".into(),
        ));
    }
    Ok(())
}

/// Read mount metadata only; never open or hydrate the directory's contents.
#[cfg(target_os = "macos")]
pub(crate) fn ensure_local_volume(path: &Path) -> Result<(), ScanError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| ScanError::Access("폴더 경로에 잘못된 문자가 포함되어 있습니다".into()))?;
    let mut information = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::statfs(path.as_ptr(), information.as_mut_ptr()) } != 0 {
        return Err(ScanError::Access(io::Error::last_os_error().to_string()));
    }
    let information = unsafe { information.assume_init() };
    if information.f_flags & libc::MNT_LOCAL as u32 == 0 {
        return Err(ScanError::Access(
            "네트워크 저장장치는 로컬 검색 대상으로 사용할 수 없습니다".into(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn ensure_local_volume(path: &Path) -> Result<(), ScanError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut volume = vec![0_u16; 32_768];
    if unsafe { GetVolumePathNameW(wide.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) } == 0 {
        return Err(ScanError::Access(io::Error::last_os_error().to_string()));
    }
    let drive_type = unsafe { GetDriveTypeW(volume.as_ptr()) };
    // DRIVE_REMOVABLE, DRIVE_FIXED, DRIVE_CDROM, DRIVE_RAMDISK.
    if !matches!(drive_type, 2 | 3 | 5 | 6) {
        return Err(ScanError::Access(
            "네트워크 저장장치는 로컬 검색 대상으로 사용할 수 없습니다".into(),
        ));
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
pub(crate) fn ensure_local_volume(_path: &Path) -> Result<(), ScanError> {
    // Match the existing metadata-only snapshot platform support.
    Ok(())
}

pub(crate) fn is_online_only(metadata: &fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        // Darwin sys/stat.h: SF_DATALESS. Never infer this from allocated size:
        // ordinary sparse local files remain valid scan targets.
        metadata.st_flags() & 0x4000_0000 != 0
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS.
        metadata.file_attributes() & (0x1000 | 0x40000 | 0x400000) != 0
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = metadata;
        false
    }
}

pub(crate) fn is_reparse_point(metadata: &fs::Metadata) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unc_roots_are_rejected_before_filesystem_access() {
        assert!(reject_network_path(Path::new("\\\\server\\share")).is_err());
        assert!(reject_network_path(Path::new("\\\\?\\UNC\\server\\share")).is_err());
        assert!(reject_network_path(Path::new("\\\\?\\C:\\local")).is_ok());
    }

    #[test]
    fn recognizes_cloud_roots_and_their_descendants_across_platform_paths() {
        for path in [
            "/Users/a/Library/CloudStorage",
            "/Users/b/Library/CloudStorage/GoogleDrive-test@example.com/My Drive/a.txt",
            "/System/Volumes/Data/Users/a/Library/Mobile Documents/com~apple~CloudDocs",
            "/custom/home/Library/CloudStorage/NewProvider/a.txt",
            "/Users/a/Google Drive/a.txt",
            "/Users/a/OneDrive - Company/a.txt",
            "/Users/a/Dropbox (Personal)/a.txt",
            "/Volumes/GoogleDrive-test@example.com/a.txt",
            "/Volumes/test@example.com - Google Drive/a.txt",
            "C:\\Users\\a\\OneDrive\\a.txt",
            "\\\\?\\C:\\Users\\a\\Dropbox\\a.txt",
        ] {
            assert!(is_cloud_path(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn does_not_exclude_similarly_named_local_paths() {
        for path in [
            "/Users/a/Documents/report.txt",
            "/Users/a/Library/CloudStorage-backup/report.txt",
            "/Users/a/Library/Mobile Documents backup/report.txt",
            "/Users/a/Documents/Google Drive guide/report.txt",
            "/Users/a/Dropbox-backup-notes.txt",
            "/Volumes/Google archive/report.txt",
            "C:\\Users\\a\\Documents\\OneDrive tutorial.txt",
        ] {
            assert!(!is_cloud_path(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn cloud_directories_are_never_scheduled_for_read_dir() {
        let temp = tempfile::tempdir().unwrap();
        let cloud = temp.path().join("Users/test/Library/CloudStorage/provider");
        fs::create_dir_all(&cloud).unwrap();
        fs::write(cloud.join("private.txt"), b"must not be read").unwrap();
        let entries: Vec<_> =
            crate::streaming_walk::StreamingWalk::new(temp.path(), &|| false).collect();
        assert!(
            entries
                .iter()
                .all(|entry| !is_cloud_path(&entry.as_ref().unwrap().path()))
        );
        assert_eq!(
            crate::open_read_shared(&cloud.join("private.txt"))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::PermissionDenied,
        );
    }
}
