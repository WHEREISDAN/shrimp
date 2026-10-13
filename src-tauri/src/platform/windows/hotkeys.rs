//! Global shortcuts on Windows: Shrimp's low-level keyboard and mouse hooks
//! report what happens, and one thread matches it to the bindings, records
//! new shortcuts and releases stuck ones (CORE-14). No permission is needed,
//! but the hooks don't see input meant for elevated apps.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handy_keys::{Hotkey, Key, KeyEvent};

use super::hook::{Filter, Hooks};
use crate::hotkeys::{
    Action, Binding, HotkeyEvent, Matched, Matcher, MouseEvent, RecordInput, RecordUpdate,
    Recording, blocked_buttons, blocked_hotkeys,
};
use crate::platform::{HotkeyBackend, PlatformError};

/// How often the thread wakes without input, for stuck shortcuts.
const TICK: Duration = Duration::from_millis(100);
/// A shortcut that looks held this long without a running dictation was
/// released while Shrimp wasn't looking (CORE-14).
const STUCK_AFTER: Duration = Duration::from_secs(10);

/// What the hooks report.
#[derive(Debug, Clone, Copy)]
pub enum RawInput {
    Key(KeyEvent),
    Mouse(MouseEvent),
    /// A left or right click while a shortcut is recorded.
    Click,
}

#[derive(Default)]
struct Shared {
    matcher: Matcher,
    recording: Option<Recording>,
    protected: Vec<Action>,
}

#[derive(Default)]
pub struct WinHotkeys {
    filter: Arc<Mutex<Filter>>,
    shared: Arc<Mutex<Shared>>,
    worker: Option<Worker>,
}

struct Worker {
    running: Arc<AtomicBool>,
    thread: JoinHandle<()>,
    _hooks: Hooks,
}

type OnEvent = Box<dyn Fn(HotkeyEvent) + Send + Sync>;

impl HotkeyBackend for WinHotkeys {
    fn start(&mut self, on_event: OnEvent) -> Result<(), PlatformError> {
        if self.worker.is_some() {
            return Ok(());
        }
        let (tx, rx) = mpsc::channel();
        let hooks = Hooks::start(Arc::clone(&self.filter), tx)?;
        let running = Arc::new(AtomicBool::new(true));
        let thread = std::thread::Builder::new()
            .name("shrimp-hotkeys".into())
            .spawn({
                let running = Arc::clone(&running);
                let shared = Arc::clone(&self.shared);
                move || forward(&rx, &shared, &running, on_event.as_ref())
            })
            .map_err(|err| PlatformError::Failed(format!("could not start the hotkeys: {err}")))?;
        self.worker = Some(Worker {
            running,
            thread,
            _hooks: hooks,
        });
        Ok(())
    }

    fn set_bindings(&mut self, bindings: &[Binding]) {
        let mut shared = lock(&self.shared);
        shared.matcher.set_bindings(bindings);
        let mut filter = lock(&self.filter);
        if shared.recording.is_none() {
            filter.hotkeys = blocked_hotkeys(bindings);
        }
        filter.buttons = blocked_buttons(bindings);
    }

    fn set_protected(&mut self, actions: &[Action]) {
        lock(&self.shared).protected = actions.to_vec();
    }

    fn set_recording(&mut self, recording: Option<Recording>) {
        let mut shared = lock(&self.shared);
        let active = recording.is_some();
        shared.recording = recording;
        let mut filter = lock(&self.filter);
        filter.recording = active;
        filter.hotkeys = if active {
            // Caps Lock records on press; swallowing it keeps capitals off.
            HashSet::from([Hotkey {
                modifiers: handy_keys::Modifiers::empty(),
                key: Some(Key::CapsLock),
            }])
        } else {
            blocked_hotkeys(shared.matcher.bindings())
        };
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.running.store(false, Ordering::Relaxed);
            let _ = worker.thread.join();
        }
    }
}

impl Drop for WinHotkeys {
    fn drop(&mut self) {
        self.stop();
    }
}

fn forward(
    rx: &Receiver<RawInput>,
    shared: &Mutex<Shared>,
    running: &AtomicBool,
    on_event: &(dyn Fn(HotkeyEvent) + Send + Sync),
) {
    while running.load(Ordering::Relaxed) {
        let events = match rx.recv_timeout(TICK) {
            Ok(input) => handle(&mut lock(shared), input),
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let events = events.into_iter().chain(tick(&mut lock(shared)));
        events.for_each(on_event);
    }
}

fn handle(shared: &mut Shared, input: RawInput) -> Vec<HotkeyEvent> {
    if let Some(recording) = shared.recording.as_mut() {
        let input = match input {
            RawInput::Key(event) => RecordInput::Key(event),
            RawInput::Mouse(event) => RecordInput::Mouse(event),
            RawInput::Click => RecordInput::Click,
        };
        let Some(update) = recording.handle(input) else {
            return Vec::new();
        };
        if !matches!(update, RecordUpdate::Preview(_)) {
            shared.recording = None;
        }
        return vec![HotkeyEvent::Recording(update)];
    }
    let now = Instant::now();
    let matched = match input {
        // Windows reports Caps Lock's release, which the matcher otherwise
        // learns from polling.
        RawInput::Key(event) if event.key == Some(Key::CapsLock) && !event.is_key_down => {
            shared.matcher.release_key(Key::CapsLock)
        }
        RawInput::Key(event) => shared.matcher.process(&event, now),
        RawInput::Mouse(event) => shared.matcher.process_mouse(&event, now),
        RawInput::Click => Vec::new(),
    };
    matched.into_iter().map(to_event).collect()
}

fn tick(shared: &mut Shared) -> Vec<HotkeyEvent> {
    let protected = shared.protected.clone();
    shared
        .matcher
        .release_stuck(Instant::now(), STUCK_AFTER, &protected)
        .into_iter()
        .map(to_event)
        .collect()
}

fn to_event(matched: Matched) -> HotkeyEvent {
    match matched {
        Matched::Shortcut(event) => HotkeyEvent::Shortcut(event),
        Matched::OtherKey => HotkeyEvent::OtherKey,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}
