//! Launch at login (APP-06): Shrimp is the user's login item through
//! `SMAppService.mainAppService`, the current Service Management API. macOS
//! doesn't say whether a launch came from the login item, so a launch within
//! two minutes of the console login, with the login item on, counts as one.

use std::ffi::CStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use objc2_service_management::{SMAppService, SMAppServiceStatus};

use crate::platform::{LoginItem, PlatformError};

/// A launch this soon after logging in came from the login item.
const AFTER_LOGIN: Duration = Duration::from_secs(120);
/// Debug builds can pretend to have been launched at login (`1`) or not (`0`).
const LAUNCHED_AT_LOGIN_ENV: &str = "SHRIMP_LAUNCHED_AT_LOGIN";

pub struct MacLoginItem;

impl LoginItem for MacLoginItem {
    fn enabled(&self) -> bool {
        let service = unsafe { SMAppService::mainAppService() };
        let status = unsafe { service.status() };
        status == SMAppServiceStatus::Enabled
    }

    fn set_enabled(&self, on: bool) -> Result<(), PlatformError> {
        let service = unsafe { SMAppService::mainAppService() };
        let result = if on {
            unsafe { service.registerAndReturnError() }
        } else {
            unsafe { service.unregisterAndReturnError() }
        };
        result.map_err(|error| PlatformError::Failed(error.localizedDescription().to_string()))
    }

    fn launched_at_login(&self) -> bool {
        if cfg!(debug_assertions)
            && let Ok(forced) = std::env::var(LAUNCHED_AT_LOGIN_ENV)
        {
            return forced == "1";
        }
        let since_login = console_login().and_then(|at| SystemTime::now().duration_since(at).ok());
        self.enabled() && since_login.is_some_and(|elapsed| elapsed < AFTER_LOGIN)
    }
}

/// When the current console session began, from the utmpx records.
fn console_login() -> Option<SystemTime> {
    let mut latest: Option<SystemTime> = None;
    unsafe {
        libc::setutxent();
        loop {
            let entry = libc::getutxent();
            if entry.is_null() {
                break;
            }
            let entry = &*entry;
            if entry.ut_type != libc::USER_PROCESS {
                continue;
            }
            let line = CStr::from_ptr(entry.ut_line.as_ptr()).to_string_lossy();
            if line != "console" {
                continue;
            }
            let at = UNIX_EPOCH
                + Duration::from_secs(entry.ut_tv.tv_sec as u64)
                + Duration::from_micros(entry.ut_tv.tv_usec as u64);
            latest = Some(latest.map_or(at, |known| known.max(at)));
        }
        libc::endutxent();
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_console_session() {
        // Every logged-in Mac has one; CI runners may not.
        if let Some(at) = console_login() {
            assert!(at <= SystemTime::now());
        }
    }
}
