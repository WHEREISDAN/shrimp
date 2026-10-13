//! Secure Keyboard Entry (KEYS-09). While another app has it on, event taps
//! receive modifier changes (so Fn and other modifier-only holds keep
//! working) but no key presses, so key combinations can't be seen.

use objc2_app_kit::NSRunningApplication;
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_graphics::CGSessionCopyCurrentDictionary;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

/// The app that has Secure Keyboard Entry on, or None when it is off.
pub fn secure_input_owner() -> Option<String> {
    if !unsafe { IsSecureEventInputEnabled() } {
        return None;
    }
    let name = owner_pid()
        .and_then(NSRunningApplication::runningApplicationWithProcessIdentifier)
        .and_then(|app| app.localizedName())
        .map(|name| name.to_string());
    Some(name.unwrap_or_else(|| "Another app".into()))
}

fn owner_pid() -> Option<libc::pid_t> {
    let session = CGSessionCopyCurrentDictionary()?;
    // SAFETY: the session dictionary maps CFString keys to CF values.
    let session = unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(session) };
    let pid = session
        .get(&CFString::from_static_str("kCGSSessionSecureInputPID"))?
        .downcast::<CFNumber>()
        .ok()?
        .as_i32()?;
    (pid > 0).then_some(pid)
}
