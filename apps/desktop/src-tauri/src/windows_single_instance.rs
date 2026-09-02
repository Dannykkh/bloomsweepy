use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, WAIT_FAILED, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, ResetEvent, SetEvent, WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindowAsync,
};

const MUTEX_NAME: &str = r"Local\com.broomsweepy.desktop.single-instance.mutex";
const FOREGROUND_EVENT_NAME: &str =
    r"Local\com.broomsweepy.desktop.single-instance.foreground-requested";
const WINDOW_TITLE: &str = concat!("BroomSweepy ", env!("CARGO_PKG_VERSION"));
const ACTIVATION_RETRY_COUNT: usize = 30;
const ACTIVATION_RETRY_DELAY: Duration = Duration::from_millis(50);
const WINDOW_TITLE_BUFFER_LEN: usize = 256;

pub(crate) enum AcquireOutcome {
    Primary(PrimaryInstance),
    Secondary,
}

pub(crate) struct PrimaryInstance {
    // Win32 kernel handles are process-wide and may safely be used from the window worker.
    // Store the pointer values as usize so this lifetime guard can cross Rust threads.
    mutex_handle: usize,
    foreground_event_handle: usize,
}

impl PrimaryInstance {
    pub(crate) fn foreground_requested(&self) -> bool {
        // SAFETY: the event handle stays open for the lifetime of this guard.
        unsafe { WaitForSingleObject(self.foreground_event_handle as _, 0) == WAIT_OBJECT_0 }
    }

    pub(crate) fn wait_for_foreground_request(&self, timeout: Duration) -> Result<bool, String> {
        let timeout_ms = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
        // SAFETY: the event handle stays open for the lifetime of this guard.
        let result = unsafe { WaitForSingleObject(self.foreground_event_handle as _, timeout_ms) };
        match result {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => Err(format!(
                "foreground event wait failed with Win32 error {}",
                unsafe { GetLastError() }
            )),
            other => Err(format!("foreground event wait returned {other}")),
        }
    }

    pub(crate) fn reset_foreground_request(&self) -> Result<(), String> {
        // SAFETY: the event handle stays open for the lifetime of this guard.
        if unsafe { ResetEvent(self.foreground_event_handle as _) } == 0 {
            return Err(format!(
                "foreground event reset failed with Win32 error {}",
                unsafe { GetLastError() }
            ));
        }
        Ok(())
    }
}

impl Drop for PrimaryInstance {
    fn drop(&mut self) {
        // SAFETY: these non-null handles are owned by this guard and closed exactly once.
        unsafe {
            CloseHandle(self.foreground_event_handle as _);
            CloseHandle(self.mutex_handle as _);
        }
    }
}

pub(crate) fn acquire_or_activate(activate_existing: bool) -> Result<AcquireOutcome, String> {
    acquire_named_instance(MUTEX_NAME, FOREGROUND_EVENT_NAME, activate_existing)
}

fn acquire_named_instance(
    mutex_name: &str,
    event_name: &str,
    activate_existing: bool,
) -> Result<AcquireOutcome, String> {
    let event_name = wide_null(event_name);
    // Create the shared event before the mutex. This removes the small race where a
    // secondary observes the mutex before the primary can publish its activation event.
    // SAFETY: the name is NUL-terminated and both optional security pointers are null.
    let event_handle = unsafe { CreateEventW(ptr::null(), 1, 0, event_name.as_ptr()) };
    if event_handle.is_null() {
        return Err(format!(
            "foreground event creation failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }

    let mutex_name = wide_null(mutex_name);
    // SAFETY: the name is NUL-terminated and the optional security pointer is null.
    let mutex_handle = unsafe { CreateMutexW(ptr::null(), 0, mutex_name.as_ptr()) };
    if mutex_handle.is_null() {
        let error = unsafe { GetLastError() };
        // SAFETY: event_handle was created successfully and is owned here.
        unsafe { CloseHandle(event_handle) };
        return Err(format!(
            "single-instance mutex creation failed with Win32 error {error}"
        ));
    }

    // GetLastError must be read immediately after CreateMutexW: a valid handle plus
    // ERROR_ALREADY_EXISTS means another process owns the primary instance lifetime.
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if !already_exists {
        return Ok(AcquireOutcome::Primary(PrimaryInstance {
            mutex_handle: mutex_handle as usize,
            foreground_event_handle: event_handle as usize,
        }));
    }

    if activate_existing {
        // Signal first so a primary that has not hidden its startup window yet can
        // cancel that hide. Direct Win32 activation handles an already-hidden window.
        // SAFETY: event_handle is valid until it is closed below.
        if unsafe { SetEvent(event_handle) } == 0 {
            eprintln!(
                "기존 창 활성화 신호를 보내지 못했습니다: Win32 error {}",
                unsafe { GetLastError() }
            );
        }
        activate_existing_window();
    }

    // SAFETY: secondary processes retain neither shared kernel object.
    unsafe {
        CloseHandle(mutex_handle);
        CloseHandle(event_handle);
    }
    Ok(AcquireOutcome::Secondary)
}

fn activate_existing_window() {
    for _ in 0..ACTIVATION_RETRY_COUNT {
        if let Some(window) = find_broomsweepy_window() {
            // These APIs post to the owning GUI thread instead of entering Tauri's
            // runtime dispatcher from a callback in this secondary process.
            // SAFETY: EnumWindows returned a live top-level HWND.
            unsafe {
                ShowWindowAsync(window, SW_RESTORE);
                ShowWindowAsync(window, SW_SHOW);
                SetForegroundWindow(window);
            }
            return;
        }
        thread::sleep(ACTIVATION_RETRY_DELAY);
    }
}

struct WindowSearch {
    expected_title: Vec<u16>,
    result: HWND,
}

fn find_broomsweepy_window() -> Option<HWND> {
    let mut search = WindowSearch {
        expected_title: OsStr::new(WINDOW_TITLE).encode_wide().collect(),
        result: ptr::null_mut(),
    };

    // SAFETY: the callback only borrows search during this synchronous EnumWindows call.
    unsafe {
        EnumWindows(
            Some(find_window_callback),
            (&mut search as *mut WindowSearch) as LPARAM,
        );
    }

    (!search.result.is_null()).then_some(search.result)
}

unsafe extern "system" fn find_window_callback(window: HWND, parameter: LPARAM) -> i32 {
    // SAFETY: parameter is the live WindowSearch pointer supplied to EnumWindows above.
    let search = unsafe { &mut *(parameter as *mut WindowSearch) };
    let mut title = [0_u16; WINDOW_TITLE_BUFFER_LEN];
    // SAFETY: title is writable for the declared buffer length and window comes from EnumWindows.
    let title_len = unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) };
    if title_len > 0 && title_matches(&title[..title_len as usize], &search.expected_title) {
        search.result = window;
        return 0;
    }
    1
}

fn title_matches(actual: &[u16], expected: &[u16]) -> bool {
    actual == expected
}

fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_object_names_are_app_specific_and_nul_terminated() {
        assert!(MUTEX_NAME.contains("com.broomsweepy.desktop"));
        assert!(FOREGROUND_EVENT_NAME.contains("com.broomsweepy.desktop"));
        assert_eq!(wide_null(MUTEX_NAME).last(), Some(&0));
    }

    #[test]
    fn window_title_match_is_exact() {
        let expected: Vec<u16> = OsStr::new(WINDOW_TITLE).encode_wide().collect();
        assert!(title_matches(&expected, &expected));

        let other: Vec<u16> = OsStr::new("BroomSweepy").encode_wide().collect();
        assert!(!title_matches(&other, &expected));
    }

    #[test]
    fn named_instance_guard_detects_a_secondary_without_activation() {
        let unique = format!(
            r"Local\com.broomsweepy.desktop.test.{}.{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        );
        let event = format!("{unique}.event");
        let first = acquire_named_instance(&unique, &event, false).expect("first acquisition");
        let AcquireOutcome::Primary(primary) = first else {
            panic!("first acquisition must become the primary");
        };
        assert!(!primary.foreground_requested());

        // SAFETY: the primary guard owns this live event handle for the test duration.
        assert_ne!(unsafe { SetEvent(primary.foreground_event_handle as _) }, 0);
        assert!(
            primary
                .wait_for_foreground_request(Duration::ZERO)
                .expect("wait for test activation")
        );
        primary
            .reset_foreground_request()
            .expect("reset test activation");
        assert!(!primary.foreground_requested());

        let second = acquire_named_instance(&unique, &event, false).expect("second acquisition");
        assert!(matches!(second, AcquireOutcome::Secondary));
    }
}
