//! Thin wrappers over the Accessibility C API: attributes by name, ranges,
//! parameterized text reads and children. The crate does not export the
//! `kAX…` names (they are `CFSTR` macros), so attributes are built from
//! static strings.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_application_services::{AXError, AXUIElement, AXValue, AXValueType};
use objc2_core_foundation::{
    CFArray, CFBoolean, CFHash, CFNumber, CFRange, CFRetained, CFString, CFType, CFURL,
};

use crate::platform::TextRange;

pub type Element = CFRetained<AXUIElement>;

pub fn application(pid: i32, timeout_seconds: f32) -> Element {
    let app = unsafe { AXUIElement::new_application(pid) };
    unsafe { app.set_messaging_timeout(timeout_seconds) };
    app
}

pub fn copy(element: &AXUIElement, name: &'static str) -> Result<CFRetained<CFType>, AXError> {
    let attribute = CFString::from_static_str(name);
    let mut value: *const CFType = std::ptr::null();
    let error = unsafe { element.copy_attribute_value(&attribute, NonNull::from(&mut value)) };
    if error != AXError::Success {
        return Err(error);
    }
    NonNull::new(value.cast_mut())
        .map(|value| unsafe { CFRetained::from_raw(value) })
        .ok_or(AXError::NoValue)
}

pub fn element(element: &AXUIElement, name: &'static str) -> Option<Element> {
    copy(element, name).ok()?.downcast::<AXUIElement>().ok()
}

pub fn string(element: &AXUIElement, name: &'static str) -> Option<String> {
    let value = copy(element, name).ok()?;
    value
        .downcast::<CFString>()
        .ok()
        .map(|text| text.to_string())
}

pub fn number(element: &AXUIElement, name: &'static str) -> Option<i64> {
    copy(element, name)
        .ok()?
        .downcast::<CFNumber>()
        .ok()?
        .as_i64()
}

/// A URL attribute (`AXURL`, `AXDocument`) as text; it may be a CFURL or a string.
pub fn url(element: &AXUIElement, name: &'static str) -> Option<String> {
    let value = copy(element, name).ok()?;
    let value = match value.downcast::<CFString>() {
        Ok(text) => return Some(text.to_string()),
        Err(value) => value,
    };
    let url = value.downcast::<CFURL>().ok()?;
    Some(url.string().to_string())
}

pub fn range(element: &AXUIElement, name: &'static str) -> Option<TextRange> {
    let value = copy(element, name).ok()?.downcast::<AXValue>().ok()?;
    let mut range = CFRange {
        location: 0,
        length: 0,
    };
    let read = unsafe {
        value.value(
            AXValueType::CFRange,
            NonNull::from(&mut range).cast::<c_void>(),
        )
    };
    (read && range.location >= 0 && range.length >= 0).then_some(TextRange {
        start: range.location as usize,
        len: range.length as usize,
    })
}

fn range_value(range: TextRange) -> Option<CFRetained<AXValue>> {
    let mut range = CFRange {
        location: range.start as isize,
        length: range.len as isize,
    };
    unsafe {
        AXValue::new(
            AXValueType::CFRange,
            NonNull::from(&mut range).cast::<c_void>(),
        )
    }
}

/// `AXStringForRange`: part of a text element's value without reading all of it.
pub fn string_for_range(element: &AXUIElement, range: TextRange) -> Option<String> {
    let parameter = range_value(range)?;
    let attribute = CFString::from_static_str("AXStringForRange");
    let mut value: *const CFType = std::ptr::null();
    let error = unsafe {
        element.copy_parameterized_attribute_value(
            &attribute,
            &parameter,
            NonNull::from(&mut value),
        )
    };
    if error != AXError::Success {
        return None;
    }
    let value =
        NonNull::new(value.cast_mut()).map(|value| unsafe { CFRetained::from_raw(value) })?;
    value
        .downcast::<CFString>()
        .ok()
        .map(|text| text.to_string())
}

pub fn set_selected_range(element: &AXUIElement, range: TextRange) -> bool {
    let Some(value) = range_value(range) else {
        return false;
    };
    let attribute = CFString::from_static_str("AXSelectedTextRange");
    unsafe { element.set_attribute_value(&attribute, &value) == AXError::Success }
}

pub fn set_bool(element: &AXUIElement, name: &'static str, value: bool) -> AXError {
    let attribute = CFString::from_static_str(name);
    unsafe { element.set_attribute_value(&attribute, CFBoolean::new(value)) }
}

pub fn has(element: &AXUIElement, name: &'static str) -> bool {
    copy(element, name).is_ok()
}

pub fn is_settable(element: &AXUIElement, name: &'static str) -> bool {
    let attribute = CFString::from_static_str(name);
    let mut settable: u8 = 0;
    let error = unsafe { element.is_attribute_settable(&attribute, NonNull::from(&mut settable)) };
    error == AXError::Success && settable != 0
}

pub fn children(element: &AXUIElement) -> Vec<Element> {
    let Some(array) = copy(element, "AXChildren")
        .ok()
        .and_then(|value| value.downcast::<CFArray>().ok())
    else {
        return Vec::new();
    };
    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(array) };
    array
        .to_vec()
        .into_iter()
        .filter_map(|child| child.downcast::<AXUIElement>().ok())
        .collect()
}

/// The strings of an array attribute such as `AXDOMClassList`.
pub fn strings(element: &AXUIElement, name: &'static str) -> Vec<String> {
    let Some(array) = copy(element, name)
        .ok()
        .and_then(|value| value.downcast::<CFArray>().ok())
    else {
        return Vec::new();
    };
    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(array) };
    array
        .to_vec()
        .into_iter()
        .filter_map(|item| item.downcast::<CFString>().ok())
        .map(|text| text.to_string())
        .collect()
}

/// A hash that is equal for the same element across reads.
pub fn hash(element: &AXUIElement) -> u64 {
    let any: &CFType = element;
    CFHash(Some(any)) as u64
}
