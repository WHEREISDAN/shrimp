//! What the Fn (Globe) key does on its own, from System Settings › Keyboard ›
//! "Press 🌐 key to" (`com.apple.HIToolbox` `AppleFnUsageType`).

use std::process::Command;

use objc2_core_foundation::{CFPreferencesGetAppIntegerValue, CFString};

use crate::platform::{GlobeKeyAction, PlatformError};

pub fn globe_key_action() -> GlobeKeyAction {
    let key = CFString::from_static_str("AppleFnUsageType");
    let domain = CFString::from_static_str("com.apple.HIToolbox");
    let mut exists: u8 = 0;
    let value = unsafe { CFPreferencesGetAppIntegerValue(&key, &domain, &mut exists) };
    GlobeKeyAction::from_usage_type((exists != 0).then_some(value))
}

pub fn open_keyboard_settings() -> Result<(), PlatformError> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.Keyboard-Settings.extension")
        .spawn()
        .map(|_| ())
        .map_err(|err| PlatformError::Failed(format!("could not open System Settings: {err}")))
}
