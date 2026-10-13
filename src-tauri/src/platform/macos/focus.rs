//! Whether the frontmost app has an editable text field focused, read through
//! Accessibility. Used to decide between pasting and leaving the transcript on
//! the clipboard (INS-04).

use std::collections::HashSet;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::time::Duration;

use objc2_app_kit::NSWorkspace;
use objc2_application_services::{AXError, AXUIElement};
use objc2_core_foundation::{CFBoolean, CFRetained, CFString, CFType};

const MESSAGING_TIMEOUT_SECONDS: f32 = 0.25;
const RETRY_DELAY: Duration = Duration::from_millis(80);

/// Roles of elements that accept typed text.
const TEXT_ROLES: [&str; 4] = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"];
/// Roles that are never text input, even when they hold keyboard focus.
const NON_TEXT_ROLES: [&str; 14] = [
    "AXButton",
    "AXCheckBox",
    "AXRadioButton",
    "AXPopUpButton",
    "AXMenuButton",
    "AXMenuItem",
    "AXList",
    "AXOutline",
    "AXTable",
    "AXRow",
    "AXCell",
    "AXImage",
    "AXSlider",
    "AXWindow",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedField {
    Editable,
    NotEditable,
    /// The app does not report its focus reliably; paste and hope.
    Unknown,
}

/// Pids whose Chromium/Electron accessibility tree Shrimp has switched on.
static MANUAL_ACCESSIBILITY: Mutex<Option<HashSet<i32>>> = Mutex::new(None);

pub fn frontmost_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

pub fn focused_field(pid: i32) -> FocusedField {
    if pid == std::process::id() as i32 {
        // Shrimp's own windows have no dictation targets, and AX calls into
        // our own process from its main thread would deadlock.
        return FocusedField::NotEditable;
    }
    let app = unsafe { AXUIElement::new_application(pid) };
    unsafe { app.set_messaging_timeout(MESSAGING_TIMEOUT_SECONDS) };
    let first_contact = enable_manual_accessibility(&app, pid);
    let (mut field, mut role) = query(&app, first_contact);
    if field == FocusedField::NotEditable {
        // WebKit and Chromium build their tree on the first request; ask again.
        std::thread::sleep(RETRY_DELAY);
        (field, role) = query(&app, false);
    }
    log::debug!("focused element {role:?} in pid {pid}: {field:?}");
    field
}

fn query(app: &AXUIElement, tree_just_enabled: bool) -> (FocusedField, Option<String>) {
    match copy_attribute(app, "AXFocusedUIElement") {
        Ok(value) => match value.downcast::<AXUIElement>() {
            Ok(element) => classify(&element),
            Err(_) => (FocusedField::Unknown, None),
        },
        Err(AXError::NoValue) if !tree_just_enabled => (FocusedField::NotEditable, None),
        Err(_) => (FocusedField::Unknown, None),
    }
}

fn classify(element: &AXUIElement) -> (FocusedField, Option<String>) {
    let role = copy_attribute(element, "AXRole")
        .ok()
        .and_then(|value| value.downcast::<CFString>().ok())
        .map(|role| role.to_string());
    let field = match role.as_deref() {
        Some(role) if TEXT_ROLES.contains(&role) => FocusedField::Editable,
        _ if has_attribute(element, "AXSelectedTextRange") || is_settable(element, "AXValue") => {
            FocusedField::Editable
        }
        Some(role) if NON_TEXT_ROLES.contains(&role) => FocusedField::NotEditable,
        _ => FocusedField::Unknown,
    };
    (field, role)
}

/// Chromium and Electron only expose their accessibility tree when asked.
/// Returns true the first time an app accepts the request; native apps
/// reject the attribute and are read normally from the start.
fn enable_manual_accessibility(app: &AXUIElement, pid: i32) -> bool {
    let mut seen = MANUAL_ACCESSIBILITY
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let seen = seen.get_or_insert_with(HashSet::new);
    if seen.contains(&pid) {
        return false;
    }
    seen.insert(pid);
    let attribute = CFString::from_static_str("AXManualAccessibility");
    let value = CFBoolean::new(true);
    let result = unsafe { app.set_attribute_value(&attribute, value) };
    result == AXError::Success
}

fn copy_attribute(
    element: &AXUIElement,
    name: &'static str,
) -> Result<CFRetained<CFType>, AXError> {
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

fn has_attribute(element: &AXUIElement, name: &'static str) -> bool {
    copy_attribute(element, name).is_ok()
}

fn is_settable(element: &AXUIElement, name: &'static str) -> bool {
    let attribute = CFString::from_static_str(name);
    let mut settable: u8 = 0;
    let error = unsafe { element.is_attribute_settable(&attribute, NonNull::from(&mut settable)) };
    error == AXError::Success && settable != 0
}
