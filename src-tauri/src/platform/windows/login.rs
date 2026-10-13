//! Launch at login (APP-06, WIN-10): a value under the user's `Run` key that
//! starts Shrimp with `--background`. The installer writes the same value,
//! so Shrimp starts at login after installing. Task Manager's Startup tab
//! can switch it off without deleting it (`StartupApproved\Run`), which
//! counts as off.

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegCloseKey,
    RegDeleteKeyValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::core::HSTRING;

use crate::platform::{LoginItem, PlatformError};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
/// Debug builds can pretend to have been launched at login (`1`) or not (`0`).
const LAUNCHED_AT_LOGIN_ENV: &str = "SHRIMP_LAUNCHED_AT_LOGIN";

/// The value's name: the product name, so the dev build has its own.
fn value_name() -> &'static str {
    if cfg!(debug_assertions) {
        "Shrimp Dev"
    } else {
        "Shrimp"
    }
}

/// The command the value runs.
pub fn command() -> Option<String> {
    let executable = std::env::current_exe().ok()?;
    Some(format!("\"{}\" --background", executable.display()))
}

fn read(
    key: &str,
    name: &str,
    flags: windows::Win32::System::Registry::REG_ROUTINE_FLAGS,
) -> Option<Vec<u8>> {
    let mut size = 0u32;
    let key = HSTRING::from(key);
    let name = HSTRING::from(name);
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            flags,
            None,
            None,
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let mut buffer = vec![0u8; size as usize];
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            flags,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then(|| {
        buffer.truncate(size as usize);
        buffer
    })
}

fn run_value() -> Option<String> {
    read(RUN, value_name(), RRF_RT_REG_SZ).map(|bytes| super::clipboard::utf16_text(&bytes))
}

/// Task Manager marks a disabled startup entry with an odd first byte.
fn disabled_in_task_manager() -> bool {
    read(APPROVED, value_name(), RRF_RT_REG_BINARY)
        .and_then(|bytes| bytes.first().copied())
        .is_some_and(|flag| flag & 1 == 1)
}

fn open_run() -> Result<HKEY, PlatformError> {
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN),
            None,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(key)
    } else {
        Err(PlatformError::Failed(format!(
            "could not open the Run key ({})",
            status.0
        )))
    }
}

fn delete(key: &str) -> Result<(), PlatformError> {
    let status = unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(value_name()),
        )
    };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(PlatformError::Failed(format!(
            "could not remove the login item ({})",
            status.0
        )))
    }
}

pub struct WinLoginItem;

impl LoginItem for WinLoginItem {
    fn enabled(&self) -> bool {
        run_value().is_some() && !disabled_in_task_manager()
    }

    fn set_enabled(&self, on: bool) -> Result<(), PlatformError> {
        if !on {
            return delete(RUN);
        }
        let command =
            command().ok_or_else(|| PlatformError::Failed("no executable path".into()))?;
        let data: Vec<u8> = command
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        let key = open_run()?;
        let status =
            unsafe { RegSetValueExW(key, &HSTRING::from(value_name()), None, REG_SZ, Some(&data)) };
        unsafe {
            let _ = RegCloseKey(key);
        }
        if status != ERROR_SUCCESS {
            return Err(PlatformError::Failed(format!(
                "could not add the login item ({})",
                status.0
            )));
        }
        // Turning it on in Shrimp overrides an earlier "Disabled" in Task Manager.
        delete(APPROVED)
    }

    fn launched_at_login(&self) -> bool {
        // The Run value passes --background, which the app checks itself.
        cfg!(debug_assertions)
            && std::env::var(LAUNCHED_AT_LOGIN_ENV).is_ok_and(|forced| forced == "1")
    }
}
