//! Connected mice and their buttons (CORE-18), through IOKit's HID manager:
//! devices that are Generic Desktop mice, counting the distinct buttons
//! their input reports carry. Nothing is opened, so no permission is asked.

use std::collections::BTreeSet;
use std::ffi::c_void;

use objc2_core_foundation::{CFBoolean, CFDictionary, CFNumber, CFString};
use objc2_io_kit::{
    IOHIDDevice, IOHIDElement, IOHIDElementType, IOHIDManager, kIOHIDOptionsTypeNone,
};

use super::iokit::APPLE_VENDORS;
use crate::platform::MouseDevices;

const GENERIC_DESKTOP_PAGE: i32 = 0x01;
const MOUSE_USAGE: i32 = 0x02;
const BUTTON_PAGE: u32 = 0x09;
/// Touch pads report as mice too; their digitizer elements give them away.
const DIGITIZER_PAGE: u32 = 0x0D;

pub struct HidMice;

impl MouseDevices for HidMice {
    fn most_buttons(&self) -> u32 {
        let manager = IOHIDManager::new(None, kIOHIDOptionsTypeNone);
        let keys = [
            CFString::from_static_str("DeviceUsagePage"),
            CFString::from_static_str("DeviceUsage"),
        ];
        let values = [
            CFNumber::new_i32(GENERIC_DESKTOP_PAGE),
            CFNumber::new_i32(MOUSE_USAGE),
        ];
        let matching = CFDictionary::<CFString, CFNumber>::from_slices(
            &[&keys[0], &keys[1]],
            &[&values[0], &values[1]],
        );
        // SAFETY: the dictionary maps the documented matching keys to numbers.
        unsafe { manager.set_device_matching(Some(matching.as_opaque())) };
        let Some(devices) = manager.devices() else {
            return 0;
        };
        let count = usize::try_from(devices.count()).unwrap_or(0);
        let mut pointers: Vec<*const c_void> = vec![std::ptr::null(); count];
        // SAFETY: the buffer holds `count` values; the set outlives the loop.
        unsafe { devices.values(pointers.as_mut_ptr()) };
        pointers
            .into_iter()
            // SAFETY: the manager's set holds IOHIDDevice references.
            .filter_map(|pointer| unsafe { pointer.cast::<IOHIDDevice>().as_ref() })
            .filter(|device| counts(device))
            .map(buttons)
            .max()
            .unwrap_or(0)
    }
}

/// External mice that aren't Apple's: built-in trackpads, Magic Mouse and
/// Magic Trackpad are left out, as Wispr Flow leaves them out.
fn counts(device: &IOHIDDevice) -> bool {
    let built_in = device
        .property(&CFString::from_static_str("Built-In"))
        .and_then(|value| value.downcast::<CFBoolean>().ok())
        .is_some_and(|value| value.as_bool());
    let vendor = device
        .property(&CFString::from_static_str("VendorID"))
        .and_then(|value| value.downcast::<CFNumber>().ok())
        .and_then(|value| value.as_i64());
    !built_in && !vendor.is_some_and(|id| APPLE_VENDORS.contains(&id))
}

/// The distinct buttons a device reports; 0 for a touch pad.
fn buttons(device: &IOHIDDevice) -> u32 {
    // SAFETY: no matching dictionary: every element is returned.
    let Some(elements) = (unsafe { device.matching_elements(None, kIOHIDOptionsTypeNone) }) else {
        return 0;
    };
    // SAFETY: a device's elements are IOHIDElement references.
    let elements = unsafe { elements.cast_unchecked::<IOHIDElement>() };
    let mut usages = BTreeSet::new();
    for element in elements.iter() {
        match element.usage_page() {
            DIGITIZER_PAGE => return 0,
            BUTTON_PAGE if element.r#type() == IOHIDElementType::Input_Button => {
                usages.insert(element.usage());
            }
            _ => {}
        }
    }
    u32::try_from(usages.len()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lists this Mac's mice; run with `--ignored --nocapture`.
    #[test]
    #[ignore = "reads the connected devices"]
    fn reads_the_connected_mice() {
        println!("most buttons: {}", HidMice.most_buttons());
    }
}
