//! Windows-only bounded KEY_READ/64-bit adapter. No registry mutation APIs.
use super::model::{
    RegistryRead, RegistryReader, SettingDefinition, SettingScope, WindowsMetadata,
    decode_dword_value, decode_metadata_string,
};
use std::io;
use windows_sys::Win32::{
    Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_PATH_NOT_FOUND,
    },
    System::Registry::{
        REG_BINARY, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_NONE, REG_QWORD, REG_SZ,
        RegQueryValueExW,
    },
};
use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY},
};

pub(crate) struct NativeReader;
const VERSION_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

fn open_key(scope: SettingScope, path: &str) -> io::Result<RegKey> {
    let hive = match scope {
        SettingScope::CurrentUser => HKEY_CURRENT_USER,
        SettingScope::Machine => HKEY_LOCAL_MACHINE,
    };
    RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
}

fn read_error(code: Option<u32>) -> RegistryRead {
    match code {
        Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) => RegistryRead::Missing,
        Some(ERROR_ACCESS_DENIED) => RegistryRead::AccessDenied,
        _ => RegistryRead::Error,
    }
}

fn type_name(kind: u32) -> String {
    match kind {
        REG_NONE => "REG_NONE",
        REG_SZ => "REG_SZ",
        REG_EXPAND_SZ => "REG_EXPAND_SZ",
        REG_BINARY => "REG_BINARY",
        REG_DWORD => "REG_DWORD",
        REG_MULTI_SZ => "REG_MULTI_SZ",
        REG_QWORD => "REG_QWORD",
        _ => "REG_UNKNOWN",
    }
    .into()
}

fn dword(scope: SettingScope, key: &str, name: &str) -> RegistryRead {
    let key = match open_key(scope, key) {
        Ok(key) => key,
        Err(error) => return read_error(error.raw_os_error().map(|code| code as u32)),
    };
    let name = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut bytes = [0_u8; 4];
    let mut length = 4_u32;
    let mut kind = 0_u32;
    let status = unsafe {
        RegQueryValueExW(
            key.raw_handle() as _,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            bytes.as_mut_ptr(),
            &mut length,
        )
    };
    if status == 0 || status == ERROR_MORE_DATA {
        decode_dword_value(
            &type_name(kind),
            &bytes,
            length as usize,
            status == ERROR_MORE_DATA,
        )
    } else {
        read_error(Some(status))
    }
}

fn metadata_string(name: &str) -> Option<String> {
    let key = open_key(SettingScope::Machine, VERSION_KEY).ok()?;
    let name = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    // Only fixed OS metadata gets a bounded string buffer. Policy/settings
    // values above are always queried with exactly four bytes, without retries.
    let mut bytes = [0_u8; 256];
    let mut length = bytes.len() as u32;
    let mut kind = 0_u32;
    let status = unsafe {
        RegQueryValueExW(
            key.raw_handle() as _,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            bytes.as_mut_ptr(),
            &mut length,
        )
    };
    if status != 0 {
        return None;
    }
    decode_metadata_string(&type_name(kind), &bytes, length as usize)
}

impl RegistryReader for NativeReader {
    fn read(&self, definition: &SettingDefinition) -> RegistryRead {
        dword(definition.scope, definition.key, definition.name)
    }
    fn metadata(&self) -> WindowsMetadata {
        let os_build = metadata_string("CurrentBuildNumber")
            .filter(|value| value.len() <= 10 && value.bytes().all(|byte| byte.is_ascii_digit()));
        let edition = metadata_string("EditionID").filter(|value| {
            value.len() <= 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b' '))
        });
        let os_revision = match dword(SettingScope::Machine, VERSION_KEY, "UBR") {
            RegistryRead::Dword(value) => Some(value),
            _ => None,
        };
        let mut warnings = Vec::new();
        if os_build.is_none() {
            warnings.push(
                "Windows 빌드를 확인하지 못했습니다. 정책 지원 여부를 확정하지 않습니다.".into(),
            );
        }
        if os_revision.is_none() {
            warnings.push("Windows 빌드 수정 번호를 확인하지 못했습니다.".into());
        }
        if edition.is_none() {
            warnings.push(
                "Windows 에디션을 확인하지 못했습니다. SKU별 정책 지원은 미확인입니다.".into(),
            );
        }
        WindowsMetadata {
            os_build,
            os_revision,
            edition,
            warnings,
        }
    }
}

pub(crate) fn launch_folder_options() -> Result<(), String> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf, process::Command};
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
    let mut buffer = [0_u16; 32768];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() || buffer[..length].contains(&0) {
        return Err("Windows 시스템 폴더를 확인하지 못해 폴더 옵션을 실행하지 않았습니다".into());
    }
    let control = PathBuf::from(OsString::from_wide(&buffer[..length])).join("control.exe");
    if !control.is_absolute() || !control.is_file() {
        return Err("고정 Windows 제어판 실행 파일을 확인하지 못했습니다".into());
    }
    Command::new(control)
        .args(["/name", "Microsoft.FolderOptions"])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("폴더 옵션 연결 요청을 전달하지 못했습니다: {error}"))
}
