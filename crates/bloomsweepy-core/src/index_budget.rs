//! Conservative, per-connection index budgets. No process-global SQLite heap
//! limit is used: unrelated app databases must not be interrupted by an index.
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

const MIB: u64 = 1024 * 1024;
const RESOURCE_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub(crate) struct IndexBudget {
    pub(crate) max_database_bytes: u64,
    pub(crate) max_storage_bytes: u64,
    pub(crate) min_free_bytes: u64,
    #[cfg(test)]
    pub(crate) free_bytes: Option<Arc<std::sync::atomic::AtomicU64>>,
}

impl Default for IndexBudget {
    fn default() -> Self {
        Self {
            max_database_bytes: 256 * MIB,
            max_storage_bytes: 512 * MIB,
            min_free_bytes: 2 * 1024 * MIB,
            #[cfg(test)]
            free_bytes: None,
        }
    }
}

#[derive(Clone)]
enum StopReason {
    Cancelled,
    Resource(String),
}

pub(crate) struct BuildGuard {
    path: PathBuf,
    budget: IndexBudget,
    stopped: Arc<AtomicBool>,
    reason: Mutex<Option<StopReason>>,
}

impl BuildGuard {
    fn stop(&self, reason: StopReason) {
        let mut stored = self
            .reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if stored.is_none() {
            *stored = Some(reason);
            self.stopped.store(true, Ordering::Release);
        }
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    /// Also called immediately before COMMIT, so a quick build cannot skip the
    /// watchdog's first sample. Page limits are strict; sidecar/free-space limits
    /// are sampled, not an OS quota or a promise about process RSS.
    pub(crate) fn check_now(&self) -> Result<(), String> {
        let result = self
            .budget
            .check_storage(&self.path)
            .and_then(|()| crate::ensure_operation_memory());
        if let Err(message) = &result {
            self.stop(StopReason::Resource(message.clone()));
        }
        result
    }

    pub(crate) fn configure(&self, connection: &Connection) -> Result<(), String> {
        self.check_now()?;
        let page_size: i64 = connection
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        let page_size = u64::try_from(page_size)
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| "색인 페이지 크기가 올바르지 않아 작업을 중단했습니다".to_owned())?;
        let pages = (self.budget.max_database_bytes / page_size).max(1);
        let accepted: i64 = connection
            .query_row(&format!("PRAGMA max_page_count = {pages}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        let accepted = u64::try_from(accepted)
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| "색인 페이지 한도를 확인하지 못해 작업을 중단했습니다".to_owned())?;
        // SQLite does not shrink an existing database to satisfy max_page_count.
        // Refuse the rebuild rather than delete or truncate an existing index.
        if accepted > pages {
            return Err(
                "기존 색인이 저장 한도보다 커서 갱신을 중단했습니다. 기존 검색 자료는 보존됩니다"
                    .to_owned(),
            );
        }
        let stopped = Arc::clone(&self.stopped);
        connection
            .progress_handler(1_000, Some(move || stopped.load(Ordering::Acquire)))
            .map_err(|error| error.to_string())
    }
}

impl IndexBudget {
    fn check_storage(&self, path: &Path) -> Result<(), String> {
        let database_bytes = file_bytes(path)?;
        let mut storage_bytes = database_bytes;
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = path.as_os_str().to_os_string();
            sidecar.push(suffix);
            let bytes = file_bytes(Path::new(&sidecar))?;
            if suffix == "-wal" && bytes > self.max_database_bytes {
                return Err(
                    "색인 임시 기록(WAL)이 저장 한도에 도달해 중단했습니다. 기존 색인은 보존됩니다"
                        .to_owned(),
                );
            }
            storage_bytes = storage_bytes.saturating_add(bytes);
        }
        if database_bytes > self.max_database_bytes || storage_bytes > self.max_storage_bytes {
            return Err(
                "색인 저장 용량 한도에 도달해 중단했습니다. 기존 색인은 보존됩니다".to_owned(),
            );
        }
        for location in [path.to_path_buf(), std::env::temp_dir()] {
            #[cfg(test)]
            let available = match &self.free_bytes {
                Some(bytes) => bytes.load(Ordering::Acquire),
                None => available_bytes(&location)?,
            };
            #[cfg(not(test))]
            let available = available_bytes(&location)?;
            if available < self.min_free_bytes {
                return Err(format!(
                    "색인 작업에 필요한 여유 공간이 부족합니다 (최소 {} MiB 확보 필요). 기존 색인은 보존됩니다",
                    self.min_free_bytes / MIB
                ));
            }
        }
        Ok(())
    }
}

/// The scoped thread observes cancellation even inside a long SQL index/FTS
/// statement. It owns no connection or file contents and exits before return.
pub(crate) fn run_guarded<T, E, C>(
    path: &Path,
    budget: IndexBudget,
    should_cancel: &C,
    cancelled: impl Fn() -> E,
    resource_error: impl Fn(String) -> E,
    operation: impl FnOnce(&BuildGuard) -> Result<T, E>,
) -> Result<T, E>
where
    C: Fn() -> bool + Sync,
{
    if should_cancel() {
        return Err(cancelled());
    }
    let guard = BuildGuard {
        path: path.to_path_buf(),
        budget,
        stopped: Arc::new(AtomicBool::new(false)),
        reason: Mutex::new(None),
    };
    guard.check_now().map_err(&resource_error)?;
    let result = std::thread::scope(|scope| {
        let (finished, wait) = mpsc::channel::<()>();
        let guard_ref = &guard;
        scope.spawn(move || {
            while matches!(
                wait.recv_timeout(RESOURCE_POLL_INTERVAL),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                if should_cancel() {
                    guard_ref.stop(StopReason::Cancelled);
                    break;
                }
                if guard_ref.check_now().is_err() {
                    break;
                }
            }
        });
        let result = operation(&guard);
        drop(finished);
        result
    });
    // A successful COMMIT is authoritative. A cancellation arriving during the
    // optional post-commit checkpoint must not claim that it was rolled back.
    result.map_err(|error| {
        match guard
            .reason
            .into_inner()
            .unwrap_or_else(|error| error.into_inner())
        {
            Some(StopReason::Cancelled) => cancelled(),
            Some(StopReason::Resource(message)) => resource_error(message),
            None => error,
        }
    })
}

pub(crate) fn configure_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "PRAGMA temp_store = FILE;
         PRAGMA cache_size = -4096;
         PRAGMA temp.cache_size = -1024;
         PRAGMA temp.max_page_count = 16384;
         PRAGMA mmap_size = 0;
         PRAGMA threads = 0;
         PRAGMA cache_spill = ON;
         PRAGMA journal_size_limit = 0;
         PRAGMA wal_autocheckpoint = 256;",
    )
}

pub(crate) fn checkpoint_after_commit(connection: &Connection) {
    // TRUNCATE never truncates the main DB. A busy reader may retain the WAL;
    // the next build checks the retained bytes instead of growing indefinitely.
    let _ = connection.progress_handler(0, None::<fn() -> bool>);
    let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
}

pub(crate) fn sqlite_error_message(error: &rusqlite::Error) -> String {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DiskFull) => {
            "색인 저장 한도 또는 실제 디스크 여유 공간이 부족해 작업을 중단했습니다".to_owned()
        }
        Some(rusqlite::ErrorCode::OutOfMemory) => {
            "색인 작업에 필요한 메모리가 부족해 중단했습니다".to_owned()
        }
        _ => error.to_string(),
    }
}

fn file_bytes(path: &Path) -> Result<u64, String> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(format!(
            "색인 저장 공간을 확인하지 못해 중단했습니다: {error}"
        )),
    }
}

fn existing_location(path: &Path) -> Result<PathBuf, String> {
    let mut current = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    while !current.exists() {
        if !current.pop() {
            return Err("색인 저장장치의 여유 공간을 확인하지 못했습니다".to_owned());
        }
    }
    Ok(current)
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // statvfs integer widths differ across Unix targets.
fn available_bytes(path: &Path) -> Result<u64, String> {
    use std::os::unix::ffi::OsStrExt;
    let location = existing_location(path)?;
    let location = std::ffi::CString::new(location.as_os_str().as_bytes())
        .map_err(|error| error.to_string())?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: the path is NUL terminated and stats points to writable storage.
    if unsafe { libc::statvfs(location.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return Err(format!(
            "여유 공간을 확인하지 못했습니다: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: a successful statvfs call initialized stats.
    let stats = unsafe { stats.assume_init() };
    Ok((stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64))
}

#[cfg(windows)]
fn available_bytes(path: &Path) -> Result<u64, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let location = existing_location(path)?;
    let location = if location.is_dir() {
        location.as_path()
    } else {
        location.parent().unwrap_or(&location)
    };
    let wide: Vec<u16> = location.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0_u64;
    // SAFETY: wide is NUL terminated and available points to writable u64 storage.
    if unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(format!(
            "여유 공간을 확인하지 못했습니다: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(available)
}

#[cfg(not(any(unix, windows)))]
fn available_bytes(_path: &Path) -> Result<u64, String> {
    Err("이 운영체제에서는 색인 저장 공간을 확인할 수 없습니다".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn low_free_space_and_existing_over_budget_indexes_are_rejected_without_mutation() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("index.sqlite3");
        fs::write(&path, b"preserve existing bytes").unwrap();
        let mut budget = IndexBudget {
            free_bytes: Some(Arc::new(AtomicU64::new(0))),
            ..IndexBudget::default()
        };
        assert!(
            budget
                .check_storage(&path)
                .unwrap_err()
                .contains("여유 공간")
        );
        budget
            .free_bytes
            .as_ref()
            .unwrap()
            .store(u64::MAX, Ordering::Release);
        budget.max_database_bytes = 1;
        assert!(
            budget
                .check_storage(&path)
                .unwrap_err()
                .contains("저장 용량")
        );
        assert_eq!(fs::read(&path).unwrap(), b"preserve existing bytes");
    }

    #[test]
    fn connection_uses_bounded_disk_backed_temp_and_page_cache() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        let read = |name: &str| {
            connection
                .query_row(&format!("PRAGMA {name}"), [], |row| row.get::<_, i64>(0))
                .unwrap()
        };
        assert_eq!(read("temp_store"), 1);
        assert_eq!(read("cache_size"), -4096);
        assert_eq!(read("temp.cache_size"), -1024);
        assert_eq!(read("temp.max_page_count"), 16384);
        assert_eq!(read("threads"), 0);
    }

    #[test]
    fn sqlite_page_cap_rolls_back_and_does_not_destroy_the_previous_index() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("index.sqlite3");
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE entries(value BLOB); INSERT INTO entries VALUES('stable');").unwrap();
        }
        let budget = IndexBudget {
            max_database_bytes: 64 * 1024,
            max_storage_bytes: 128 * 1024,
            min_free_bytes: 1,
            free_bytes: Some(Arc::new(AtomicU64::new(u64::MAX))),
        };
        let attempt = |bytes: u64| {
            run_guarded(
                &path,
                budget.clone(),
                &|| false,
                || "cancelled".to_owned(),
                |message| message,
                |guard| {
                    let mut connection =
                        Connection::open(&path).map_err(|error| error.to_string())?;
                    configure_connection(&connection).map_err(|error| error.to_string())?;
                    guard.configure(&connection)?;
                    let transaction = connection
                        .transaction()
                        .map_err(|error| error.to_string())?;
                    transaction
                        .execute("DELETE FROM entries", [])
                        .map_err(|error| error.to_string())?;
                    let bytes = i64::try_from(bytes).map_err(|error| error.to_string())?;
                    transaction
                        .execute("INSERT INTO entries VALUES(zeroblob(?1))", [bytes])
                        .map_err(|error| error.to_string())?;
                    guard.check_now()?;
                    transaction.commit().map_err(|error| error.to_string())?;
                    checkpoint_after_commit(&connection);
                    Ok::<_, String>(())
                },
            )
        };
        assert!(attempt(512 * 1024).is_err());
        {
            let connection = Connection::open(&path).unwrap();
            let previous: String = connection
                .query_row("SELECT value FROM entries", [], |row| row.get(0))
                .unwrap();
            assert_eq!(previous, "stable");
        }
        attempt(1024).unwrap();
        assert!(fs::metadata(&path).unwrap().len() <= budget.max_database_bytes);
    }

    #[test]
    fn watchdog_interrupts_long_sql_on_cancel_and_on_low_disk() {
        for cancel_request in [true, false] {
            let fixture = tempfile::tempdir().unwrap();
            let path = fixture.path().join("index.sqlite3");
            let cancel = AtomicBool::new(false);
            let free_bytes = Arc::new(AtomicU64::new(u64::MAX));
            let budget = IndexBudget {
                min_free_bytes: 1,
                free_bytes: Some(Arc::clone(&free_bytes)),
                ..IndexBudget::default()
            };
            let started = std::time::Instant::now();
            let result = run_guarded(
                &path,
                budget,
                &|| cancel.load(Ordering::Acquire),
                || "cancelled".to_owned(),
                |message| message,
                |guard| {
                    let connection = Connection::open(&path).map_err(|error| error.to_string())?;
                    configure_connection(&connection).map_err(|error| error.to_string())?;
                    guard.configure(&connection)?;
                    if cancel_request {
                        cancel.store(true, Ordering::Release);
                    } else {
                        free_bytes.store(0, Ordering::Release);
                    }
                    connection.query_row(
                    "WITH RECURSIVE counter(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM counter WHERE n<1000000000) SELECT SUM(n) FROM counter",
                    [], |row| row.get::<_, i64>(0),
                ).map_err(|error| error.to_string())
                },
            );
            let error = result.unwrap_err();
            if cancel_request {
                assert_eq!(error, "cancelled");
            } else {
                assert!(error.contains("여유 공간"));
            }
            assert!(started.elapsed() < Duration::from_secs(3));
        }
    }
}
