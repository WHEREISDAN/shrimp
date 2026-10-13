//! Finds the key that types "v" while Command is held in the active keyboard
//! layout, so Cmd+V pastes on Dvorak, AZERTY and "Dvorak - QWERTY ⌘" alike
//! (INS-03). Text Input Sources must be used from the main thread.

use std::ffi::c_void;

#[cfg(test)]
use objc2_core_foundation::{CFDictionary, CFRetained, CFString};

/// `kVK_ANSI_V`, the "v" key on a US layout.
pub const ANSI_V: u16 = 9;

type TISInputSourceRef = *mut c_void;
type CFDataRef = *const c_void;
#[cfg(test)]
type CFArrayRef = *const c_void;

const UC_KEY_ACTION_DOWN: u16 = 0;
/// `(cmdKey >> 8) & 0xFF`: the Command bit in UCKeyTranslate's modifier state.
const COMMAND_MODIFIER_STATE: u32 = 1;
const NO_DEAD_KEYS_MASK: u32 = 1;
const HIGHEST_KEY_CODE: u16 = 0x7F;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    #[cfg(test)]
    static kTISPropertyInputSourceID: *const c_void;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> TISInputSourceRef;
    #[cfg(test)]
    fn TISCreateInputSourceList(properties: *const c_void, include_all_installed: u8)
    -> CFArrayRef;
    fn TISGetInputSourceProperty(source: TISInputSourceRef, key: *const c_void) -> *const c_void;
    fn LMGetKbdType() -> u8;
    #[allow(clippy::too_many_arguments)]
    fn UCKeyTranslate(
        layout: *const c_void,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        options: u32,
        dead_key_state: *mut u32,
        max_length: usize,
        actual_length: *mut usize,
        unicode: *mut u16,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    #[cfg(test)]
    fn CFArrayGetCount(array: CFArrayRef) -> isize;
    #[cfg(test)]
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> *const c_void;
    fn CFRelease(object: *const c_void);
}

/// The key code for Cmd+V in the current layout, falling back to [`ANSI_V`].
pub fn paste_key_code() -> u16 {
    let source = unsafe { TISCopyCurrentKeyboardLayoutInputSource() };
    if source.is_null() {
        return ANSI_V;
    }
    let code = unsafe { command_key_for(source, 'v') };
    unsafe { CFRelease(source) };
    code.unwrap_or(ANSI_V)
}

/// Same lookup for an installed layout by ID, e.g. `com.apple.keylayout.Dvorak`.
#[cfg(test)]
fn paste_key_code_for_layout(layout_id: &str) -> Option<u16> {
    let key = unsafe { &*kTISPropertyInputSourceID.cast::<CFString>() };
    let value = CFString::from_str(layout_id);
    let filter = CFDictionary::<CFString, CFString>::from_slices(&[key], &[&*value]);
    let filter_ptr = CFRetained::as_ptr(&filter).as_ptr() as *const c_void;
    let sources = unsafe { TISCreateInputSourceList(filter_ptr, 1) };
    if sources.is_null() {
        return None;
    }
    let code = unsafe {
        (CFArrayGetCount(sources) > 0)
            .then(|| CFArrayGetValueAtIndex(sources, 0) as TISInputSourceRef)
            .and_then(|source| command_key_for(source, 'v'))
    };
    unsafe { CFRelease(sources) };
    code
}

unsafe fn command_key_for(source: TISInputSourceRef, wanted: char) -> Option<u16> {
    let data = unsafe { TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData) };
    if data.is_null() {
        return None;
    }
    let layout = unsafe { CFDataGetBytePtr(data) }.cast::<c_void>();
    let keyboard_type = u32::from(unsafe { LMGetKbdType() });
    std::iter::once(ANSI_V)
        .chain(0..=HIGHEST_KEY_CODE)
        .find(|&code| unsafe { translate(layout, code, keyboard_type) } == Some(wanted))
}

unsafe fn translate(layout: *const c_void, code: u16, keyboard_type: u32) -> Option<char> {
    let mut dead_key_state = 0u32;
    let mut length = 0usize;
    let mut chars = [0u16; 4];
    let status = unsafe {
        UCKeyTranslate(
            layout,
            code,
            UC_KEY_ACTION_DOWN,
            COMMAND_MODIFIER_STATE,
            keyboard_type,
            NO_DEAD_KEYS_MASK,
            &mut dead_key_state,
            chars.len(),
            &mut length,
            chars.as_mut_ptr(),
        )
    };
    if status != 0 || length != 1 {
        return None;
    }
    char::from_u32(u32::from(chars[0])).map(|c| c.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_paste_key_on_qwerty_dvorak_and_azerty() {
        assert_eq!(
            paste_key_code_for_layout("com.apple.keylayout.US"),
            Some(ANSI_V)
        );
        // Dvorak types "v" with the key in the ANSI "." position.
        assert_eq!(
            paste_key_code_for_layout("com.apple.keylayout.Dvorak"),
            Some(47)
        );
        // AZERTY keeps "v" in place; "Dvorak - QWERTY ⌘" switches to QWERTY with Command.
        assert_eq!(
            paste_key_code_for_layout("com.apple.keylayout.French"),
            Some(ANSI_V)
        );
        assert_eq!(
            paste_key_code_for_layout("com.apple.keylayout.DVORAK-QWERTYCMD"),
            Some(ANSI_V)
        );
    }
}
