//! The foreground window and the app behind it: executable name, display
//! name (the file description), and whether it runs elevated, which a
//! normal process can't type into (WIN-06).

use std::ffi::c_void;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY, TokenIntegrityLevel,
};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId,
};
use windows::core::{BOOL as CoreBool, PCWSTR, PWSTR, w};

use crate::platform::AppIdentity;

/// UWP apps draw inside this host; the app is a child window's process.
const FRAME_HOST: &str = "ApplicationFrameHost.exe";

pub fn foreground_window() -> Option<HWND> {
    let window = unsafe { GetForegroundWindow() };
    (!window.is_invalid()).then_some(window)
}

pub fn window_pid(window: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    pid
}

pub fn own_pid() -> u32 {
    std::process::id()
}

pub fn window_title(window: HWND) -> Option<String> {
    let length = unsafe { GetWindowTextLengthW(window) };
    if length <= 0 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let copied = unsafe { GetWindowTextW(window, &mut buffer) };
    let title = String::from_utf16_lossy(&buffer[..copied.max(0) as usize]);
    (!title.is_empty()).then_some(title)
}

struct Process(HANDLE);

impl Process {
    fn open(pid: u32) -> Option<Self> {
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .ok()
            .map(Self)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub fn image_path(pid: u32) -> Option<String> {
    let process = Process::open(pid)?;
    let mut buffer = vec![0u16; 1024];
    let mut size = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    }
    .ok()?;
    Some(String::from_utf16_lossy(&buffer[..size as usize]))
}

fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// The FileDescription of an executable ("Notepad", "Google Chrome").
fn description(path: &str) -> Option<String> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let path = PCWSTR(wide.as_ptr());
    let size = unsafe { GetFileVersionInfoSizeW(path, None) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    unsafe { GetFileVersionInfoW(path, None, size, data.as_mut_ptr().cast()) }.ok()?;
    let mut pointer: *mut c_void = std::ptr::null_mut();
    let mut length = 0u32;
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            w!("\\VarFileInfo\\Translation"),
            &mut pointer,
            &mut length,
        )
    };
    let translations: Vec<(u16, u16)> = if found.as_bool() && length >= 4 {
        // SAFETY: the translation table is pairs of u16 inside `data`.
        let pairs =
            unsafe { std::slice::from_raw_parts(pointer.cast::<u16>(), length as usize / 2) };
        pairs
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (pair[0], pair[1]))
            .collect()
    } else {
        Vec::new()
    };
    translations
        .into_iter()
        .chain([(0x0409, 0x04B0), (0x0409, 0x04E4)])
        .find_map(|(language, code_page)| {
            let key = format!("\\StringFileInfo\\{language:04x}{code_page:04x}\\FileDescription\0");
            let key: Vec<u16> = key.encode_utf16().collect();
            let mut pointer: *mut c_void = std::ptr::null_mut();
            let mut length = 0u32;
            let found = unsafe {
                VerQueryValueW(
                    data.as_ptr().cast(),
                    PCWSTR(key.as_ptr()),
                    &mut pointer,
                    &mut length,
                )
            };
            if !found.as_bool() || length == 0 {
                return None;
            }
            // SAFETY: the value is a NUL-terminated UTF-16 string inside `data`.
            let units =
                unsafe { std::slice::from_raw_parts(pointer.cast::<u16>(), length as usize) };
            let text = String::from_utf16_lossy(units);
            let text = text.trim_end_matches('\0').trim().to_owned();
            (!text.is_empty()).then_some(text)
        })
}

/// The pid of the first child window owned by another process: the UWP app
/// inside ApplicationFrameHost.
fn hosted_pid(window: HWND, host: u32) -> Option<u32> {
    struct Search {
        host: u32,
        found: Option<u32>,
    }
    unsafe extern "system" fn visit(child: HWND, data: LPARAM) -> CoreBool {
        // SAFETY: data is the Search below, alive for the enumeration.
        let search = unsafe { &mut *(data.0 as *mut Search) };
        let pid = window_pid(child);
        if pid != 0 && pid != search.host {
            search.found = Some(pid);
            return false.into();
        }
        true.into()
    }
    let mut search = Search { host, found: None };
    unsafe {
        let _ = EnumChildWindows(
            Some(window),
            Some(visit),
            LPARAM(&mut search as *mut Search as isize),
        );
    }
    search.found
}

/// The process that owns what the foreground window shows.
pub fn foreground_pid() -> Option<u32> {
    let window = foreground_window()?;
    let pid = window_pid(window);
    let path = image_path(pid)?;
    if file_name(&path).eq_ignore_ascii_case(FRAME_HOST) {
        return Some(hosted_pid(window, pid).unwrap_or(pid));
    }
    Some(pid)
}

pub fn identity(pid: u32) -> Option<AppIdentity> {
    let path = image_path(pid)?;
    let executable = file_name(&path).to_owned();
    let name = description(&path).unwrap_or_else(|| {
        executable
            .strip_suffix(".exe")
            .unwrap_or(&executable)
            .to_owned()
    });
    Some(AppIdentity {
        bundle_id: executable,
        name,
    })
}

/// A process's mandatory integrity level (0x2000 medium, 0x3000 high), or
/// None when its token can't be read, which for a process of the same user
/// means it is elevated.
fn integrity(process: HANDLE) -> Option<u32> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.ok()?;
    let token = Process(token);
    let mut size = 0u32;
    unsafe {
        let _ = GetTokenInformation(token.0, TokenIntegrityLevel, None, 0, &mut size);
    }
    let mut buffer = vec![0u8; size as usize];
    unsafe {
        GetTokenInformation(
            token.0,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            size,
            &mut size,
        )
    }
    .ok()?;
    // SAFETY: the buffer holds a TOKEN_MANDATORY_LABEL followed by its SID.
    let label = unsafe { &*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
    unsafe {
        let count = *GetSidSubAuthorityCount(label.Label.Sid);
        Some(*GetSidSubAuthority(
            label.Label.Sid,
            u32::from(count.saturating_sub(1)),
        ))
    }
}

/// Whether `pid` runs at a higher integrity level than Shrimp, so keys sent
/// to it are dropped (UIPI).
pub fn above_own_integrity(pid: u32) -> bool {
    let own = integrity(unsafe { GetCurrentProcess() }).unwrap_or(0x2000);
    let Some(process) = Process::open(pid) else {
        return false;
    };
    match integrity(process.0) {
        Some(level) => level > own,
        None => own < 0x3000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_its_own_process() {
        let me = identity(own_pid()).unwrap();
        assert!(me.bundle_id.to_lowercase().ends_with(".exe"), "{me:?}");
        assert!(!me.name.is_empty());
        assert!(!above_own_integrity(own_pid()));
    }

    #[test]
    fn reads_an_executable_description() {
        let notepad = r"C:\Windows\System32\notepad.exe";
        if std::path::Path::new(notepad).exists() {
            assert_eq!(description(notepad).as_deref(), Some("Notepad"));
        }
        assert_eq!(file_name(r"C:\a\b\Code.exe"), "Code.exe");
    }
}
