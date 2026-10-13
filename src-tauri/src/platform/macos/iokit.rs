//! IOKit lookups: whether a keyboard has Apple's Fn key (CORE-10), whether
//! the laptop lid is closed (MIC-05), and the Caps Lock lock state, which a
//! Caps Lock shortcut keeps from toggling (KEYS-04).

use std::ffi::CStr;

use objc2_core_foundation::{CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_graphics::{CGEventSource, CGEventSourceStateID};
use objc2_io_kit::{
    IOHIDGetModifierLockState, IOHIDSetModifierLockState, IOIteratorNext, IOObjectRelease,
    IORegistryEntryCreateCFProperty, IOServiceClose, IOServiceGetMatchingService,
    IOServiceGetMatchingServices, IOServiceMatching, IOServiceOpen, io_connect_t, io_object_t,
    kIOHIDCapsLockState, kIOHIDParamConnectType, kIOMainPortDefault,
};

/// Apple's USB and Bluetooth vendor IDs.
const APPLE_VENDORS: [i64; 2] = [0x05AC, 0x004C];
const KEYBOARD_PAGE: i64 = 1;
const KEYBOARD_USAGE: i64 = 6;
/// `kVK_CapsLock`.
const CAPS_LOCK_KEY: u16 = 0x39;

fn property(entry: io_object_t, key: &'static str) -> Option<CFRetained<CFType>> {
    let key = CFString::from_static_str(key);
    unsafe { IORegistryEntryCreateCFProperty(entry, Some(&key), None, 0) }
}

fn number(entry: io_object_t, key: &'static str) -> Option<i64> {
    property(entry, key)?.downcast::<CFNumber>().ok()?.as_i64()
}

fn flag(entry: io_object_t, key: &'static str) -> Option<bool> {
    Some(
        property(entry, key)?
            .downcast::<CFBoolean>()
            .ok()?
            .as_bool(),
    )
}

unsafe extern "C" {
    static mach_task_self_: libc::mach_port_t;
}

/// A matching dictionary for services of `class`.
fn matching(class: &CStr) -> Option<CFRetained<CFDictionary>> {
    let matching = unsafe { IOServiceMatching(class.as_ptr()) }?;
    // SAFETY: a mutable dictionary is a dictionary.
    Some(unsafe { CFRetained::cast_unchecked::<CFDictionary>(matching) })
}

/// Calls `visit` with each registered service of `class`.
fn each_service(class: &CStr, mut visit: impl FnMut(io_object_t)) {
    let Some(matching) = matching(class) else {
        return;
    };
    let mut iterator = 0;
    let status =
        unsafe { IOServiceGetMatchingServices(kIOMainPortDefault, Some(matching), &mut iterator) };
    if status != 0 || iterator == 0 {
        return;
    }
    loop {
        let service = IOIteratorNext(iterator);
        if service == 0 {
            break;
        }
        visit(service);
        IOObjectRelease(service);
    }
    IOObjectRelease(iterator);
}

/// Whether any connected keyboard is an Apple keyboard, which has the Fn
/// (Globe) key. Wispr Flow switches its defaults when none is found.
pub fn has_apple_fn_key() -> bool {
    let mut found = false;
    each_service(c"IOHIDEventService", |service| {
        let keyboard = number(service, "PrimaryUsagePage") == Some(KEYBOARD_PAGE)
            && number(service, "PrimaryUsage") == Some(KEYBOARD_USAGE);
        let apple = number(service, "VendorID").is_some_and(|id| APPLE_VENDORS.contains(&id));
        found |= keyboard && (apple || flag(service, "SupportsGlobeKey") == Some(true));
    });
    found
}

/// Whether a MacBook's lid is closed (clamshell mode). False on desktops.
pub fn lid_closed() -> bool {
    let Some(matching) = matching(c"IOPMrootDomain") else {
        return false;
    };
    let root = unsafe { IOServiceGetMatchingService(kIOMainPortDefault, Some(matching)) };
    if root == 0 {
        return false;
    }
    let closed = flag(root, "AppleClamshellState").unwrap_or(false);
    IOObjectRelease(root);
    closed
}

/// Whether the Caps Lock key itself is down right now.
pub fn caps_lock_key_down() -> bool {
    CGEventSource::key_state(CGEventSourceStateID::HIDSystemState, CAPS_LOCK_KEY)
}

fn with_hid_system<T>(work: impl FnOnce(io_connect_t) -> T) -> Option<T> {
    let matching = matching(c"IOHIDSystem")?;
    let service = unsafe { IOServiceGetMatchingService(kIOMainPortDefault, Some(matching)) };
    if service == 0 {
        return None;
    }
    let mut connect: io_connect_t = 0;
    let status = unsafe {
        IOServiceOpen(
            service,
            mach_task_self_,
            kIOHIDParamConnectType,
            &mut connect,
        )
    };
    IOObjectRelease(service);
    if status != 0 {
        log::warn!("could not open the HID system ({status})");
        return None;
    }
    let result = work(connect);
    IOServiceClose(connect);
    Some(result)
}

/// The Caps Lock lock state (capitals on).
pub fn caps_lock_on() -> Option<bool> {
    with_hid_system(|connect| {
        let mut on = false;
        let status =
            unsafe { IOHIDGetModifierLockState(connect, kIOHIDCapsLockState as i32, &mut on) };
        (status == 0).then_some(on)
    })
    .flatten()
}

/// Sets the Caps Lock lock state.
pub fn set_caps_lock(on: bool) {
    let status = with_hid_system(|connect| {
        IOHIDSetModifierLockState(connect, kIOHIDCapsLockState as i32, on)
    });
    if status != Some(0) {
        log::warn!("could not set Caps Lock: {status:?}");
    }
}
