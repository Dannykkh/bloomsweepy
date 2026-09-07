//! Demand-driven local DFS. No directory-wide collection, worker queue, or
//! background read-ahead: at most one prefetched entry per open directory.

use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, FileType, Metadata, ReadDir};
use std::io;
use std::path::{Path, PathBuf};

const MAX_OPEN_DIRECTORIES: usize = 128;
const MAX_RETAINED_PATH_BYTES: usize = 256 * 1024;
pub(crate) const MAX_WALK_PATH_BYTES: usize = 16 * 1024;

#[derive(Debug)]
pub(crate) struct WalkEntry {
    path: PathBuf,
    depth: usize,
    metadata: Metadata,
    empty_directory: bool,
}

impl WalkEntry {
    pub(crate) fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub(crate) fn depth(&self) -> usize {
        self.depth
    }

    pub(crate) fn file_name(&self) -> &OsStr {
        self.path.file_name().unwrap_or(self.path.as_os_str())
    }

    pub(crate) fn file_type(&self) -> FileType {
        self.metadata.file_type()
    }

    pub(crate) fn metadata(&self) -> io::Result<Metadata> {
        Ok(self.metadata.clone())
    }

    /// Determined from the unfiltered directory stream. A parent containing
    /// only excluded cloud entries, hidden entries, or errors is not empty.
    pub(crate) fn is_empty_directory(&self) -> bool {
        self.empty_directory
    }
}

#[derive(Debug)]
pub(crate) struct WalkError {
    path: PathBuf,
    source: io::Error,
    resource_limit: bool,
}

impl WalkError {
    pub(crate) fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    pub(crate) fn is_resource_limit(&self) -> bool {
        self.resource_limit
    }
}

fn is_resource_io_error(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::OutOfMemory {
        return true;
    }
    #[cfg(unix)]
    {
        matches!(
            error.raw_os_error(),
            Some(libc::ENOMEM | libc::EMFILE | libc::ENFILE)
        )
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{
            ERROR_COMMITMENT_LIMIT, ERROR_NO_SYSTEM_RESOURCES, ERROR_NOT_ENOUGH_MEMORY,
            ERROR_OUTOFMEMORY, ERROR_PAGEFILE_QUOTA, ERROR_TOO_MANY_OPEN_FILES,
            ERROR_WORKING_SET_QUOTA,
        };
        matches!(
            error
                .raw_os_error()
                .and_then(|code| u32::try_from(code).ok()),
            Some(
                ERROR_NOT_ENOUGH_MEMORY
                    | ERROR_OUTOFMEMORY
                    | ERROR_TOO_MANY_OPEN_FILES
                    | ERROR_NO_SYSTEM_RESOURCES
                    | ERROR_WORKING_SET_QUOTA
                    | ERROR_PAGEFILE_QUOTA
                    | ERROR_COMMITMENT_LIMIT
            )
        )
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

impl fmt::Display for WalkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.source)
    }
}

impl std::error::Error for WalkError {}

struct Frame {
    path: PathBuf,
    depth: usize,
    reader: ReadDir,
    first: Option<io::Result<fs::DirEntry>>,
}

pub(crate) struct StreamingWalk<'a, C: Fn() -> bool> {
    root: Option<PathBuf>,
    frames: Vec<Frame>,
    should_cancel: &'a C,
    retained_path_bytes: usize,
    excluded_entries: u64,
    finished: bool,
    max_open_directories: usize,
    max_retained_path_bytes: usize,
    #[cfg(test)]
    peak_open_directories: usize,
    #[cfg(test)]
    raw_entries_read: usize,
}

impl<'a, C: Fn() -> bool> StreamingWalk<'a, C> {
    pub(crate) fn new(root: &Path, should_cancel: &'a C) -> Self {
        Self {
            root: Some(root.to_path_buf()),
            frames: Vec::new(),
            should_cancel,
            retained_path_bytes: 0,
            excluded_entries: 0,
            finished: false,
            max_open_directories: MAX_OPEN_DIRECTORIES,
            max_retained_path_bytes: MAX_RETAINED_PATH_BYTES,
            #[cfg(test)]
            peak_open_directories: 0,
            #[cfg(test)]
            raw_entries_read: 0,
        }
    }

    pub(crate) fn excluded_entries(&self) -> u64 {
        self.excluded_entries
    }

    fn stop(&mut self) {
        self.finished = true;
        self.root = None;
        self.frames.clear();
        self.retained_path_bytes = 0;
    }

    fn limit(&mut self, path: PathBuf) -> WalkError {
        self.resource_error(
            path,
            "탐색의 깊이 또는 경로 메모리 안전 상한에 도달했습니다. 더 작은 폴더를 선택해 주세요"
                .to_owned(),
        )
    }

    fn resource_error(&mut self, path: PathBuf, message: String) -> WalkError {
        self.stop();
        WalkError {
            path,
            source: io::Error::other(message),
            resource_limit: true,
        }
    }

    fn access_error(&mut self, path: PathBuf, source: io::Error) -> WalkError {
        let resource_limit = is_resource_io_error(&source);
        if resource_limit {
            // Do not turn OS handle/memory exhaustion into an incomplete but
            // successful scan. Index consumers must abort and preserve old data.
            self.stop();
        }
        WalkError {
            path,
            source,
            resource_limit,
        }
    }

    fn visit(&mut self, path: PathBuf, depth: usize) -> Option<Result<WalkEntry, WalkError>> {
        let path_bytes = path.as_os_str().len();
        if path_bytes > MAX_WALK_PATH_BYTES {
            return Some(Err(self.limit(path)));
        }
        // Lexical rejection precedes metadata/read_dir to avoid hydrating a
        // known cloud root. Dataless/reparse metadata is checked before descent.
        if crate::scan_policy::is_cloud_path(&path) {
            self.excluded_entries = self.excluded_entries.saturating_add(1);
            return None;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => return Some(Err(self.access_error(path, error))),
        };
        if crate::scan_policy::is_online_only(&metadata)
            || crate::scan_policy::is_reparse_point(&metadata)
        {
            self.excluded_entries = self.excluded_entries.saturating_add(1);
            return None;
        }
        let mut empty_directory = false;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            if depth >= self.max_open_directories
                || self.frames.len() >= self.max_open_directories
                || self.retained_path_bytes.saturating_add(path_bytes)
                    > self.max_retained_path_bytes
            {
                return Some(Err(self.limit(path)));
            }
            if (self.should_cancel)() {
                self.stop();
                return Some(Err(self.access_error(
                    path,
                    io::Error::new(io::ErrorKind::Interrupted, "scan was cancelled"),
                )));
            }
            let mut reader = match fs::read_dir(&path) {
                Ok(reader) => reader,
                Err(error) => return Some(Err(self.access_error(path, error))),
            };
            let first = reader.next();
            #[cfg(test)]
            if first.is_some() {
                self.raw_entries_read += 1;
            }
            empty_directory = first.is_none();
            if !empty_directory {
                self.retained_path_bytes += path_bytes;
                self.frames.push(Frame {
                    path: path.clone(),
                    depth,
                    reader,
                    first,
                });
                #[cfg(test)]
                {
                    self.peak_open_directories = self.peak_open_directories.max(self.frames.len());
                }
            }
        }
        Some(Ok(WalkEntry {
            path,
            depth,
            metadata,
            empty_directory,
        }))
    }
}

impl<C: Fn() -> bool> Iterator for StreamingWalk<'_, C> {
    type Item = Result<WalkEntry, WalkError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.finished {
                return None;
            }
            if (self.should_cancel)() {
                let path = self
                    .root
                    .clone()
                    .or_else(|| self.frames.last().map(|f| f.path.clone()))
                    .unwrap_or_default();
                self.stop();
                return Some(Err(self.access_error(
                    path,
                    io::Error::new(io::ErrorKind::Interrupted, "scan was cancelled"),
                )));
            }
            if let Err(message) = crate::ensure_operation_memory() {
                let path = self
                    .root
                    .clone()
                    .or_else(|| self.frames.last().map(|frame| frame.path.clone()))
                    .unwrap_or_default();
                return Some(Err(self.resource_error(path, message)));
            }
            if let Some(root) = self.root.take() {
                if let Some(entry) = self.visit(root, 0) {
                    return Some(entry);
                }
                continue;
            }
            let frame = self.frames.last_mut()?;
            let next = if let Some(first) = frame.first.take() {
                Some(first)
            } else {
                let next = frame.reader.next();
                #[cfg(test)]
                if next.is_some() {
                    self.raw_entries_read += 1;
                }
                next
            };
            match next {
                Some(Ok(entry)) => {
                    let depth = frame.depth + 1;
                    if let Some(entry) = self.visit(entry.path(), depth) {
                        return Some(entry);
                    }
                }
                Some(Err(error)) => {
                    let path = frame.path.clone();
                    return Some(Err(self.access_error(path, error)));
                }
                None => {
                    let frame = self.frames.pop().expect("a completed frame exists");
                    self.retained_path_bytes -= frame.path.as_os_str().len();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn injected_os_resource_errors_abort_and_release_frames() {
        let mut errors = vec![io::Error::new(io::ErrorKind::OutOfMemory, "synthetic")];
        #[cfg(unix)]
        errors.extend([libc::ENOMEM, libc::EMFILE, libc::ENFILE].map(io::Error::from_raw_os_error));
        #[cfg(windows)]
        errors.extend([4, 8, 14, 1450, 1453, 1454, 1455].map(io::Error::from_raw_os_error));
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("file"), []).unwrap();
        for error in errors {
            let cancel = || false;
            let mut walk = StreamingWalk::new(temp.path(), &cancel);
            assert!(walk.next().unwrap().is_ok());
            walk.frames.last_mut().unwrap().first = Some(Err(error));
            assert!(walk.next().unwrap().unwrap_err().is_resource_limit());
            assert!(walk.frames.is_empty());
            assert_eq!(walk.retained_path_bytes, 0);
            assert!(walk.next().is_none());
        }
    }

    #[test]
    fn injected_permission_error_remains_a_nonfatal_issue() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("one"), []).unwrap();
        fs::write(temp.path().join("two"), []).unwrap();
        let cancel = || false;
        let mut walk = StreamingWalk::new(temp.path(), &cancel);
        assert!(walk.next().unwrap().is_ok());
        walk.frames.last_mut().unwrap().first = Some(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "synthetic",
        )));
        assert!(!walk.next().unwrap().unwrap_err().is_resource_limit());
        assert!(!walk.frames.is_empty());
        assert!(walk.next().unwrap().is_ok());
        assert!(walk.next().is_none());
    }

    #[test]
    fn wide_directory_is_consumed_on_demand_without_sibling_queue() {
        let temp = tempfile::tempdir().unwrap();
        for number in 0..2_048 {
            fs::write(temp.path().join(format!("file-{number}")), []).unwrap();
        }
        let cancel = || false;
        let mut walk = StreamingWalk::new(temp.path(), &cancel);
        assert_eq!(walk.next().unwrap().unwrap().depth(), 0);
        assert_eq!(
            walk.raw_entries_read, 1,
            "must not drain a directory at construction"
        );
        for _ in 0..8 {
            assert!(walk.next().unwrap().unwrap().file_type().is_file());
        }
        assert_eq!(walk.raw_entries_read, 8);
        assert_eq!(walk.frames.len(), 1);
        let remaining = walk.by_ref().count();
        assert_eq!(remaining, 2_040);
        assert_eq!(walk.peak_open_directories, 1);
        assert!(walk.frames.is_empty());
        assert_eq!(walk.retained_path_bytes, 0);
    }

    #[test]
    fn depth_and_path_budgets_stop_and_release_open_directories() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c/d/e")).unwrap();
        let cancel = || false;
        let mut walk = StreamingWalk::new(temp.path(), &cancel);
        walk.max_open_directories = 3;
        let errors: Vec<_> = walk.by_ref().filter_map(Result::err).collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].is_resource_limit());
        assert_eq!(walk.peak_open_directories, 3);
        assert!(walk.frames.is_empty());

        let mut walk = StreamingWalk::new(temp.path(), &cancel);
        walk.max_retained_path_bytes = temp.path().as_os_str().len();
        assert!(walk.next().unwrap().is_ok());
        assert!(walk.next().unwrap().unwrap_err().is_resource_limit());
        assert!(walk.frames.is_empty());
        assert_eq!(walk.retained_path_bytes, 0);
    }

    #[test]
    fn cancellation_drops_state_without_consuming_remaining_entries() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c")).unwrap();
        let cancelled = Cell::new(false);
        let check = || cancelled.get();
        let mut walk = StreamingWalk::new(temp.path(), &check);
        assert!(walk.next().unwrap().is_ok());
        let consumed = walk.raw_entries_read;
        cancelled.set(true);
        assert!(walk.next().unwrap().is_err());
        assert!(walk.next().is_none());
        assert_eq!(walk.raw_entries_read, consumed);
        assert!(walk.frames.is_empty());
    }

    #[test]
    fn cloud_only_parent_and_hidden_content_are_not_empty() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("Users/test/Library");
        fs::create_dir_all(library.join("CloudStorage/provider/deep")).unwrap();
        fs::create_dir(temp.path().join("empty")).unwrap();
        fs::create_dir(temp.path().join("hidden")).unwrap();
        fs::write(temp.path().join("hidden/.keep"), []).unwrap();
        let cancel = || false;
        let mut walk = StreamingWalk::new(temp.path(), &cancel);
        let entries: Vec<_> = walk.by_ref().map(Result::unwrap).collect();
        assert!(
            entries
                .iter()
                .all(|entry| !crate::scan_policy::is_cloud_path(&entry.path()))
        );
        assert_eq!(walk.excluded_entries(), 1);
        let empty: Vec<_> = entries
            .iter()
            .filter(|entry| entry.is_empty_directory())
            .map(|entry| entry.path())
            .collect();
        assert_eq!(empty, vec![temp.path().join("empty")]);
        assert!(
            !entries
                .iter()
                .find(|entry| entry.path() == library)
                .unwrap()
                .is_empty_directory()
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_directories_are_not_followed() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("private"), []).unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("link")).unwrap();
        let entries: Vec<_> = StreamingWalk::new(temp.path(), &|| false)
            .map(Result::unwrap)
            .collect();
        assert_eq!(entries.len(), 2);
        assert!(entries[1].file_type().is_symlink());
        assert!(!entries[1].is_empty_directory());
    }
}
