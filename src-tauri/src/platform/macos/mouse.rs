//! Mouse-button shortcuts (CORE-13): an active `CGEventTap` for the middle
//! button and Mouse 4 to 10. Bound buttons are swallowed whatever modifiers
//! are held. While a shortcut is recorded, every extra button is swallowed,
//! and a left or right click cancels the recording without reaching its
//! target (KEYS-06). handy-keys reports only three extra buttons and never
//! blocks them, hence this tap.

use std::cell::Cell;
use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use handy_keys::Modifiers;
use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, kCFRunLoopCommonModes};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventMask, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};

use super::hotkeys::RawInput;
use crate::hotkeys::{MouseButton, MouseEvent};
use crate::platform::PlatformError;

/// What the tap swallows.
#[derive(Debug, Default)]
pub struct MouseFilter {
    pub blocked: HashSet<MouseButton>,
    pub recording: bool,
    /// A click that cancelled a recording; its release is swallowed too.
    swallow_left_up: bool,
    swallow_right_up: bool,
}

struct Context {
    filter: Arc<Mutex<MouseFilter>>,
    events: Sender<RawInput>,
    tap: Cell<Option<NonNull<CFMachPort>>>,
}

/// The tap's run loop, stopped from any thread.
struct TapLoop(CFRetained<CFRunLoop>);

// SAFETY: CFRunLoop is documented as thread-safe; objc2 doesn't mark it Send.
unsafe impl Send for TapLoop {}

pub struct MouseTap {
    run_loop: TapLoop,
    handle: Option<JoinHandle<()>>,
}

impl MouseTap {
    pub fn start(
        filter: Arc<Mutex<MouseFilter>>,
        events: Sender<RawInput>,
    ) -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("shrimp-mouse".into())
            .spawn(move || run(filter, events, &ready_tx))
            .map_err(|err| {
                PlatformError::Failed(format!("could not start the mouse hook: {err}"))
            })?;
        let run_loop = ready_rx
            .recv()
            .map_err(|_| PlatformError::Failed("the mouse hook stopped".into()))?
            .map_err(PlatformError::Failed)?;
        Ok(Self {
            run_loop,
            handle: Some(handle),
        })
    }
}

impl Drop for MouseTap {
    fn drop(&mut self) {
        // A block survives until the loop is entered, unlike CFRunLoopStop.
        let block = block2::StackBlock::new(|| {
            if let Some(current) = CFRunLoop::current() {
                current.stop();
            }
        });
        unsafe {
            self.run_loop.0.perform_block(
                kCFRunLoopCommonModes.map(|mode| mode.as_ref()),
                Some(&block),
            );
        }
        self.run_loop.0.wake_up();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn modifiers(flags: CGEventFlags) -> Modifiers {
    [
        (CGEventFlags::MaskControl, Modifiers::CTRL_LEFT),
        (CGEventFlags::MaskAlternate, Modifiers::OPT_LEFT),
        (CGEventFlags::MaskShift, Modifiers::SHIFT_LEFT),
        (CGEventFlags::MaskCommand, Modifiers::CMD_LEFT),
        (CGEventFlags::MaskSecondaryFn, Modifiers::FN),
    ]
    .into_iter()
    .filter(|(flag, _)| flags.contains(*flag))
    .fold(Modifiers::empty(), |acc, (_, modifier)| acc | modifier)
}

unsafe extern "C-unwind" fn callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: user_info is the Context owned by `run`, alive while the tap runs.
    let context = unsafe { &*(user_info as *const Context) };
    if matches!(
        event_type,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        if let Some(tap) = context.tap.get() {
            CGEvent::tap_enable(unsafe { tap.as_ref() }, true);
        }
        return event.as_ptr();
    }
    let Ok(mut filter) = context.filter.lock() else {
        return event.as_ptr();
    };
    // SAFETY: the event is valid for the duration of the callback.
    let cg_event = unsafe { event.as_ref() };
    let block = match event_type {
        CGEventType::OtherMouseDown | CGEventType::OtherMouseUp => {
            let number =
                CGEvent::integer_value_field(Some(cg_event), CGEventField::MouseEventButtonNumber);
            match MouseButton::new(number) {
                Some(button) => {
                    let _ = context.events.send(RawInput::Mouse(MouseEvent {
                        button,
                        down: event_type == CGEventType::OtherMouseDown,
                        modifiers: modifiers(CGEvent::flags(Some(cg_event))),
                    }));
                    filter.recording || filter.blocked.contains(&button)
                }
                None => false,
            }
        }
        CGEventType::LeftMouseDown | CGEventType::RightMouseDown if filter.recording => {
            let _ = context.events.send(RawInput::Click);
            if event_type == CGEventType::LeftMouseDown {
                filter.swallow_left_up = true;
            } else {
                filter.swallow_right_up = true;
            }
            true
        }
        CGEventType::LeftMouseUp => std::mem::take(&mut filter.swallow_left_up),
        CGEventType::RightMouseUp => std::mem::take(&mut filter.swallow_right_up),
        _ => false,
    };
    if block {
        std::ptr::null_mut()
    } else {
        event.as_ptr()
    }
}

fn run(
    filter: Arc<Mutex<MouseFilter>>,
    events: Sender<RawInput>,
    ready: &Sender<Result<TapLoop, String>>,
) {
    let mask: CGEventMask = [
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseUp,
        CGEventType::RightMouseDown,
        CGEventType::RightMouseUp,
    ]
    .iter()
    .fold(0, |mask, event_type| mask | (1 << event_type.0));
    let context = Box::into_raw(Box::new(Context {
        filter,
        events,
        tap: Cell::new(None),
    }));
    // SAFETY: the callback and context stay valid until the tap is invalidated below.
    let tap = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            mask,
            Some(callback),
            context.cast(),
        )
    };
    let Some(tap) = tap else {
        drop(unsafe { Box::from_raw(context) });
        let _ = ready.send(Err("could not create the mouse event tap".into()));
        return;
    };
    unsafe { (*context).tap.set(Some(NonNull::from(&*tap))) };
    let (Some(source), Some(run_loop)) = (
        CFMachPort::new_run_loop_source(None, Some(&tap), 0),
        CFRunLoop::current(),
    ) else {
        CFMachPort::invalidate(&tap);
        drop(unsafe { Box::from_raw(context) });
        let _ = ready.send(Err("could not run the mouse event tap".into()));
        return;
    };
    let modes = unsafe { kCFRunLoopCommonModes };
    run_loop.add_source(Some(&source), modes);
    CGEvent::tap_enable(&tap, true);
    let _ = ready.send(Ok(TapLoop(run_loop.clone())));
    CFRunLoop::run();
    run_loop.remove_source(Some(&source), modes);
    CGEvent::tap_enable(&tap, false);
    CFMachPort::invalidate(&tap);
    drop(unsafe { Box::from_raw(context) });
}
