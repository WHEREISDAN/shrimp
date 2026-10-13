//! Reads the focused field for context awareness and Command Mode (CTX-01,
//! CMD-02): the selection, the text around it, the web page it belongs to and
//! the short texts of the window. Secure fields are never read (CTX-03).

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use objc2_application_services::AXUIElement;
use objc2_core_foundation::Type;

use super::ax::{self, Element};
use super::focus::{self, MESSAGING_TIMEOUT_SECONDS};
use crate::platform::{ContextRequest, Editable, FieldContext, FieldKind, TextRange};

const RETRY_DELAY: Duration = Duration::from_millis(80);
/// Above this, the whole value is not read to cut out the text around the cursor.
const MAX_VALUE_FALLBACK: usize = 200_000;
const MAX_PARENTS: usize = 40;
const URL_SEARCH_ELEMENTS: usize = 250;
const URL_SEARCH_DEPTH: usize = 12;
const SCAN_ELEMENTS: usize = 1_500;
const SCAN_DEPTH: usize = 45;
const SCAN_BUDGET: Duration = Duration::from_millis(150);
const SCAN_TEXTS: usize = 300;
const SCAN_TEXT_CHARS: usize = 120;
/// Code read from editors that don't have focus, in all.
const SCAN_CODE_CHARS: usize = 20_000;

/// Monaco's hidden input, and xterm.js's, in VS Code, Cursor and Windsurf.
const CODE_EDITOR_CLASS: &str = "inputarea";
const TERMINAL_CLASS: &str = "xterm-helper-textarea";

pub fn read(pid: i32, request: &ContextRequest) -> Option<FieldContext> {
    let started = Instant::now();
    let app = ax::application(pid, MESSAGING_TIMEOUT_SECONDS);
    let first_contact = focus::enable_manual_accessibility(&app, pid);
    let window = ax::element(&app, "AXFocusedWindow");
    let mut focused = ax::element(&app, "AXFocusedUIElement");
    if focused.is_none() && first_contact {
        std::thread::sleep(RETRY_DELAY);
        focused = ax::element(&app, "AXFocusedUIElement");
    }
    let mut context = FieldContext {
        editable: Editable::No,
        window_title: window
            .as_deref()
            .and_then(|window| ax::string(window, "AXTitle")),
        ..FieldContext::default()
    };
    if let Some(element) = focused.as_deref() {
        read_element(element, request, &mut context);
    }
    if context.url.is_none()
        && request.find_url
        && let Some(window) = window.as_deref()
    {
        context.url = find_web_area(window).and_then(|area| ax::url(&area, "AXURL"));
    }
    if request.scan_window
        && let Some(window) = window.as_deref()
    {
        let scanned = scan(window, focused.as_deref());
        context.screen_text = scanned.texts;
        context.code_text = scanned.code;
    }
    log::debug!(
        "read {:?} field in pid {pid} ({} chars, {} screen texts) in {} ms",
        context.kind,
        context.length,
        context.screen_text.len(),
        started.elapsed().as_millis()
    );
    Some(context)
}

fn read_element(element: &AXUIElement, request: &ContextRequest, context: &mut FieldContext) {
    let (editable, role) = focus::classify(element);
    let subrole = ax::string(element, "AXSubrole");
    context.field_id = ax::hash(element);
    context.editable = editable;
    let (web_area, in_web) = enclosing_web_area(element);
    context.in_web_content = in_web;
    context.url = web_area.and_then(|area| ax::url(&area, "AXURL"));
    let secure = [role.as_deref(), subrole.as_deref()].contains(&Some("AXSecureTextField"));
    if secure {
        context.kind = FieldKind::Secure;
        return;
    }
    let classes = ax::strings(element, "AXDOMClassList");
    context.kind = if classes.iter().any(|class| class == TERMINAL_CLASS) {
        FieldKind::Terminal
    } else if classes.iter().any(|class| class == CODE_EDITOR_CLASS) {
        FieldKind::Code
    } else {
        FieldKind::Text
    };
    if editable == Editable::No {
        return;
    }

    let length = ax::number(element, "AXNumberOfCharacters").map_or(0, |n| n.max(0) as usize);
    let placeholder = ax::string(element, "AXPlaceholderValue").filter(|text| !text.is_empty());
    let mut value = (length > 0 && length <= request.text_limit)
        .then(|| ax::string(element, "AXValue"))
        .flatten();
    if value.is_some() && value == placeholder {
        // Only the hint is showing: the field is empty.
        context.text = Some(String::new());
        return;
    }
    context.length = length;
    let Some(selection) = ax::range(element, "AXSelectedTextRange") else {
        context.text = value;
        return;
    };
    context.selection = Some(selection);
    if selection.len > 0 {
        context.selected_text = ax::string(element, "AXSelectedText").unwrap_or_default();
    }
    if request.around > 0 && length > 0 {
        let before = TextRange {
            start: selection.start.saturating_sub(request.around),
            len: selection.start.min(request.around),
        };
        let after_end = (selection.end() + request.around).min(length);
        let after = TextRange {
            start: selection.end().min(length),
            len: after_end.saturating_sub(selection.end()),
        };
        if value.is_none() {
            let parts = (
                ax::string_for_range(element, before),
                ax::string_for_range(element, after),
            );
            if let (Some(before), Some(after)) = parts {
                context.before = before;
                context.after = after;
            } else if length <= MAX_VALUE_FALLBACK {
                value = ax::string(element, "AXValue");
            }
        }
        if let Some(value) = &value {
            let units: Vec<u16> = value.encode_utf16().collect();
            context.before = utf16_slice(&units, before);
            context.after = utf16_slice(&units, after);
            if context.selected_text.is_empty() && selection.len > 0 {
                context.selected_text = utf16_slice(&units, selection);
            }
        }
    }
    context.text = value.filter(|value| value.chars().count() <= request.text_limit);
}

fn utf16_slice(units: &[u16], range: TextRange) -> String {
    let start = range.start.min(units.len());
    let end = range.end().min(units.len());
    String::from_utf16_lossy(&units[start..end])
}

/// The `AXWebArea` the element sits in, if any, and whether there is one.
fn enclosing_web_area(element: &AXUIElement) -> (Option<Element>, bool) {
    let mut current = ax::element(element, "AXParent");
    for _ in 0..MAX_PARENTS {
        let Some(node) = current else {
            break;
        };
        match ax::string(&node, "AXRole").as_deref() {
            Some("AXWebArea") => return (Some(node), true),
            Some("AXWindow" | "AXApplication") => break,
            _ => {}
        }
        current = ax::element(&node, "AXParent");
    }
    (None, false)
}

/// The first web area in the window, breadth first (the page of a browser tab).
fn find_web_area(window: &AXUIElement) -> Option<Element> {
    let mut queue: VecDeque<(Element, usize)> = ax::children(window)
        .into_iter()
        .map(|child| (child, 1))
        .collect();
    let mut visited = 0;
    while let Some((node, depth)) = queue.pop_front() {
        visited += 1;
        if visited > URL_SEARCH_ELEMENTS {
            break;
        }
        if ax::string(&node, "AXRole").as_deref() == Some("AXWebArea") {
            return Some(node);
        }
        if depth < URL_SEARCH_DEPTH {
            queue.extend(
                ax::children(&node)
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    None
}

struct Scanned {
    texts: Vec<String>,
    code: Vec<String>,
}

/// Short visible texts of the window, for names and file names on screen,
/// and the text of code editors (Monaco's "editor" text areas).
fn scan(window: &AXUIElement, focused: Option<&AXUIElement>) -> Scanned {
    let started = Instant::now();
    let skip = focused.map(ax::hash);
    let mut texts = Vec::new();
    let mut code: Vec<String> = Vec::new();
    let mut code_chars = 0;
    let mut seen = HashSet::new();
    let mut stack: Vec<(Element, usize)> = vec![(window.retain(), 0)];
    let mut visited = 0;
    while let Some((node, depth)) = stack.pop() {
        visited += 1;
        if visited > SCAN_ELEMENTS || texts.len() >= SCAN_TEXTS || started.elapsed() > SCAN_BUDGET {
            break;
        }
        let role = ax::string(&node, "AXRole").unwrap_or_default();
        if role == "AXSecureTextField" || Some(ax::hash(&node)) == skip {
            continue;
        }
        if role == "AXTextArea"
            && code_chars < SCAN_CODE_CHARS
            && ax::string(&node, "AXRoleDescription").as_deref() == Some("editor")
            && let Some(text) = ax::string(&node, "AXValue")
        {
            code_chars += text.len();
            code.push(text);
        }
        let attributes: &[&'static str] = match role.as_str() {
            "AXStaticText" => &["AXValue"],
            "AXTextField" | "AXTextArea" | "AXWebArea" | "AXScrollArea" => &[],
            _ => &["AXTitle", "AXDescription"],
        };
        for attribute in attributes {
            if let Some(text) = ax::string(&node, attribute) {
                let text = text.trim();
                let chars = text.chars().count();
                if (2..=SCAN_TEXT_CHARS).contains(&chars) && seen.insert(text.to_owned()) {
                    texts.push(text.to_owned());
                }
            }
        }
        if depth < SCAN_DEPTH {
            let children = ax::children(&node);
            stack.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
        }
    }
    Scanned { texts, code }
}
