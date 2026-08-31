//! Read-only NTFS catalogue source for Windows.
//!
//! The raw MFT reader is derived from the MIT-licensed AllTheThings project
//! (Copyright (c) 2026 Swatto). BroomSweepy adds bounded subtree resolution,
//! SQLite-oriented records, journal checkpoints, cancellation, and safe
//! portable-walk fallback. The upstream notice is preserved in
//! `apps/desktop/THIRD_PARTY_NOTICES.md`.

use super::{CatalogRecord, CatalogRecordSink, FileCatalogEntryKind};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::mem::{size_of, zeroed};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::ptr;

use thiserror::Error;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, GetFileInformationByHandle, GetVolumeInformationW, OPEN_EXISTING, ReadFile,
    SetFilePointerEx,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const GENERIC_READ: u32 = 0x8000_0000;
const FILE_BEGIN: u32 = 0;
const ROOT_MFT_RECORD: u64 = 5;
const RECORD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
const WINDOWS_TO_UNIX_EPOCH_100NS: u64 = 116_444_736_000_000_000;
const MAX_MFT_RECORDS: u64 = 100_000_000;
const MAX_DIRECTORY_RECORDS: usize = 5_000_000;
const MAX_JOURNAL_CHANGE_RECORDS: usize = 250_000;
const MAX_DIRECTORY_DEPTH: usize = 1_024;

const FSCTL_QUERY_USN_JOURNAL: u32 = 0x0009_00F4;
const FSCTL_READ_USN_JOURNAL: u32 = 0x0009_00BB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NtfsCheckpoint {
    pub volume_serial: u32,
    pub journal_id: u64,
    pub next_usn: i64,
    pub root_record_id: u64,
}

#[derive(Debug, Clone)]
pub(super) struct JournalChange {
    pub record_id: u64,
    pub parent_ids: Vec<u64>,
    pub directory_hint: bool,
}

#[derive(Debug)]
pub(super) enum JournalDelta {
    Changes {
        changes: Vec<JournalChange>,
        checkpoint: NtfsCheckpoint,
    },
    FullRequired(String),
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct NtfsScanStats {
    pub scanned_records: u64,
    pub malformed_records: u64,
}

#[derive(Debug)]
pub(super) enum ChangedRecordsOutcome {
    Applied(NtfsScanStats),
    FullRequired(String),
}

#[derive(Debug)]
pub(super) enum NtfsAvailability {
    Ready(NtfsSource),
    Unavailable(String),
}

#[derive(Debug, Error)]
pub(super) enum NtfsError {
    #[error("Windows 빠른 파일 읽기를 취소했습니다")]
    Cancelled,
    #[error("다음 드라이브를 빠르게 읽을 권한이 없습니다: {0}")]
    AccessDenied(String),
    #[error("드라이브 {volume}을 열지 못했습니다: Windows 오류 {code}")]
    Open { volume: String, code: u32 },
    #[error("드라이브 {volume}의 파일 정보를 읽지 못했습니다 (위치 {offset}, Windows 오류 {code})")]
    Read {
        volume: String,
        offset: u64,
        code: u32,
    },
    #[error("빠른 읽기를 지원하지 않는 드라이브입니다: {0}")]
    NotNtfs(String),
    #[error("Windows 파일 목록을 읽는 중 확인할 수 없는 정보가 있습니다 ({volume}: {detail})")]
    Malformed { volume: String, detail: String },
    #[error("폴더 수가 안전 한도를 넘어 빠른 읽기를 중단했습니다")]
    DirectoryLimit,
    #[error("읽은 파일 정보를 검색 목록에 저장하지 못했습니다: {0}")]
    Sink(String),
}

pub(super) struct NtfsSource {
    reader: MftReader,
    drive: char,
    root: PathBuf,
    root_key: String,
    root_record_id: u64,
    volume_serial: u32,
    initial_journal: Option<JournalState>,
}

impl std::fmt::Debug for NtfsSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NtfsSource")
            .field("drive", &self.drive)
            .field("root", &self.root)
            .field("root_record_id", &self.root_record_id)
            .field("volume_serial", &self.volume_serial)
            .finish_non_exhaustive()
    }
}

impl NtfsSource {
    pub(super) fn try_open(root: &Path) -> NtfsAvailability {
        let Some(drive) = drive_letter(root) else {
            return NtfsAvailability::Unavailable(
                "빠른 드라이브 읽기를 사용할 수 없는 위치라 일반 방식으로 확인합니다".to_owned(),
            );
        };
        if path_key(root) != format!("{}:", drive.to_ascii_lowercase()) {
            return NtfsAvailability::Unavailable(
                "선택한 폴더는 일반 방식으로 확인하는 편이 더 작고 안전합니다".to_owned(),
            );
        }
        if !is_ntfs(drive) {
            return NtfsAvailability::Unavailable(format!(
                "{drive}: 드라이브에서는 빠른 읽기를 사용할 수 없어 일반 방식으로 확인합니다"
            ));
        }
        let Some((volume_serial, root_record_id)) = path_identity(root) else {
            return NtfsAvailability::Unavailable(
                "선택한 위치의 파일 정보를 빠르게 읽을 수 없어 일반 방식으로 확인합니다".to_owned(),
            );
        };
        let reader = match MftReader::open(drive) {
            Ok(reader) => reader,
            Err(error) => {
                return NtfsAvailability::Unavailable(format!(
                    "Windows 빠른 읽기를 시작하지 못해 일반 방식으로 바꿨습니다: {error}"
                ));
            }
        };
        let initial_journal = query_journal(reader.volume()).ok();
        NtfsAvailability::Ready(Self {
            reader,
            drive,
            root: root.to_path_buf(),
            root_key: path_key(root),
            root_record_id: root_record_id & RECORD_MASK,
            volume_serial,
            initial_journal,
        })
    }

    pub(super) fn checkpoint(&self) -> Option<NtfsCheckpoint> {
        self.initial_journal.map(|journal| NtfsCheckpoint {
            volume_serial: self.volume_serial,
            journal_id: journal.journal_id,
            next_usn: journal.next_usn,
            root_record_id: self.root_record_id,
        })
    }

    pub(super) fn volume_serial(&self) -> u32 {
        self.volume_serial
    }

    pub(super) fn root_record_id(&self) -> u64 {
        self.root_record_id
    }

    #[cfg(test)]
    pub(super) fn record_count(&self) -> u64 {
        self.reader.record_count
    }

    pub(super) fn read_journal_delta(
        &self,
        previous: NtfsCheckpoint,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<JournalDelta, NtfsError> {
        if previous.volume_serial != self.volume_serial
            || previous.root_record_id != self.root_record_id
        {
            return Ok(JournalDelta::FullRequired(
                "드라이브나 선택 위치의 정보가 바뀌어 전체를 다시 읽습니다".to_owned(),
            ));
        }

        let state = match query_journal(self.reader.volume()) {
            Ok(state) => state,
            Err(code) => {
                return Ok(JournalDelta::FullRequired(format!(
                    "Windows 변경 기록을 읽지 못해 드라이브 전체를 다시 확인합니다: Windows 오류 {code}"
                )));
            }
        };
        if state.journal_id != previous.journal_id
            || previous.next_usn < state.lowest_valid_usn
            || previous.next_usn > state.next_usn
        {
            return Ok(JournalDelta::FullRequired(
                "Windows 변경 기록이 새로 만들어졌거나 이전 기록이 없어 드라이브 전체를 다시 확인합니다"
                    .to_owned(),
            ));
        }

        let mut cursor = previous.next_usn;
        let target = state.next_usn;
        let mut changes = HashMap::<u64, JournalChange>::new();
        let mut out = vec![0_u8; 1024 * 1024];

        while cursor < target {
            if should_cancel() {
                return Err(NtfsError::Cancelled);
            }
            let request = ReadUsnJournalDataV0 {
                start_usn: cursor,
                reason_mask: u32::MAX,
                return_only_on_close: 0,
                timeout: 0,
                bytes_to_wait_for: 0,
                usn_journal_id: state.journal_id,
            };
            let returned = unsafe {
                self.reader.volume().device_io_control(
                    FSCTL_READ_USN_JOURNAL,
                    &request as *const _ as *const c_void,
                    size_of::<ReadUsnJournalDataV0>() as u32,
                    out.as_mut_ptr() as *mut c_void,
                    out.len() as u32,
                )
            };
            let returned = match returned {
                Ok(value) => value as usize,
                Err(code) => {
                    return Ok(JournalDelta::FullRequired(format!(
                        "Windows 변경 기록을 읽지 못해 드라이브 전체를 다시 확인합니다: Windows 오류 {code}"
                    )));
                }
            };
            if returned < 8 {
                return Ok(JournalDelta::FullRequired(
                    "Windows 변경 기록이 완전하지 않아 드라이브 전체를 다시 확인합니다".to_owned(),
                ));
            }
            let next = i64::from_le_bytes(out[0..8].try_into().unwrap_or_default());
            if next <= cursor {
                return Ok(JournalDelta::FullRequired(
                    "Windows 변경 기록의 위치를 확인할 수 없어 드라이브 전체를 다시 확인합니다"
                        .to_owned(),
                ));
            }
            parse_journal_records(&out[..returned], &mut changes)?;
            if changes.len() > MAX_JOURNAL_CHANGE_RECORDS {
                return Ok(JournalDelta::FullRequired(
                    "바뀐 파일이 너무 많아 드라이브 전체를 다시 확인합니다".to_owned(),
                ));
            }
            cursor = next.min(target);
        }

        Ok(JournalDelta::Changes {
            changes: changes.into_values().collect(),
            checkpoint: NtfsCheckpoint {
                volume_serial: self.volume_serial,
                journal_id: state.journal_id,
                next_usn: target,
                root_record_id: self.root_record_id,
            },
        })
    }

    pub(super) fn enumerate_full(
        &self,
        excluded_paths: &[PathBuf],
        sink: &mut dyn CatalogRecordSink,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<NtfsScanStats, NtfsError> {
        let mut directories = HashMap::<u64, DirectoryLink>::new();
        let first_pass = self.reader.enumerate(
            |record| {
                if record.is_dir && record.record_no != ROOT_MFT_RECORD {
                    directories
                        .entry(record.record_no)
                        .or_insert(DirectoryLink {
                            parent_id: record.parent_no,
                            name: record.name,
                        });
                }
                directories.len() <= MAX_DIRECTORY_RECORDS
            },
            |_| {},
            should_cancel,
        )?;
        if directories.len() > MAX_DIRECTORY_RECORDS {
            return Err(NtfsError::DirectoryLimit);
        }

        let volume_root = PathBuf::from(format!("{}:\\", self.drive));
        let mut resolved = HashMap::<u64, PathBuf>::new();
        resolved.insert(ROOT_MFT_RECORD, volume_root);
        resolved.insert(self.root_record_id, self.root.clone());
        let mut membership = HashMap::<u64, bool>::new();
        membership.insert(self.root_record_id, true);
        if self.root_record_id != ROOT_MFT_RECORD {
            membership.insert(ROOT_MFT_RECORD, false);
        }

        let mut sink_error = None;
        let second_pass = self.reader.enumerate(
            |record| {
                if record.record_no == self.root_record_id {
                    return true;
                }
                if !is_descendant(
                    record.parent_no,
                    self.root_record_id,
                    &directories,
                    &mut membership,
                ) {
                    return true;
                }
                let Some(parent) = resolve_directory(record.parent_no, &directories, &mut resolved)
                else {
                    return true;
                };
                let path = parent.join(&record.name);
                if !is_within_key(&path_key(&path), &self.root_key)
                    || is_excluded(&path, excluded_paths)
                {
                    return true;
                }
                let catalog_record = raw_to_catalog(record, path, parent);
                match sink.push(catalog_record) {
                    Ok(keep_going) => keep_going,
                    Err(error) => {
                        sink_error = Some(error);
                        false
                    }
                }
            },
            |_| {},
            should_cancel,
        )?;
        if let Some(error) = sink_error {
            return Err(NtfsError::Sink(error));
        }
        sink.set_scanned_entries(second_pass.scanned_records);

        Ok(NtfsScanStats {
            scanned_records: second_pass.scanned_records,
            malformed_records: first_pass
                .malformed_records
                .saturating_add(second_pass.malformed_records),
        })
    }

    pub(super) fn enumerate_changed(
        &self,
        changes: &[JournalChange],
        directory_paths: &HashMap<u64, PathBuf>,
        excluded_paths: &[PathBuf],
        sink: &mut dyn CatalogRecordSink,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<ChangedRecordsOutcome, NtfsError> {
        let mut stats = NtfsScanStats::default();
        let mut record_buffer = Vec::new();
        let mut names = Vec::new();

        for (index, change) in changes.iter().enumerate() {
            if should_cancel() {
                return Err(NtfsError::Cancelled);
            }
            sink.set_scanned_entries(index as u64 + 1);
            let records =
                self.reader
                    .read_record(change.record_id, &mut record_buffer, &mut names)?;
            stats.scanned_records = stats.scanned_records.saturating_add(1);
            for record in records {
                if record.is_dir {
                    return Ok(ChangedRecordsOutcome::FullRequired(
                        "폴더 구조가 바뀌어 전체 경로를 다시 확인합니다".to_owned(),
                    ));
                }
                let Some(parent) = directory_paths.get(&record.parent_no) else {
                    continue;
                };
                let path = parent.join(&record.name);
                if !is_within_key(&path_key(&path), &self.root_key)
                    || is_excluded(&path, excluded_paths)
                {
                    continue;
                }
                if !sink
                    .push(raw_to_catalog(record, path, parent.clone()))
                    .map_err(NtfsError::Sink)?
                {
                    return Ok(ChangedRecordsOutcome::FullRequired(
                        "파일 수가 한도에 도달해 선택한 위치 전체를 다시 확인합니다".to_owned(),
                    ));
                }
            }
        }
        Ok(ChangedRecordsOutcome::Applied(stats))
    }
}

fn raw_to_catalog(record: RawRecord, path: PathBuf, parent: PathBuf) -> CatalogRecord {
    let extension = if record.is_dir {
        String::new()
    } else {
        path.extension()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    CatalogRecord {
        path: display_path(&path),
        name: record.name,
        parent: display_path(&parent),
        extension,
        kind: if record.is_dir {
            FileCatalogEntryKind::Directory
        } else {
            FileCatalogEntryKind::File
        },
        logical_bytes: record.size.unwrap_or(0),
        modified_at_ms: filetime_to_unix_ms(record.modified_ft),
        source_record_id: Some(record.record_no),
        source_parent_record_id: Some(record.parent_no),
    }
}

fn filetime_to_unix_ms(filetime: u64) -> Option<u128> {
    filetime
        .checked_sub(WINDOWS_TO_UNIX_EPOCH_100NS)
        .map(|ticks| (ticks / 10_000) as u128)
}

fn is_ntfs(drive: char) -> bool {
    let root = wide_null(&format!("{}:\\", drive));
    let mut file_system = [0_u16; 16];
    let ok = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            file_system.as_mut_ptr(),
            file_system.len() as u32,
        )
    };
    if ok == 0 {
        return false;
    }
    let length = file_system
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(file_system.len());
    String::from_utf16_lossy(&file_system[..length]).eq_ignore_ascii_case("NTFS")
}

fn path_identity(path: &Path) -> Option<(u32, u64)> {
    let mut options = OpenOptions::new();
    options
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let handle = options.open(path).ok()?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    let ok =
        unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, &mut information) };
    if ok == 0 {
        return None;
    }
    Some((
        information.dwVolumeSerialNumber,
        ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
    ))
}

fn drive_letter(path: &Path) -> Option<char> {
    let value = display_path(path);
    let bytes = value.as_bytes();
    (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        .then(|| (bytes[0] as char).to_ascii_uppercase())
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value
        .strip_prefix(r"\\?\UNC\")
        .map(|path| format!(r"\\{path}"))
        .or_else(|| value.strip_prefix(r"\\?\").map(str::to_owned))
        .unwrap_or_else(|| value.into_owned())
}

fn path_key(path: &Path) -> String {
    display_path(path)
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn is_within_key(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn is_excluded(path: &Path, excluded_paths: &[PathBuf]) -> bool {
    let key = path_key(path);
    excluded_paths
        .iter()
        .any(|excluded| path_key(excluded) == key)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Debug, Clone)]
struct DirectoryLink {
    parent_id: u64,
    name: String,
}

fn is_descendant(
    directory_id: u64,
    root_id: u64,
    directories: &HashMap<u64, DirectoryLink>,
    cache: &mut HashMap<u64, bool>,
) -> bool {
    if let Some(value) = cache.get(&directory_id) {
        return *value;
    }
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut current = directory_id;
    let result = loop {
        if current == root_id {
            break true;
        }
        if let Some(value) = cache.get(&current) {
            break *value;
        }
        if chain.len() >= MAX_DIRECTORY_DEPTH || !seen.insert(current) {
            break false;
        }
        chain.push(current);
        let Some(link) = directories.get(&current) else {
            break false;
        };
        current = link.parent_id;
    };
    for id in chain {
        cache.insert(id, result);
    }
    result
}

fn resolve_directory(
    directory_id: u64,
    directories: &HashMap<u64, DirectoryLink>,
    cache: &mut HashMap<u64, PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = cache.get(&directory_id) {
        return Some(path.clone());
    }
    let mut chain = Vec::<(u64, String)>::new();
    let mut seen = HashSet::new();
    let mut current = directory_id;
    let mut base = loop {
        if let Some(path) = cache.get(&current) {
            break path.clone();
        }
        if chain.len() >= MAX_DIRECTORY_DEPTH || !seen.insert(current) {
            return None;
        }
        let link = directories.get(&current)?;
        chain.push((current, link.name.clone()));
        current = link.parent_id;
    };
    for (id, name) in chain.into_iter().rev() {
        base.push(name);
        cache.insert(id, base.clone());
    }
    Some(base)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UsnJournalDataV0 {
    journal_id: u64,
    first_usn: i64,
    next_usn: i64,
    lowest_valid_usn: i64,
    max_usn: i64,
    maximum_size: u64,
    allocation_delta: u64,
}

#[repr(C)]
struct ReadUsnJournalDataV0 {
    start_usn: i64,
    reason_mask: u32,
    return_only_on_close: u32,
    timeout: u64,
    bytes_to_wait_for: u64,
    usn_journal_id: u64,
}

#[derive(Debug, Clone, Copy)]
struct JournalState {
    journal_id: u64,
    next_usn: i64,
    lowest_valid_usn: i64,
}

fn query_journal(volume: &Volume) -> Result<JournalState, u32> {
    let mut journal: UsnJournalDataV0 = unsafe { zeroed() };
    unsafe {
        volume.device_io_control(
            FSCTL_QUERY_USN_JOURNAL,
            ptr::null(),
            0,
            &mut journal as *mut _ as *mut c_void,
            size_of::<UsnJournalDataV0>() as u32,
        )?;
    }
    Ok(JournalState {
        journal_id: journal.journal_id,
        next_usn: journal.next_usn,
        lowest_valid_usn: journal.lowest_valid_usn,
    })
}

fn parse_journal_records(
    buffer: &[u8],
    changes: &mut HashMap<u64, JournalChange>,
) -> Result<(), NtfsError> {
    let mut offset = 8_usize;
    while offset + 60 <= buffer.len() {
        let length = read_u32(buffer, offset) as usize;
        if length < 60 || offset.saturating_add(length) > buffer.len() {
            return Err(NtfsError::Malformed {
                volume: "Windows 변경 기록".to_owned(),
                detail: "변경 기록의 길이가 올바르지 않습니다".to_owned(),
            });
        }
        let major = read_u16(buffer, offset + 4);
        if major != 2 {
            return Err(NtfsError::Malformed {
                volume: "Windows 변경 기록".to_owned(),
                detail: format!("지원하지 않는 Windows 변경 기록 형식 {major}"),
            });
        }
        let record_id = read_u64(buffer, offset + 8) & RECORD_MASK;
        let parent_id = read_u64(buffer, offset + 16) & RECORD_MASK;
        let attributes = read_u32(buffer, offset + 52);
        let change = changes.entry(record_id).or_insert_with(|| JournalChange {
            record_id,
            parent_ids: Vec::new(),
            directory_hint: false,
        });
        if !change.parent_ids.contains(&parent_id) && change.parent_ids.len() < 4 {
            change.parent_ids.push(parent_id);
        }
        change.directory_hint |= attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
        offset += length;
    }
    if offset != buffer.len() {
        return Err(NtfsError::Malformed {
            volume: "Windows 변경 기록".to_owned(),
            detail: "변경 기록의 마지막 부분이 완전하지 않습니다".to_owned(),
        });
    }
    Ok(())
}

struct Volume {
    handle: HANDLE,
    drive: char,
}

impl Volume {
    fn open(drive: char) -> Result<Self, NtfsError> {
        let label = format!("{drive}:");
        let device = wide_null(&format!(r"\\.\{drive}:"));
        let handle = unsafe {
            CreateFileW(
                device.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let code = unsafe { GetLastError() };
            return Err(if code == ERROR_ACCESS_DENIED {
                NtfsError::AccessDenied(label)
            } else {
                NtfsError::Open {
                    volume: label,
                    code,
                }
            });
        }
        Ok(Self { handle, drive })
    }

    unsafe fn device_io_control(
        &self,
        code: u32,
        input: *const c_void,
        input_size: u32,
        output: *mut c_void,
        output_size: u32,
    ) -> Result<u32, u32> {
        let mut returned = 0_u32;
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                code,
                input,
                input_size,
                output,
                output_size,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            Err(unsafe { GetLastError() })
        } else {
            Ok(returned)
        }
    }

    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> Result<(), NtfsError> {
        let read_error = |code| NtfsError::Read {
            volume: format!("{}:", self.drive),
            offset,
            code,
        };
        let ok =
            unsafe { SetFilePointerEx(self.handle, offset as i64, ptr::null_mut(), FILE_BEGIN) };
        if ok == 0 {
            return Err(read_error(unsafe { GetLastError() }));
        }
        let mut filled = 0_usize;
        while filled < buffer.len() {
            let mut read = 0_u32;
            let wanted = (buffer.len() - filled).min(u32::MAX as usize) as u32;
            let ok = unsafe {
                ReadFile(
                    self.handle,
                    buffer[filled..].as_mut_ptr(),
                    wanted,
                    &mut read,
                    ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(read_error(unsafe { GetLastError() }));
            }
            if read == 0 {
                return Err(read_error(0));
            }
            filled += read as usize;
        }
        Ok(())
    }
}

impl Drop for Volume {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Run {
    start_lcn: Option<u64>,
    clusters: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct StandardInfo {
    modified_filetime: u64,
}

#[derive(Debug, Clone)]
struct RawRecord {
    record_no: u64,
    parent_no: u64,
    name: String,
    is_dir: bool,
    size: Option<u64>,
    modified_ft: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct MftScanStats {
    scanned_records: u64,
    malformed_records: u64,
}

struct MftReader {
    volume: Volume,
    bytes_per_sector: u16,
    bytes_per_cluster: u32,
    record_size: u32,
    record_count: u64,
    runs: Vec<Run>,
}

impl MftReader {
    fn open(drive: char) -> Result<Self, NtfsError> {
        let volume = Volume::open(drive)?;
        let label = format!("{drive}:");
        let mut boot = [0_u8; 512];
        volume.read_at(0, &mut boot)?;
        if &boot[3..11] != b"NTFS    " {
            return Err(NtfsError::NotNtfs(label));
        }
        let bytes_per_sector = read_u16(&boot, 0x0B);
        let sectors_per_cluster = boot[0x0D];
        let bytes_per_cluster = bytes_per_sector as u32 * sectors_per_cluster as u32;
        let mft_lcn = read_u64(&boot, 0x30);
        let clusters_per_record = boot[0x40] as i8;
        let record_size = if clusters_per_record >= 0 {
            clusters_per_record as u32 * bytes_per_cluster
        } else {
            1_u32 << (-clusters_per_record as u32)
        };
        if bytes_per_sector == 0 || bytes_per_cluster == 0 || record_size == 0 {
            return Err(NtfsError::Malformed {
                volume: label,
                detail: "invalid NTFS geometry".to_owned(),
            });
        }
        let mut record_zero = vec![0_u8; record_size as usize];
        volume.read_at(mft_lcn * bytes_per_cluster as u64, &mut record_zero)?;
        if !apply_fixup(&mut record_zero, bytes_per_sector) {
            return Err(NtfsError::Malformed {
                volume: label,
                detail: "첫 번째 파일 기록의 복구 정보가 올바르지 않습니다".to_owned(),
            });
        }
        let (runs, data_size) = parse_mft_data_runs(&record_zero, &label)?;
        let covered_bytes = runs.iter().try_fold(0_u64, |total, run| {
            run.clusters
                .checked_mul(bytes_per_cluster as u64)
                .and_then(|bytes| total.checked_add(bytes))
        });
        if covered_bytes.is_none_or(|covered| covered < data_size) {
            return Err(NtfsError::Malformed {
                volume: label,
                detail: "파일 목록 저장 위치가 예상 크기와 맞지 않습니다".to_owned(),
            });
        }
        let record_count = data_size / record_size as u64;
        if record_count == 0 || record_count > MAX_MFT_RECORDS {
            return Err(NtfsError::Malformed {
                volume: label,
                detail: format!("파일 기록 수가 비정상적으로 큽니다: {record_count}"),
            });
        }
        Ok(Self {
            volume,
            bytes_per_sector,
            bytes_per_cluster,
            record_size,
            record_count,
            runs,
        })
    }

    fn volume(&self) -> &Volume {
        &self.volume
    }

    fn record_offset(&self, record_number: u64) -> Option<u64> {
        if record_number >= self.record_count {
            return None;
        }
        let record_size = self.record_size as u64;
        let cluster_size = self.bytes_per_cluster as u64;
        let mut accumulated = 0_u64;
        for run in &self.runs {
            let run_bytes = run.clusters.checked_mul(cluster_size)?;
            let records = run_bytes / record_size;
            if record_number < accumulated.saturating_add(records) {
                let lcn = run.start_lcn?;
                return lcn.checked_mul(cluster_size).and_then(|base| {
                    base.checked_add((record_number - accumulated) * record_size)
                });
            }
            accumulated = accumulated.saturating_add(records);
        }
        None
    }

    fn enumerate<F, P>(
        &self,
        mut on_record: F,
        mut on_progress: P,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<MftScanStats, NtfsError>
    where
        F: FnMut(RawRecord) -> bool,
        P: FnMut(u64),
    {
        const CHUNK_RECORDS: usize = 1_024;
        let record_size = self.record_size as usize;
        let cluster_size = self.bytes_per_cluster as u64;
        let mut buffer = vec![0_u8; record_size * CHUNK_RECORDS];
        let mut names = Vec::new();
        let mut record_number = 0_u64;
        let mut stats = MftScanStats::default();

        for run in &self.runs {
            if record_number >= self.record_count {
                break;
            }
            let Some(run_bytes) = run.clusters.checked_mul(cluster_size) else {
                stats.malformed_records = stats.malformed_records.saturating_add(1);
                continue;
            };
            let Some(lcn) = run.start_lcn else {
                record_number = record_number.saturating_add(run_bytes / record_size as u64);
                continue;
            };
            let Some(base) = lcn.checked_mul(cluster_size) else {
                stats.malformed_records = stats.malformed_records.saturating_add(1);
                continue;
            };
            let mut position = 0_u64;
            while position < run_bytes {
                if should_cancel() {
                    return Err(NtfsError::Cancelled);
                }
                let mut wanted = (run_bytes - position).min(buffer.len() as u64) as usize;
                wanted -= wanted % record_size;
                if wanted == 0 {
                    break;
                }
                self.volume
                    .read_at(base + position, &mut buffer[..wanted])?;
                for offset in (0..wanted).step_by(record_size) {
                    if record_number >= self.record_count {
                        return Ok(stats);
                    }
                    stats.scanned_records = stats.scanned_records.saturating_add(1);
                    let record = &mut buffer[offset..offset + record_size];
                    match parse_file_record(
                        record,
                        record_number,
                        self.bytes_per_sector,
                        &mut names,
                    ) {
                        ParsedRecord::Records(records) => {
                            for record in records {
                                if !on_record(record) {
                                    return Ok(stats);
                                }
                            }
                        }
                        ParsedRecord::Malformed => {
                            stats.malformed_records = stats.malformed_records.saturating_add(1)
                        }
                        ParsedRecord::Skipped => {}
                    }
                    record_number = record_number.saturating_add(1);
                    if record_number.is_multiple_of(2_048) {
                        on_progress(record_number);
                    }
                }
                position += wanted as u64;
            }
        }
        on_progress(stats.scanned_records);
        Ok(stats)
    }

    fn read_record(
        &self,
        record_number: u64,
        buffer: &mut Vec<u8>,
        names: &mut Vec<(u64, String)>,
    ) -> Result<Vec<RawRecord>, NtfsError> {
        let Some(offset) = self.record_offset(record_number) else {
            return Ok(Vec::new());
        };
        let record_size = self.record_size as usize;
        let sector = self.bytes_per_sector as u64;
        let aligned = offset / sector * sector;
        let padding = (offset - aligned) as usize;
        let length = (padding + record_size).next_multiple_of(sector as usize);
        buffer.resize(length, 0);
        self.volume.read_at(aligned, buffer)?;
        match parse_file_record(
            &mut buffer[padding..padding + record_size],
            record_number,
            self.bytes_per_sector,
            names,
        ) {
            ParsedRecord::Records(records) => Ok(records),
            ParsedRecord::Skipped => Ok(Vec::new()),
            ParsedRecord::Malformed => Err(NtfsError::Malformed {
                volume: "Windows 파일 기록".to_owned(),
                detail: format!("파일 기록 {record_number}을 확인할 수 없습니다"),
            }),
        }
    }
}

enum ParsedRecord {
    Records(Vec<RawRecord>),
    Skipped,
    Malformed,
}

fn parse_file_record(
    record: &mut [u8],
    record_number: u64,
    bytes_per_sector: u16,
    names: &mut Vec<(u64, String)>,
) -> ParsedRecord {
    if record.len() < 0x30 || &record[..4] != b"FILE" {
        return ParsedRecord::Skipped;
    }
    let flags = read_u16(record, 0x16);
    if flags & 0x01 == 0 {
        return ParsedRecord::Skipped;
    }
    if !apply_fixup(record, bytes_per_sector) {
        return ParsedRecord::Malformed;
    }
    let is_directory = flags & 0x02 != 0;
    let used = (read_u32(record, 0x18) as usize).min(record.len());
    let mut attribute_offset = read_u16(record, 0x14) as usize;
    if attribute_offset >= used {
        return ParsedRecord::Malformed;
    }
    names.clear();
    let mut data_size = None;
    let mut name_size = None;
    let mut standard = StandardInfo::default();

    while attribute_offset + 16 <= used {
        let attribute_type = read_u32(record, attribute_offset);
        if attribute_type == u32::MAX {
            break;
        }
        let length = read_u32(record, attribute_offset + 4) as usize;
        if length < 16 || attribute_offset.saturating_add(length) > used {
            return ParsedRecord::Malformed;
        }
        let non_resident = record[attribute_offset + 8];
        let attribute_name_length = record[attribute_offset + 9];
        match attribute_type {
            0x10 => standard = read_standard_info(record, attribute_offset),
            0x30 => {
                let content = attribute_offset + read_u16(record, attribute_offset + 0x14) as usize;
                if content + 0x42 > used {
                    return ParsedRecord::Malformed;
                }
                let parent = read_u64(record, content) & RECORD_MASK;
                name_size = Some(read_u64(record, content + 0x30));
                let character_count = record[content + 0x40] as usize;
                let namespace = record[content + 0x41];
                let start = content + 0x42;
                let end = start.saturating_add(character_count.saturating_mul(2));
                if end > used {
                    return ParsedRecord::Malformed;
                }
                if namespace != 2 {
                    let units = record[start..end]
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|pair| u16::from_le_bytes(*pair))
                        .collect::<Vec<_>>();
                    let name = String::from_utf16_lossy(&units);
                    if !names.iter().any(|(known_parent, known_name)| {
                        *known_parent == parent && known_name.eq_ignore_ascii_case(&name)
                    }) {
                        names.push((parent, name));
                    }
                }
            }
            0x80 if attribute_name_length == 0 => {
                data_size = Some(if non_resident == 0 {
                    read_u32(record, attribute_offset + 0x10) as u64
                } else {
                    read_u64(record, attribute_offset + 0x30)
                });
            }
            _ => {}
        }
        attribute_offset += length;
    }
    if names.is_empty() {
        return ParsedRecord::Skipped;
    }
    let size = if is_directory {
        None
    } else {
        data_size.or(name_size)
    };
    ParsedRecord::Records(
        names
            .drain(..)
            .map(|(parent_no, name)| RawRecord {
                record_no: record_number,
                parent_no,
                name,
                is_dir: is_directory,
                size,
                modified_ft: standard.modified_filetime,
            })
            .collect(),
    )
}

fn read_standard_info(record: &[u8], attribute_offset: usize) -> StandardInfo {
    let content = attribute_offset + read_u16(record, attribute_offset + 0x14) as usize;
    StandardInfo {
        modified_filetime: read_u64(record, content + 0x08),
    }
}

fn apply_fixup(record: &mut [u8], bytes_per_sector: u16) -> bool {
    if record.len() < 8 || bytes_per_sector == 0 {
        return false;
    }
    let array_offset = read_u16(record, 0x04) as usize;
    let array_count = read_u16(record, 0x06) as usize;
    let sector_size = bytes_per_sector as usize;
    if array_count < 2 || array_offset + array_count * 2 > record.len() {
        return false;
    }
    let sequence = read_u16(record, array_offset);
    for index in 1..array_count {
        let Some(end) = index
            .checked_mul(sector_size)
            .and_then(|value| value.checked_sub(2))
        else {
            return false;
        };
        if end + 2 > record.len() || read_u16(record, end) != sequence {
            return false;
        }
    }
    for index in 1..array_count {
        let end = index * sector_size - 2;
        let replacement = array_offset + index * 2;
        let replacement = [record[replacement], record[replacement + 1]];
        record[end..end + 2].copy_from_slice(&replacement);
    }
    true
}

fn parse_mft_data_runs(record: &[u8], volume: &str) -> Result<(Vec<Run>, u64), NtfsError> {
    if record.len() < 0x30 || &record[..4] != b"FILE" {
        return Err(NtfsError::Malformed {
            volume: volume.to_owned(),
            detail: "첫 번째 항목이 올바른 파일 기록이 아닙니다".to_owned(),
        });
    }
    let used = (read_u32(record, 0x18) as usize).min(record.len());
    let mut offset = read_u16(record, 0x14) as usize;
    while offset + 16 <= used {
        let attribute_type = read_u32(record, offset);
        if attribute_type == u32::MAX {
            break;
        }
        let length = read_u32(record, offset + 4) as usize;
        if length < 16 || offset.saturating_add(length) > used {
            break;
        }
        if attribute_type == 0x80 && record[offset + 8] == 1 && record[offset + 9] == 0 {
            let run_offset = offset + read_u16(record, offset + 0x20) as usize;
            let end = offset + length;
            let real_size = read_u64(record, offset + 0x30);
            if run_offset < end {
                let runs = decode_runs(&record[run_offset..end]);
                if !runs.is_empty() && real_size > 0 {
                    return Ok((runs, real_size));
                }
            }
        }
        offset += length;
    }
    Err(NtfsError::Malformed {
        volume: volume.to_owned(),
        detail: "읽을 수 있는 파일 목록 저장 영역이 없습니다".to_owned(),
    })
}

fn decode_runs(bytes: &[u8]) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut index = 0_usize;
    let mut previous_lcn = 0_i64;
    while index < bytes.len() {
        let header = bytes[index];
        if header == 0 {
            break;
        }
        index += 1;
        let length_bytes = (header & 0x0F) as usize;
        let offset_bytes = (header >> 4) as usize;
        if length_bytes == 0 || index + length_bytes + offset_bytes > bytes.len() {
            return Vec::new();
        }
        let mut clusters = 0_u64;
        for byte in 0..length_bytes {
            clusters |= (bytes[index + byte] as u64) << (8 * byte);
        }
        index += length_bytes;
        if clusters == 0 {
            return Vec::new();
        }
        if offset_bytes == 0 {
            runs.push(Run {
                start_lcn: None,
                clusters,
            });
        } else {
            let mut relative = 0_i64;
            for byte in 0..offset_bytes {
                relative |= (bytes[index + byte] as i64) << (8 * byte);
            }
            let shift = 64 - 8 * offset_bytes as u32;
            relative = (relative << shift) >> shift;
            let Some(next_lcn) = previous_lcn.checked_add(relative) else {
                return Vec::new();
            };
            if next_lcn < 0 {
                return Vec::new();
            }
            previous_lcn = next_lcn;
            runs.push(Run {
                start_lcn: Some(next_lcn as u64),
                clusters,
            });
        }
        index += offset_bytes;
    }
    runs
}

#[inline]
fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    bytes
        .get(offset..offset.saturating_add(2))
        .and_then(|slice| slice.try_into().ok())
        .map(u16::from_le_bytes)
        .unwrap_or(0)
}

#[inline]
fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    bytes
        .get(offset..offset.saturating_add(4))
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

#[inline]
fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    bytes
        .get(offset..offset.saturating_add(8))
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_paths_below_the_selected_root() {
        let directories = HashMap::from([
            (
                10,
                DirectoryLink {
                    parent_id: ROOT_MFT_RECORD,
                    name: "Users".to_owned(),
                },
            ),
            (
                11,
                DirectoryLink {
                    parent_id: 10,
                    name: "Danny".to_owned(),
                },
            ),
            (
                12,
                DirectoryLink {
                    parent_id: ROOT_MFT_RECORD,
                    name: "Windows".to_owned(),
                },
            ),
        ]);
        let mut membership = HashMap::from([(11, true), (ROOT_MFT_RECORD, false)]);
        assert!(is_descendant(11, 11, &directories, &mut membership));
        assert!(!is_descendant(12, 11, &directories, &mut membership));

        let mut paths = HashMap::from([
            (ROOT_MFT_RECORD, PathBuf::from(r"C:\")),
            (11, PathBuf::from(r"C:\Users\Example")),
        ]);
        assert_eq!(
            resolve_directory(11, &directories, &mut paths),
            Some(PathBuf::from(r"C:\Users\Example"))
        );
    }

    #[test]
    fn journal_parser_deduplicates_records_and_remembers_rename_parents() {
        let mut buffer = vec![0_u8; 8 + 60 * 2];
        buffer[..8].copy_from_slice(&123_i64.to_le_bytes());
        for (index, parent) in [10_u64, 20_u64].into_iter().enumerate() {
            let offset = 8 + index * 60;
            buffer[offset..offset + 4].copy_from_slice(&60_u32.to_le_bytes());
            buffer[offset + 4..offset + 6].copy_from_slice(&2_u16.to_le_bytes());
            buffer[offset + 8..offset + 16].copy_from_slice(&42_u64.to_le_bytes());
            buffer[offset + 16..offset + 24].copy_from_slice(&parent.to_le_bytes());
        }
        let mut changes = HashMap::new();
        parse_journal_records(&buffer, &mut changes).expect("parse journal records");
        let change = changes.get(&42).expect("deduplicated change");
        assert_eq!(change.parent_ids, vec![10, 20]);
    }

    #[test]
    fn data_runs_decode_signed_relative_offsets_and_sparse_runs() {
        let runs = decode_runs(&[
            0x11, 0x03, 0x0A, // 3 clusters at LCN 10
            0x11, 0x02, 0x05, // 2 clusters at LCN 15
            0x01, 0x04, // sparse 4 clusters
            0,
        ]);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].start_lcn, Some(10));
        assert_eq!(runs[1].start_lcn, Some(15));
        assert_eq!(runs[2].start_lcn, None);
    }

    #[test]
    fn converts_windows_filetime_without_underflow() {
        assert_eq!(filetime_to_unix_ms(WINDOWS_TO_UNIX_EPOCH_100NS), Some(0));
        assert_eq!(filetime_to_unix_ms(1), None);
    }

    struct CountingSink {
        records: usize,
        scanned: u64,
    }

    impl CatalogRecordSink for CountingSink {
        fn set_scanned_entries(&mut self, scanned_entries: u64) {
            self.scanned = scanned_entries;
        }

        fn push(&mut self, _record: CatalogRecord) -> Result<bool, String> {
            self.records += 1;
            Ok(self.records < 256)
        }
    }

    #[test]
    #[ignore = "requires an elevated Windows process and scans the current NTFS volume"]
    fn reads_the_real_ntfs_mft_and_queries_its_journal() {
        let current = std::env::current_dir().expect("current directory");
        let drive = drive_letter(&current).expect("drive letter");
        let root = PathBuf::from(format!("{drive}:\\"));
        let source = match NtfsSource::try_open(&root) {
            NtfsAvailability::Ready(source) => source,
            NtfsAvailability::Unavailable(reason) => panic!("NTFS source unavailable: {reason}"),
        };
        let checkpoint = source.checkpoint().expect("USN journal checkpoint");
        let mut sink = CountingSink {
            records: 0,
            scanned: 0,
        };
        let stats = source
            .enumerate_full(&[], &mut sink, &|| false)
            .expect("enumerate MFT");
        assert!(sink.records > 0);
        assert!(stats.scanned_records > 0);
        assert!(sink.scanned > 0);
        assert!(matches!(
            source
                .read_journal_delta(checkpoint, &|| false)
                .expect("read journal delta"),
            JournalDelta::Changes { .. }
        ));
    }
}
