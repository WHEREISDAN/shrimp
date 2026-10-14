//! Shell services: opening URLs and Settings pages, restarting Shrimp, free
//! disk space, the user's languages, region and name, and language names (ICU).

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Globalization::{
    GetUserDefaultGeoName, GetUserDefaultLocaleName, GetUserPreferredUILanguages,
    MUI_LANGUAGE_NAME, U_ZERO_ERROR, UErrorCode, ULDN_STANDARD_NAMES, uldn_close,
    uldn_languageDisplayName, uldn_open,
};
use windows::Win32::Security::Authentication::Identity::{GetUserNameExW, NameDisplay};
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{SM_CXSMICON, SW_SHOWNORMAL};
use windows::core::{HSTRING, PCSTR, PWSTR, w};

use crate::platform::{Launcher, PlatformError, RELAUNCH_ARG};
use crate::t;

/// No console window for helper processes.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn open(target: &str) -> Result<(), PlatformError> {
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(target),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute reports success with a value above 32.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(PlatformError::Failed(format!(
            "Windows could not open {target} ({})",
            result.0 as isize
        )))
    }
}

pub struct WinLauncher;

impl Launcher for WinLauncher {
    fn open_url(&self, url: &str) -> Result<(), PlatformError> {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(PlatformError::Failed("not a web address".into()));
        }
        open(url)
    }
}

/// The size of small icons, such as the taskbar's notification icons, at the
/// display scale Windows started Shrimp with: 16 px at 100 %.
pub fn small_icon_size() -> Option<u32> {
    let size = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) };
    u32::try_from(size).ok().filter(|size| *size > 0)
}

/// Windows 11 (build 22000 or later), the first with Mica.
pub fn is_windows_11() -> bool {
    let mut buffer = [0u16; 16];
    let mut size = std::mem::size_of_val(&buffer) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS {
        return false;
    }
    let units = (size as usize / 2).min(buffer.len());
    is_windows_11_build(&String::from_utf16_lossy(&buffer[..units]))
}

/// `CurrentBuildNumber` as the registry holds it, with its terminating zero.
fn is_windows_11_build(build: &str) -> bool {
    build
        .trim_end_matches('\0')
        .parse::<u32>()
        .is_ok_and(|build| build >= 22_000)
}

pub fn open_settings_page(page: &str) -> Result<(), PlatformError> {
    open(&format!("ms-settings:{page}"))
}

/// Settings › System › Sound (MIC-07).
pub fn open_sound_settings() -> Result<(), PlatformError> {
    open_settings_page("sound")
}

/// Settings › Time & language › Typing; Windows has no Fn key action to change.
pub fn open_keyboard_settings() -> Result<(), PlatformError> {
    open_settings_page("typing")
}

/// Settings › System › Storage (a full disk, APP-09).
pub fn open_storage_settings() -> Result<(), PlatformError> {
    open_settings_page("storagesense")
}

/// Opens Shrimp again with `RELAUNCH_ARG` once this process has exited; the
/// caller exits next. Started before the exit, the new instance would only
/// hand over to this one through the single-instance check.
pub fn relaunch_after_exit() -> Result<(), PlatformError> {
    let executable = std::env::current_exe()
        .map_err(|err| PlatformError::Failed(t!("error.relaunch.noExecutable", details = err)))?;
    let script = format!(
        "Wait-Process -Id {} -ErrorAction SilentlyContinue; \
         Start-Process -FilePath '{}' -ArgumentList '{RELAUNCH_ARG}'",
        std::process::id(),
        executable.display().to_string().replace('\'', "''")
    );
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
        ])
        .arg(script)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| PlatformError::Failed(t!("error.relaunch.failed", details = err)))
}

/// Windows runs Shrimp from wherever its installer put it; only the Mac app
/// asks to be moved into Applications (APP-11).
pub fn outside_applications() -> bool {
    false
}

pub fn move_to_applications() -> Result<bool, PlatformError> {
    Err(PlatformError::Unsupported)
}

/// Free space on the volume holding `path`, for this user, in bytes (APP-09).
pub fn free_bytes(path: &std::path::Path) -> Option<u64> {
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(&HSTRING::from(path), Some(&mut free), None, None) }.ok()?;
    Some(free)
}

/// The user's first preferred display language, e.g. "en-US".
pub fn preferred_language() -> Option<String> {
    let mut count = 0u32;
    let mut length = 0u32;
    unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut length) }
        .ok()?;
    let mut buffer = vec![0u16; length as usize];
    unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut length,
        )
    }
    .ok()?;
    let first = buffer.split(|unit| *unit == 0).next()?;
    let tag = String::from_utf16_lossy(first);
    (!tag.is_empty()).then_some(tag)
}

/// The user's region as an ISO 3166 code ("CA"): Settings › Time & language ›
/// Region, else the region of the user's locale ("en-CA").
pub fn region() -> Option<String> {
    let mut geo = [0u16; 16];
    let length = unsafe { GetUserDefaultGeoName(&mut geo) };
    let geo = text(&geo, length);
    if is_region(&geo) {
        return Some(geo.to_ascii_uppercase());
    }
    // LOCALE_NAME_MAX_LENGTH.
    let mut locale = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(&mut locale) };
    text(&locale, length)
        .split('-')
        .skip(1)
        .find(|part| is_region(part))
        .map(str::to_ascii_uppercase)
}

/// The text a call wrote into `buffer`, up to its terminating zero.
fn text(buffer: &[u16], length: i32) -> String {
    let written = usize::try_from(length).unwrap_or(0).min(buffer.len());
    let text = buffer[..written]
        .split(|unit| *unit == 0)
        .next()
        .unwrap_or_default();
    String::from_utf16_lossy(text)
}

/// Two letters; numeric UN M.49 areas such as "419" are no country.
fn is_region(code: &str) -> bool {
    code.len() == 2 && code.chars().all(|c| c.is_ascii_alphabetic())
}

/// The first word of the account's display name.
pub fn first_name() -> Option<String> {
    let mut size = 256u32;
    let mut buffer = vec![0u16; size as usize];
    let found = unsafe { GetUserNameExW(NameDisplay, Some(PWSTR(buffer.as_mut_ptr())), &mut size) };
    if !found {
        return None;
    }
    let name = String::from_utf16_lossy(&buffer[..size as usize]);
    name.split_whitespace().next().map(str::to_owned)
}

/// The name of language `code` in `language`, e.g. "Französisch" for "fr"
/// in "de", through the ICU that ships with Windows 10 and later.
pub fn language_name(code: &str, language: &str) -> Option<String> {
    let locale = std::ffi::CString::new(language.replace('-', "_")).ok()?;
    let code = std::ffi::CString::new(code.replace('-', "_")).ok()?;
    let mut status: UErrorCode = U_ZERO_ERROR;
    let names = unsafe {
        uldn_open(
            PCSTR(locale.as_ptr().cast()),
            ULDN_STANDARD_NAMES,
            &mut status,
        )
    };
    if names.is_null() || status.0 > 0 {
        return None;
    }
    let mut buffer = [0u16; 128];
    let length = unsafe {
        uldn_languageDisplayName(
            names,
            PCSTR(code.as_ptr().cast()),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            &mut status,
        )
    };
    unsafe { uldn_close(names) };
    if status.0 > 0 || length <= 0 {
        return None;
    }
    let name = String::from_utf16_lossy(&buffer[..length as usize]);
    let mut chars = name.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_languages_in_other_languages() {
        assert_eq!(language_name("fr", "de").as_deref(), Some("Französisch"));
        assert_eq!(language_name("es", "it").as_deref(), Some("Spagnolo"));
        assert_eq!(language_name("de", "en").as_deref(), Some("German"));
    }

    #[test]
    fn small_icons_follow_the_display_scale() {
        let size = small_icon_size().unwrap();
        assert!((16..=64).contains(&size), "{size}");
    }

    #[test]
    fn windows_11_starts_at_build_22000() {
        assert!(is_windows_11_build("22000\0"));
        assert!(is_windows_11_build("26100"));
        assert!(!is_windows_11_build("19044\0"));
        assert!(!is_windows_11_build("21999"));
        assert!(!is_windows_11_build(""));
        assert!(!is_windows_11_build("unknown"));
    }

    #[test]
    fn reads_the_user_and_the_disk() {
        assert!(preferred_language().is_some_and(|tag| tag.contains('-')));
        assert!(region().is_some_and(|code| is_region(&code)));
        let free = free_bytes(&std::env::temp_dir()).unwrap();
        assert!(free > 0);
    }
}
