//! UI Automation on one dedicated thread (multithreaded COM), with
//! connection and transaction timeouts so an app that doesn't answer can't
//! hold a dictation up. Callers wait a bounded time for the answer, and work
//! whose caller has stopped waiting is dropped instead of piling up behind a
//! slow app (Chromium builds its tree for seconds on first contact).

use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::Ole::{SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation, IUIAutomation2, IUIAutomationElement, IUIAutomationTextPattern,
    IUIAutomationTextRange, IUIAutomationValuePattern, TextPatternRangeEndpoint_End,
    TextPatternRangeEndpoint_Start, TextUnit_Character, UIA_CONTROLTYPE_ID, UIA_TextPatternId,
    UIA_ValuePatternId,
};
use windows::core::Interface;

const CONNECTION_TIMEOUT_MS: u32 = 400;
const TRANSACTION_TIMEOUT_MS: u32 = 300;

struct Job {
    deadline: Instant,
    work: Box<dyn FnOnce(&IUIAutomation) + Send>,
}

fn worker() -> &'static Mutex<Sender<Job>> {
    static WORKER: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        let spawned = std::thread::Builder::new()
            .name("shrimp-uia".into())
            .spawn(move || {
                let Some(automation) = create() else {
                    log::warn!("UI Automation is unavailable");
                    return;
                };
                for job in rx {
                    if Instant::now() < job.deadline {
                        (job.work)(&automation);
                    }
                }
            });
        if let Err(error) = spawned {
            log::warn!("could not start UI Automation: {error}");
        }
        Mutex::new(tx)
    })
}

fn create() -> Option<IUIAutomation> {
    unsafe {
        // S_FALSE (already initialised) is fine; the thread is ours.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
        if let Ok(timeouts) = automation.cast::<IUIAutomation2>() {
            let _ = timeouts.SetConnectionTimeout(CONNECTION_TIMEOUT_MS);
            let _ = timeouts.SetTransactionTimeout(TRANSACTION_TIMEOUT_MS);
        }
        Some(automation)
    }
}

/// Runs `work` on the UI Automation thread and waits up to `timeout`.
pub fn run<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce(&IUIAutomation) -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    let job = Job {
        deadline: Instant::now() + timeout,
        work: Box::new(move |automation| {
            let _ = tx.send(work(automation));
        }),
    };
    worker()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .send(job)
        .ok()?;
    rx.recv_timeout(timeout).ok()
}

pub fn control_type(element: &IUIAutomationElement) -> UIA_CONTROLTYPE_ID {
    unsafe { element.CurrentControlType() }.unwrap_or_default()
}

pub fn class_name(element: &IUIAutomationElement) -> String {
    unsafe { element.CurrentClassName() }
        .map(|name| name.to_string())
        .unwrap_or_default()
}

pub fn name(element: &IUIAutomationElement) -> String {
    unsafe { element.CurrentName() }
        .map(|name| name.to_string())
        .unwrap_or_default()
}

pub fn help_text(element: &IUIAutomationElement) -> String {
    unsafe { element.CurrentHelpText() }
        .map(|text| text.to_string())
        .unwrap_or_default()
}

pub fn is_password(element: &IUIAutomationElement) -> bool {
    unsafe { element.CurrentIsPassword() }.is_ok_and(|password| password.as_bool())
}

pub fn process_id(element: &IUIAutomationElement) -> u32 {
    unsafe { element.CurrentProcessId() }.map_or(0, |pid| pid as u32)
}

pub fn text_pattern(element: &IUIAutomationElement) -> Option<IUIAutomationTextPattern> {
    unsafe { element.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) }.ok()
}

pub fn value_pattern(element: &IUIAutomationElement) -> Option<IUIAutomationValuePattern> {
    unsafe { element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) }.ok()
}

pub fn value(element: &IUIAutomationElement) -> Option<String> {
    let pattern = value_pattern(element)?;
    unsafe { pattern.CurrentValue() }
        .ok()
        .map(|value| value.to_string())
}

pub fn read_only(element: &IUIAutomationElement) -> Option<bool> {
    let pattern = value_pattern(element)?;
    unsafe { pattern.CurrentIsReadOnly() }
        .ok()
        .map(|read_only| read_only.as_bool())
}

/// A stable hash of the element's runtime id, to tell later whether focus
/// is still on the same field.
pub fn element_id(element: &IUIAutomationElement) -> u64 {
    let Ok(ids) = (unsafe { element.GetRuntimeId() }) else {
        return 0;
    };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    unsafe {
        let (Ok(lower), Ok(upper)) = (SafeArrayGetLBound(ids, 1), SafeArrayGetUBound(ids, 1))
        else {
            return 0;
        };
        for index in lower..=upper {
            let mut value = 0i32;
            if SafeArrayGetElement(ids, &index, (&mut value as *mut i32).cast()).is_ok() {
                for byte in value.to_le_bytes() {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x0100_0000_01b3);
                }
            }
        }
        windows::Win32::System::Ole::SafeArrayDestroy(ids).ok();
    }
    hash
}

/// The text of `range`, at most `limit` characters (`None`: all of it).
pub fn range_text(range: &IUIAutomationTextRange, limit: Option<usize>) -> Option<String> {
    let max = limit.map_or(-1, |limit| i32::try_from(limit).unwrap_or(i32::MAX));
    unsafe { range.GetText(max) }
        .ok()
        .map(|text| text.to_string())
}

/// `count` characters before the start of `range`, as a new range.
pub fn before(range: &IUIAutomationTextRange, count: usize) -> Option<IUIAutomationTextRange> {
    unsafe {
        let before = range.Clone().ok()?;
        before
            .MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                range,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        before
            .MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, -count)
            .ok()?;
        Some(before)
    }
}

/// `count` characters after the end of `range`, as a new range.
pub fn after(range: &IUIAutomationTextRange, count: usize) -> Option<IUIAutomationTextRange> {
    unsafe {
        let after = range.Clone().ok()?;
        after
            .MoveEndpointByRange(
                TextPatternRangeEndpoint_Start,
                range,
                TextPatternRangeEndpoint_End,
            )
            .ok()?;
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        after
            .MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, count)
            .ok()?;
        Some(after)
    }
}

/// The range from the start of `document` to the start of `range`.
pub fn prefix(
    document: &IUIAutomationTextRange,
    range: &IUIAutomationTextRange,
) -> Option<IUIAutomationTextRange> {
    unsafe {
        let prefix = document.Clone().ok()?;
        prefix
            .MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                range,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
        Some(prefix)
    }
}

/// Selects `len` characters from `start` in `document`.
pub fn select(document: &IUIAutomationTextRange, start: usize, len: usize) -> bool {
    let to_i32 = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
    unsafe {
        let Ok(range) = document.Clone() else {
            return false;
        };
        let collapsed = range.MoveEndpointByRange(
            TextPatternRangeEndpoint_End,
            document,
            TextPatternRangeEndpoint_Start,
        );
        collapsed.is_ok()
            && range
                .MoveEndpointByUnit(
                    TextPatternRangeEndpoint_End,
                    TextUnit_Character,
                    to_i32(start + len),
                )
                .is_ok()
            && range
                .MoveEndpointByUnit(
                    TextPatternRangeEndpoint_Start,
                    TextUnit_Character,
                    to_i32(start),
                )
                .is_ok()
            && range.Select().is_ok()
    }
}
