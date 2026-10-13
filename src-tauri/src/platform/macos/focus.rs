//! The frontmost app, and whether it has an editable text field focused (read
//! through Accessibility). Used to decide between pasting and leaving the transcript on
//! the clipboard (INS-04).

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use objc2_app_kit::NSWorkspace;
use objc2_application_services::{AXError, AXUIElement};

use super::{ax, context};
use crate::platform::{
    AppIdentity, ContextRequest, Editable, FieldContext, FocusInspector, TextRange,
};

pub const MESSAGING_TIMEOUT_SECONDS: f32 = 0.25;
const RETRY_DELAY: Duration = Duration::from_millis(80);

/// Roles of elements that accept typed text.
const TEXT_ROLES: [&str; 5] = [
    "AXTextField",
    "AXTextArea",
    "AXComboBox",
    "AXSearchField",
    "AXSecureTextField",
];
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

/// Pids whose Chromium/Electron accessibility tree Shrimp has switched on.
static MANUAL_ACCESSIBILITY: Mutex<Option<HashSet<i32>>> = Mutex::new(None);

/// The frontmost app's bundle ID and name, from `NSWorkspace`, and its
/// focused field through Accessibility.
pub struct MacFocus;

impl FocusInspector for MacFocus {
    fn frontmost_app(&self) -> Option<AppIdentity> {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let bundle_id = app
            .bundleIdentifier()
            .map(|id| id.to_string())
            .unwrap_or_default();
        let name = app
            .localizedName()
            .map(|name| name.to_string())
            .unwrap_or_default();
        Some(AppIdentity { bundle_id, name })
    }

    fn read_field(&self, request: &ContextRequest) -> Option<FieldContext> {
        let pid = frontmost_pid()?;
        if is_own(pid) || !super::permissions::accessibility_trusted() {
            return None;
        }
        context::read(pid, request)
    }

    fn select_range(&self, field_id: u64, range: TextRange) -> bool {
        let Some(pid) = frontmost_pid().filter(|pid| !is_own(*pid)) else {
            return false;
        };
        let app = ax::application(pid, MESSAGING_TIMEOUT_SECONDS);
        match ax::element(&app, "AXFocusedUIElement") {
            Some(element) if ax::hash(&element) == field_id => {
                ax::set_selected_range(&element, range)
            }
            _ => false,
        }
    }
}

pub fn frontmost_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

/// Shrimp's own windows have no dictation targets, and AX calls into our
/// own process from its main thread would deadlock.
pub fn is_own(pid: i32) -> bool {
    pid == std::process::id() as i32
}

pub fn focused_field(pid: i32) -> Editable {
    if is_own(pid) {
        return Editable::No;
    }
    let app = ax::application(pid, MESSAGING_TIMEOUT_SECONDS);
    let first_contact = enable_manual_accessibility(&app, pid);
    let (mut field, mut role) = query(&app, first_contact);
    if field == Editable::No {
        // WebKit and Chromium build their tree on the first request; ask again.
        std::thread::sleep(RETRY_DELAY);
        (field, role) = query(&app, false);
    }
    log::debug!("focused element {role:?} in pid {pid}: {field:?}");
    field
}

fn query(app: &AXUIElement, tree_just_enabled: bool) -> (Editable, Option<String>) {
    match ax::copy(app, "AXFocusedUIElement") {
        Ok(value) => match value.downcast::<AXUIElement>() {
            Ok(element) => classify(&element),
            Err(_) => (Editable::Unknown, None),
        },
        Err(AXError::NoValue) if !tree_just_enabled => (Editable::No, None),
        Err(_) => (Editable::Unknown, None),
    }
}

pub fn classify(element: &AXUIElement) -> (Editable, Option<String>) {
    let role = ax::string(element, "AXRole");
    let field = match role.as_deref() {
        Some(role) if TEXT_ROLES.contains(&role) => Editable::Yes,
        _ if ax::has(element, "AXSelectedTextRange") || ax::is_settable(element, "AXValue") => {
            Editable::Yes
        }
        Some(role) if NON_TEXT_ROLES.contains(&role) => Editable::No,
        _ => Editable::Unknown,
    };
    (field, role)
}

/// Chromium and Electron only expose their accessibility tree when asked.
/// Returns true the first time an app accepts the request; native apps
/// reject the attribute and are read normally from the start.
pub fn enable_manual_accessibility(app: &AXUIElement, pid: i32) -> bool {
    let mut seen = MANUAL_ACCESSIBILITY
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let seen = seen.get_or_insert_with(HashSet::new);
    if seen.contains(&pid) {
        return false;
    }
    seen.insert(pid);
    ax::set_bool(app, "AXManualAccessibility", true) == AXError::Success
}
