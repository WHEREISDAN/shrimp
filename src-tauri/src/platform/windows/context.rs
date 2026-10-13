//! Reads the focused field through UI Automation for context awareness,
//! Command Mode and auto-learn (CTX-01, CMD-02, DIC-07, WIN-11): the
//! selection, the text around it, the page it belongs to and the short texts
//! of the window. Password fields are never read (CTX-03); a field showing
//! only its placeholder counts as empty (WIN-07).

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker, UIA_ButtonControlTypeId,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_DataItemControlTypeId,
    UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ImageControlTypeId, UIA_ListControlTypeId, UIA_ListItemControlTypeId,
    UIA_MenuBarControlTypeId, UIA_MenuControlTypeId, UIA_MenuItemControlTypeId,
    UIA_PaneControlTypeId, UIA_RadioButtonControlTypeId, UIA_SliderControlTypeId,
    UIA_TabItemControlTypeId, UIA_TableControlTypeId, UIA_TextControlTypeId, UIA_TreeControlTypeId,
    UIA_TreeItemControlTypeId, UIA_WindowControlTypeId,
};

use super::{process, uia};
use crate::platform::{ContextRequest, Editable, FieldContext, FieldKind, TextRange};

/// Above this, the whole text is not read to cut out the text around the cursor.
const MAX_TEXT: usize = 200_000;
const MAX_PARENTS: usize = 40;
const URL_SEARCH_ELEMENTS: usize = 250;
const URL_SEARCH_DEPTH: usize = 12;
const SCAN_ELEMENTS: usize = 1_500;
const SCAN_DEPTH: usize = 45;
const SCAN_BUDGET: Duration = Duration::from_millis(150);
const SCAN_TEXTS: usize = 300;
const SCAN_TEXT_CHARS: usize = 120;
const SCAN_CODE_CHARS: usize = 20_000;
const RETRY_DELAY: Duration = Duration::from_millis(80);

/// Monaco's input (older and newer VS Code builds) and xterm.js's.
const CODE_EDITOR_CLASSES: [&str; 2] = ["inputarea", "native-edit-context"];
const TERMINAL_CLASS: &str = "xterm-helper-textarea";

/// Whether the element takes typed text.
pub fn classify(element: &IUIAutomationElement) -> Editable {
    let kind = uia::control_type(element);
    if kind == UIA_EditControlTypeId {
        return if uia::read_only(element) == Some(true) {
            Editable::No
        } else {
            Editable::Yes
        };
    }
    if kind == UIA_DocumentControlTypeId || kind == UIA_ComboBoxControlTypeId {
        return match uia::read_only(element) {
            Some(false) => Editable::Yes,
            Some(true) if kind == UIA_DocumentControlTypeId => Editable::No,
            _ => Editable::Unknown,
        };
    }
    let never_text = [
        UIA_ButtonControlTypeId,
        UIA_CheckBoxControlTypeId,
        UIA_RadioButtonControlTypeId,
        UIA_HyperlinkControlTypeId,
        UIA_ImageControlTypeId,
        UIA_ListControlTypeId,
        UIA_ListItemControlTypeId,
        UIA_MenuControlTypeId,
        UIA_MenuBarControlTypeId,
        UIA_MenuItemControlTypeId,
        UIA_SliderControlTypeId,
        UIA_TabItemControlTypeId,
        UIA_TableControlTypeId,
        UIA_DataItemControlTypeId,
        UIA_TreeControlTypeId,
        UIA_TreeItemControlTypeId,
        UIA_WindowControlTypeId,
        UIA_PaneControlTypeId,
    ];
    if never_text.contains(&kind) && uia::text_pattern(element).is_none() {
        return Editable::No;
    }
    Editable::Unknown
}

/// The focused element, unless it belongs to Shrimp; asked twice, because
/// Chromium builds its tree on the first request.
pub fn focused(automation: &IUIAutomation) -> Option<IUIAutomationElement> {
    let element = unsafe { automation.GetFocusedElement() }.ok().or_else(|| {
        std::thread::sleep(RETRY_DELAY);
        unsafe { automation.GetFocusedElement() }.ok()
    })?;
    (uia::process_id(&element) != process::own_pid()).then_some(element)
}

pub fn read(automation: &IUIAutomation, request: &ContextRequest) -> Option<FieldContext> {
    let started = Instant::now();
    let window = process::foreground_window()?;
    if process::window_pid(window) == process::own_pid() {
        return None;
    }
    let walker = unsafe { automation.ControlViewWalker() }.ok()?;
    let mut context = FieldContext {
        editable: Editable::No,
        window_title: process::window_title(window),
        ..FieldContext::default()
    };
    let focused = focused(automation);
    if let Some(element) = &focused {
        read_element(element, &walker, request, &mut context);
    }
    let root = unsafe { automation.ElementFromHandle(window) }.ok();
    if context.url.is_none()
        && request.find_url
        && let Some(root) = &root
    {
        context.url = find_document(&walker, root).and_then(|document| page_url(&document));
    }
    if request.scan_window
        && let Some(root) = &root
    {
        let scanned = scan(&walker, root, focused.as_ref());
        context.screen_text = scanned.texts;
        context.code_text = scanned.code;
    }
    log::debug!(
        "read {:?} field ({} chars, {} screen texts) in {} ms",
        context.kind,
        context.length,
        context.screen_text.len(),
        started.elapsed().as_millis()
    );
    Some(context)
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn utf16_slice(units: &[u16], range: TextRange) -> String {
    let start = range.start.min(units.len());
    let end = range.end().min(units.len());
    String::from_utf16_lossy(&units[start..end])
}

/// The last `count` UTF-16 units of `text`.
fn tail(text: &str, count: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16_lossy(&units[units.len().saturating_sub(count)..])
}

/// A page's address: Chromium and Firefox give it as the document's value.
fn page_url(document: &IUIAutomationElement) -> Option<String> {
    uia::value(document).filter(|value| value.contains("://"))
}

/// The document the element sits in, if any.
fn enclosing_document(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
) -> Option<IUIAutomationElement> {
    let mut current = unsafe { walker.GetParentElement(element) }.ok();
    for _ in 0..MAX_PARENTS {
        let node = current?;
        let kind = uia::control_type(&node);
        if kind == UIA_DocumentControlTypeId {
            return Some(node);
        }
        if kind == UIA_WindowControlTypeId {
            return None;
        }
        current = unsafe { walker.GetParentElement(&node) }.ok();
    }
    None
}

/// The first document in the window, breadth first (a browser tab's page).
fn find_document(
    walker: &IUIAutomationTreeWalker,
    root: &IUIAutomationElement,
) -> Option<IUIAutomationElement> {
    let mut queue: VecDeque<(IUIAutomationElement, usize)> = VecDeque::from([(root.clone(), 0)]);
    let mut visited = 0;
    while let Some((node, depth)) = queue.pop_front() {
        visited += 1;
        if visited > URL_SEARCH_ELEMENTS {
            break;
        }
        if depth > 0 && uia::control_type(&node) == UIA_DocumentControlTypeId {
            return Some(node);
        }
        if depth < URL_SEARCH_DEPTH {
            queue.extend(
                children(walker, &node)
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    None
}

fn children(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
) -> Vec<IUIAutomationElement> {
    let mut children = Vec::new();
    let mut child = unsafe { walker.GetFirstChildElement(element) }.ok();
    while let Some(node) = child {
        child = unsafe { walker.GetNextSiblingElement(&node) }.ok();
        children.push(node);
        if children.len() > 500 {
            break;
        }
    }
    children
}

fn field_kind(element: &IUIAutomationElement) -> FieldKind {
    if uia::is_password(element) {
        return FieldKind::Secure;
    }
    let classes = uia::class_name(element);
    let has = |class: &str| classes.split_whitespace().any(|name| name == class);
    if has(TERMINAL_CLASS) {
        FieldKind::Terminal
    } else if CODE_EDITOR_CLASSES.iter().any(|class| has(class)) {
        FieldKind::Code
    } else {
        FieldKind::Text
    }
}

/// Text that is really a placeholder: the field's help text, or its name
/// with the cursor at the very start (Chromium shows CSS placeholders as text).
fn is_placeholder(
    element: &IUIAutomationElement,
    text: &str,
    selection: Option<TextRange>,
) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    let help = uia::help_text(element);
    if help.trim() == text {
        return true;
    }
    let at_start = selection.is_none_or(|range| range.start == 0 && range.len == 0);
    at_start && uia::name(element).trim() == text
}

fn read_element(
    element: &IUIAutomationElement,
    walker: &IUIAutomationTreeWalker,
    request: &ContextRequest,
    context: &mut FieldContext,
) {
    context.field_id = uia::element_id(element);
    context.editable = classify(element);
    let document = enclosing_document(walker, element);
    context.in_web_content = document.is_some();
    context.url = document.as_ref().and_then(page_url);
    context.kind = field_kind(element);
    if context.kind == FieldKind::Secure || context.editable == Editable::No {
        return;
    }
    let Some(pattern) = uia::text_pattern(element) else {
        // Plain value fields without text ranges.
        let value = uia::value(element).unwrap_or_default();
        if is_placeholder(element, &value, None) {
            context.text = Some(String::new());
            return;
        }
        context.length = utf16_len(&value);
        context.text = (context.length <= request.text_limit).then_some(value);
        return;
    };
    let Ok(document_range) = (unsafe { pattern.DocumentRange() }) else {
        return;
    };
    let selection = unsafe { pattern.GetSelection() }
        .ok()
        .filter(|ranges| unsafe { ranges.Length() }.is_ok_and(|length| length > 0))
        .and_then(|ranges| unsafe { ranges.GetElement(0) }.ok());
    let whole = uia::range_text(&document_range, Some(MAX_TEXT + 1)).unwrap_or_default();
    let units: Vec<u16> = whole.encode_utf16().collect();
    let complete = units.len() <= MAX_TEXT;
    let span = match (&selection, complete) {
        (Some(selected), true) => {
            let start = uia::prefix(&document_range, selected)
                .and_then(|prefix| uia::range_text(&prefix, Some(MAX_TEXT + 1)))
                .map(|text| utf16_len(&text));
            let len = uia::range_text(selected, Some(MAX_TEXT + 1)).map(|text| utf16_len(&text));
            start.zip(len).map(|(start, len)| TextRange { start, len })
        }
        _ => None,
    };
    if complete && is_placeholder(element, &whole, span) {
        context.text = Some(String::new());
        return;
    }
    context.length = units.len();
    context.selection = span;
    if let Some(range) = span {
        context.selected_text = utf16_slice(&units, range);
        if request.around > 0 {
            let before = TextRange {
                start: range.start.saturating_sub(request.around),
                len: range.start.min(request.around),
            };
            let after_end = (range.end() + request.around).min(units.len());
            let after = TextRange {
                start: range.end().min(units.len()),
                len: after_end.saturating_sub(range.end()),
            };
            context.before = utf16_slice(&units, before);
            context.after = utf16_slice(&units, after);
        }
    } else if let Some(selected) = &selection {
        // A long document: read only around the selection.
        context.selected_text = uia::range_text(selected, Some(MAX_TEXT)).unwrap_or_default();
        if request.around > 0 {
            context.before = uia::before(selected, request.around)
                .and_then(|range| uia::range_text(&range, None))
                .map(|text| tail(&text, request.around))
                .unwrap_or_default();
            context.after = uia::after(selected, request.around)
                .and_then(|range| uia::range_text(&range, Some(request.around)))
                .unwrap_or_default();
        }
    }
    context.text = (complete && units.len() <= request.text_limit).then_some(whole);
}

/// Selects `range` in the focused field if it is still `field_id`.
pub fn select_range(automation: &IUIAutomation, field_id: u64, range: TextRange) -> bool {
    let Some(element) = focused(automation) else {
        return false;
    };
    if uia::element_id(&element) != field_id {
        return false;
    }
    let Some(pattern) = uia::text_pattern(&element) else {
        return false;
    };
    let Ok(document) = (unsafe { pattern.DocumentRange() }) else {
        return false;
    };
    uia::select(&document, range.start, range.len)
}

struct Scanned {
    texts: Vec<String>,
    code: Vec<String>,
}

/// Short visible texts of the window, for names and file names on screen,
/// and the text of code editors that don't have focus.
fn scan(
    walker: &IUIAutomationTreeWalker,
    root: &IUIAutomationElement,
    focused: Option<&IUIAutomationElement>,
) -> Scanned {
    let started = Instant::now();
    let skip = focused.map(uia::element_id);
    let mut texts = Vec::new();
    let mut code = Vec::new();
    let mut code_chars = 0;
    let mut seen = HashSet::new();
    let mut stack: Vec<(IUIAutomationElement, usize)> = vec![(root.clone(), 0)];
    let mut visited = 0;
    while let Some((node, depth)) = stack.pop() {
        visited += 1;
        if visited > SCAN_ELEMENTS || texts.len() >= SCAN_TEXTS || started.elapsed() > SCAN_BUDGET {
            break;
        }
        if uia::is_password(&node) || Some(uia::element_id(&node)) == skip {
            continue;
        }
        let kind = uia::control_type(&node);
        if kind == UIA_EditControlTypeId {
            if code_chars < SCAN_CODE_CHARS
                && field_kind(&node) == FieldKind::Code
                && let Some(text) = uia::value(&node).filter(|text| !text.is_empty())
            {
                code_chars += text.len();
                code.push(text);
            }
        } else if kind != UIA_DocumentControlTypeId {
            let text = uia::name(&node);
            let text = text.trim();
            let chars = text.chars().count();
            if (2..=SCAN_TEXT_CHARS).contains(&chars) && seen.insert(text.to_owned()) {
                texts.push(text.to_owned());
            }
        }
        if depth < SCAN_DEPTH && kind != UIA_TextControlTypeId {
            let children = children(walker, &node);
            stack.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
        }
    }
    Scanned { texts, code }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_by_utf16_units() {
        let units: Vec<u16> = "a😀b".encode_utf16().collect();
        assert_eq!(utf16_slice(&units, TextRange { start: 1, len: 2 }), "😀");
        assert_eq!(utf16_slice(&units, TextRange { start: 3, len: 9 }), "b");
        assert_eq!(tail("hello world", 5), "world");
        assert_eq!(utf16_len("😀"), 2);
    }
}
