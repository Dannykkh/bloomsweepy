//! CoreFoundation preferences only. No shell, arbitrary domains from callers,
//! Finder restart, elevation, or modification of managed preferences.
use super::{Definition, Observation, StoredValue};
use std::{
    collections::HashSet,
    ffi::c_void,
    ptr,
    sync::{Mutex, OnceLock},
};

type CFTypeRef = *const c_void;
type CFStringRef = CFTypeRef;
const UTF8: u32 = 0x0800_0100;
static UNSYNCHRONIZED_DOMAINS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn check_domain_health(domain: &str) -> Result<(), String> {
    let domains = UNSYNCHRONIZED_DOMAINS
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|_| "설정 저장 상태를 확인하지 못했습니다")?;
    if domains.contains(domain) {
        return Err("이 설정 저장소의 변경 완료를 확인하지 못했습니다. 자동으로 다시 저장하지 않습니다. 앱을 다시 실행한 뒤 현재 값과 변경 기록을 확인해 주세요".into());
    }
    Ok(())
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithBytes(
        allocator: CFTypeRef,
        bytes: *const u8,
        length: isize,
        encoding: u32,
        external: u8,
    ) -> CFStringRef;
    fn CFRelease(value: CFTypeRef);
    fn CFGetTypeID(value: CFTypeRef) -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(value: CFTypeRef) -> u8;
    fn CFPreferencesCopyValue(
        key: CFStringRef,
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    ) -> CFTypeRef;
    fn CFPreferencesCopyAppValue(key: CFStringRef, application: CFStringRef) -> CFTypeRef;
    fn CFPreferencesAppValueIsForced(key: CFStringRef, application: CFStringRef) -> u8;
    fn CFPreferencesSetValue(
        key: CFStringRef,
        value: CFTypeRef,
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    );
    fn CFPreferencesSynchronize(
        application: CFStringRef,
        user: CFStringRef,
        host: CFStringRef,
    ) -> u8;
    static kCFPreferencesCurrentUser: CFStringRef;
    static kCFPreferencesAnyHost: CFStringRef;
    static kCFPreferencesCurrentHost: CFStringRef;
    static kCFPreferencesAnyApplication: CFStringRef;
    static kCFBooleanTrue: CFTypeRef;
    static kCFBooleanFalse: CFTypeRef;
}

struct OwnedCF(CFTypeRef);
impl Drop for OwnedCF {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}

fn string(value: &str) -> Result<OwnedCF, String> {
    let reference = unsafe {
        CFStringCreateWithBytes(ptr::null(), value.as_ptr(), value.len() as isize, UTF8, 0)
    };
    if reference.is_null() {
        return Err("설정 식별자를 만들지 못했습니다".into());
    }
    Ok(OwnedCF(reference))
}

fn version() -> Option<String> {
    let mut length = 0_usize;
    let name = c"kern.osproductversion".as_ptr();
    if unsafe { libc::sysctlbyname(name, ptr::null_mut(), &mut length, ptr::null_mut(), 0) } != 0
        || length == 0
        || length > 128
    {
        return None;
    }
    let mut buffer = vec![0_u8; length];
    if unsafe {
        libc::sysctlbyname(
            name,
            buffer.as_mut_ptr().cast::<c_void>(),
            &mut length,
            ptr::null_mut(),
            0,
        )
    } != 0
    {
        return None;
    }
    let value = std::ffi::CStr::from_bytes_until_nul(&buffer)
        .ok()?
        .to_str()
        .ok()?;
    Some(value.to_owned())
}

pub(super) fn platform() -> String {
    format!(
        "macOS {} ({})",
        version().unwrap_or_else(|| "unknown".into()),
        std::env::consts::ARCH
    )
}

pub(super) fn supported() -> bool {
    // Explicit compatibility boundary, not a claim that future macOS releases
    // preserve undocumented Finder preference keys or refresh behavior.
    version()
        .and_then(|value| value.split('.').next()?.parse::<u32>().ok())
        .is_some_and(|major| matches!(major, 14 | 15 | 26))
}

pub(super) fn read(item: &Definition) -> Result<Observation, String> {
    read_at(item.domain, item.key)
}

fn read_at(domain: &str, key: &str) -> Result<Observation, String> {
    // A failed write may still exist in CoreFoundation's process cache. A
    // later inspection must not silently retry that write by synchronizing.
    check_domain_health(domain)?;
    let domain_string = string(domain)?;
    let key = string(key)?;
    let application = if domain == ".GlobalPreferences" {
        unsafe { kCFPreferencesAnyApplication }
    } else {
        domain_string.0
    };
    // Synchronization invalidates this process's cached preferences too; this
    // module never leaves staged values behind after a successful operation.
    if unsafe {
        CFPreferencesSynchronize(
            application,
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        )
    } == 0
    {
        return Err("설정 저장소의 현재 값을 동기화하지 못했습니다".into());
    }
    let managed = unsafe { CFPreferencesAppValueIsForced(key.0, domain_string.0) != 0 };
    let raw = if managed {
        OwnedCF(unsafe { CFPreferencesCopyAppValue(key.0, domain_string.0) })
    } else {
        // A host-specific override would make a plain AnyHost Bool misleading.
        // Do not guess an effective value or overwrite that separate scope.
        let host_value = OwnedCF(unsafe {
            CFPreferencesCopyValue(
                key.0,
                application,
                kCFPreferencesCurrentUser,
                kCFPreferencesCurrentHost,
            )
        });
        if !host_value.0.is_null() {
            return Err("이 컴퓨터에만 적용된 별도 설정 값이 있어 변경하지 않습니다".into());
        }
        OwnedCF(unsafe {
            CFPreferencesCopyValue(
                key.0,
                application,
                kCFPreferencesCurrentUser,
                kCFPreferencesAnyHost,
            )
        })
    };
    let value = if raw.0.is_null() {
        StoredValue::Unset
    } else if unsafe { CFGetTypeID(raw.0) == CFBooleanGetTypeID() } {
        StoredValue::Bool(unsafe { CFBooleanGetValue(raw.0) != 0 })
    } else {
        StoredValue::TypeMismatch
    };
    Ok(Observation { value, managed })
}

pub(super) fn write(item: &Definition, value: Option<bool>) -> Result<(), String> {
    // The externally reachable engine can select only a fixed Definition.
    write_at(item.domain, item.key, value)
}

fn write_at(domain: &str, key: &str, value: Option<bool>) -> Result<(), String> {
    read_at(domain, key)?.editable_value()?;
    let domain_string = string(domain)?;
    let key = string(key)?;
    let application = if domain == ".GlobalPreferences" {
        unsafe { kCFPreferencesAnyApplication }
    } else {
        domain_string.0
    };
    let value = match value {
        Some(true) => unsafe { kCFBooleanTrue },
        Some(false) => unsafe { kCFBooleanFalse },
        None => ptr::null(),
    };
    unsafe {
        CFPreferencesSetValue(
            key.0,
            value,
            application,
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        )
    };
    if unsafe {
        CFPreferencesSynchronize(
            application,
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        )
    } == 0
    {
        UNSYNCHRONIZED_DOMAINS
            .get_or_init(Mutex::default)
            .lock()
            .map_err(|_| "설정 저장 상태를 보존하지 못했습니다")?
            .insert(domain.to_owned());
        return Err("설정 저장을 확인하지 못했습니다. 미완료 변경 기록을 확인해 주세요".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_native_domain_roundtrip_never_touches_finder_preferences() {
        let domain = format!(
            "com.broomsweepy.tests.mac-settings.{}.{}",
            std::process::id(),
            super::super::new_change_id().unwrap()
        );
        let key = "SyntheticBool";
        assert_eq!(read_at(&domain, key).unwrap().value, StoredValue::Unset);
        write_at(&domain, key, Some(true)).unwrap();
        assert_eq!(
            read_at(&domain, key).unwrap().value,
            StoredValue::Bool(true)
        );
        write_at(&domain, key, Some(false)).unwrap();
        assert_eq!(
            read_at(&domain, key).unwrap().value,
            StoredValue::Bool(false)
        );
        write_at(&domain, key, None).unwrap();
        assert_eq!(read_at(&domain, key).unwrap().value, StoredValue::Unset);
    }

    #[test]
    fn failed_domain_latch_prevents_a_read_from_retrying_synchronization() {
        let domain = format!(
            "com.broomsweepy.tests.latched.{}",
            super::super::new_change_id().unwrap()
        );
        UNSYNCHRONIZED_DOMAINS
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .insert(domain.clone());
        assert!(read_at(&domain, "SyntheticBool").is_err());
        assert!(write_at(&domain, "SyntheticBool", Some(true)).is_err());
        UNSYNCHRONIZED_DOMAINS
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .remove(&domain);
    }
}
