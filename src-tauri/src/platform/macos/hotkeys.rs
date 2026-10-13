//! Global shortcuts: handy-keys' active `CGEventTap` for keys, Fn and
//! modifier-only chords, and Shrimp's own tap for mouse buttons (both need
//! Accessibility). One thread matches what they report, records new
//! shortcuts, releases stuck ones (CORE-14), watches a bound Caps Lock's
//! release (KEYS-04) and Secure Keyboard Entry (KEYS-09).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handy_keys::{BlockingHotkeys, Hotkey, Key, KeyEvent, KeyboardListener, Modifiers};

use super::iokit;
use super::mouse::{MouseFilter, MouseTap};
use super::permissions::accessibility_trusted;
use super::secure_input::secure_input_owner;
use crate::hotkeys::{
    Action, Binding, HotkeyEvent, Matched, Matcher, MouseEvent, RecordInput, RecordUpdate,
    Recording, blocked_buttons, blocked_hotkeys,
};
use crate::platform::{HotkeyBackend, PlatformError};

/// How often the thread wakes without input, for Caps Lock and stuck keys.
const TICK: Duration = Duration::from_millis(40);
/// A shortcut that looks held this long without a running dictation was
/// released while Shrimp wasn't looking (CORE-14).
const STUCK_AFTER: Duration = Duration::from_secs(10);
const SECURE_INPUT_EVERY: Duration = Duration::from_secs(2);

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
pub struct MacHotkeys {
    blocking: BlockingHotkeys,
    mouse: Arc<Mutex<MouseFilter>>,
    shared: Arc<Mutex<Shared>>,
    caps_lock_bound: bool,
    worker: Option<Worker>,
}

struct Worker {
    running: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    _mouse: Option<MouseTap>,
}

type OnEvent = Box<dyn Fn(HotkeyEvent) + Send + Sync>;

impl HotkeyBackend for MacHotkeys {
    fn start(&mut self, on_event: OnEvent) -> Result<(), PlatformError> {
        if self.worker.is_some() {
            return Ok(());
        }
        if !accessibility_trusted() {
            return Err(PlatformError::AccessibilityMissing);
        }
        let listener = KeyboardListener::new_with_blocking(Arc::clone(&self.blocking))
            .map_err(|err| PlatformError::Failed(format!("keyboard hook failed: {err}")))?;
        let (tx, rx) = mpsc::channel();
        let mouse = MouseTap::start(Arc::clone(&self.mouse), tx.clone())
            .inspect_err(|error| log::warn!("mouse shortcuts unavailable: {error}"))
            .ok();
        let running = Arc::new(AtomicBool::new(true));
        let relay = spawn("shrimp-keys", {
            let running = Arc::clone(&running);
            move || relay(&listener, &tx, &running)
        })?;
        let forward = spawn("shrimp-hotkeys", {
            let running = Arc::clone(&running);
            let shared = Arc::clone(&self.shared);
            move || forward(&rx, &shared, &running, on_event.as_ref())
        })?;
        self.worker = Some(Worker {
            running,
            threads: vec![relay, forward],
            _mouse: mouse,
        });
        Ok(())
    }

    fn set_bindings(&mut self, bindings: &[Binding]) {
        let mut shared = lock(&self.shared);
        shared.matcher.set_bindings(bindings);
        if shared.recording.is_none() {
            *lock(&self.blocking) = blocked_hotkeys(bindings);
        }
        lock(&self.mouse).blocked = blocked_buttons(bindings);
        let caps_lock = bindings
            .iter()
            .any(|binding| binding.trigger.key() == Some(Key::CapsLock));
        // A bound Caps Lock starts from capitals off and stays there (KEYS-04).
        if caps_lock && !self.caps_lock_bound && iokit::caps_lock_on() == Some(true) {
            iokit::set_caps_lock(false);
        }
        self.caps_lock_bound = caps_lock;
    }

    fn set_protected(&mut self, actions: &[Action]) {
        lock(&self.shared).protected = actions.to_vec();
    }

    fn set_recording(&mut self, recording: Option<Recording>) {
        let mut shared = lock(&self.shared);
        let active = recording.is_some();
        shared.recording = recording;
        *lock(&self.blocking) = if active {
            // Fn alone would open the emoji picker while it is recorded.
            HashSet::from([Hotkey {
                modifiers: Modifiers::FN,
                key: None,
            }])
        } else {
            blocked_hotkeys(shared.matcher.bindings())
        };
        lock(&self.mouse).recording = active;
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.running.store(false, Ordering::Relaxed);
            for thread in worker.threads {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for MacHotkeys {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn(
    name: &str,
    work: impl FnOnce() + Send + 'static,
) -> Result<JoinHandle<()>, PlatformError> {
    std::thread::Builder::new()
        .name(name.into())
        .spawn(work)
        .map_err(|err| PlatformError::Failed(format!("could not start {name}: {err}")))
}

/// Moves handy-keys' events onto the channel the mouse tap also feeds.
fn relay(listener: &KeyboardListener, tx: &Sender<RawInput>, running: &AtomicBool) {
    while running.load(Ordering::Relaxed) {
        match listener.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if tx.send(RawInput::Key(event)).is_err() {
                    return;
                }
            }
            Err(handy_keys::Error::Timeout) => {}
            Err(err) => {
                log::error!("keyboard hook stopped: {err}");
                return;
            }
        }
    }
}

fn forward(
    rx: &Receiver<RawInput>,
    shared: &Mutex<Shared>,
    running: &AtomicBool,
    on_event: &(dyn Fn(HotkeyEvent) + Send + Sync),
) {
    let mut secure_input: Option<String> = None;
    let mut secure_checked = Instant::now() - SECURE_INPUT_EVERY;
    while running.load(Ordering::Relaxed) {
        let events = match rx.recv_timeout(TICK) {
            Ok(input) => handle(&mut lock(shared), input),
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let events = events.into_iter().chain(tick(&mut lock(shared)));
        events.for_each(on_event);
        if secure_checked.elapsed() >= SECURE_INPUT_EVERY {
            secure_checked = Instant::now();
            let owner = secure_input_owner();
            if owner != secure_input {
                log::info!("Secure Keyboard Entry: {owner:?}");
                secure_input.clone_from(&owner);
                on_event(HotkeyEvent::SecureInput(owner));
            }
        }
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
        if let RecordUpdate::Done(recorded) = update
            && recorded.trigger.key() == Some(Key::CapsLock)
        {
            iokit::set_caps_lock(false);
        }
        if !matches!(update, RecordUpdate::Preview(_)) {
            shared.recording = None;
        }
        return vec![HotkeyEvent::Recording(update)];
    }
    let now = Instant::now();
    let matched = match input {
        RawInput::Key(event) => {
            let matched = shared.matcher.process(&event, now);
            let caps_pressed = event.key == Some(Key::CapsLock)
                && matched
                    .iter()
                    .any(|m| matches!(m, Matched::Shortcut(event) if event.pressed));
            if caps_pressed {
                // The press turned capitals on; a Caps Lock shortcut never types capitals.
                iokit::set_caps_lock(false);
            }
            matched
        }
        RawInput::Mouse(event) => shared.matcher.process_mouse(&event, now),
        RawInput::Click => Vec::new(),
    };
    matched.into_iter().map(to_event).collect()
}

/// Caps Lock reports its lock state, so its release is read from the key;
/// shortcuts that look held too long are released.
fn tick(shared: &mut Shared) -> Vec<HotkeyEvent> {
    let mut matched = Vec::new();
    if shared.matcher.caps_lock_held() && !iokit::caps_lock_key_down() {
        matched.extend(shared.matcher.release_key(Key::CapsLock));
    }
    let protected = shared.protected.clone();
    matched.extend(
        shared
            .matcher
            .release_stuck(Instant::now(), STUCK_AFTER, &protected),
    );
    matched.into_iter().map(to_event).collect()
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
