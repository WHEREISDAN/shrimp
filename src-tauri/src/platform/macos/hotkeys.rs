//! Global shortcuts through handy-keys: an active `CGEventTap` that sees Fn and
//! modifier-only chords and swallows bound input (needs Accessibility).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use handy_keys::{BlockingHotkeys, KeyboardListener};

use super::permissions::accessibility_trusted;
use crate::hotkeys::{Binding, Matcher, ShortcutEvent, blocked_hotkeys};
use crate::platform::{HotkeyBackend, PlatformError};

const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct MacHotkeys {
    blocking: BlockingHotkeys,
    matcher: Arc<Mutex<Matcher>>,
    worker: Option<Worker>,
}

struct Worker {
    running: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl HotkeyBackend for MacHotkeys {
    fn start(
        &mut self,
        on_event: Box<dyn Fn(ShortcutEvent) + Send + Sync>,
    ) -> Result<(), PlatformError> {
        if self.worker.is_some() {
            return Ok(());
        }
        if !accessibility_trusted() {
            return Err(PlatformError::AccessibilityMissing);
        }
        let listener = KeyboardListener::new_with_blocking(Arc::clone(&self.blocking))
            .map_err(|err| PlatformError::Failed(format!("keyboard hook failed: {err}")))?;
        let running = Arc::new(AtomicBool::new(true));
        let matcher = Arc::clone(&self.matcher);
        let thread_running = Arc::clone(&running);
        let handle = std::thread::Builder::new()
            .name("shrimp-hotkeys".into())
            .spawn(move || forward(&listener, &matcher, &thread_running, on_event.as_ref()))
            .map_err(|err| {
                PlatformError::Failed(format!("could not start hotkey thread: {err}"))
            })?;
        self.worker = Some(Worker { running, handle });
        Ok(())
    }

    fn set_bindings(&mut self, bindings: &[Binding]) {
        lock(&self.matcher).set_bindings(bindings);
        *lock(&self.blocking) = blocked_hotkeys(bindings);
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.running.store(false, Ordering::Relaxed);
            let _ = worker.handle.join();
        }
    }
}

impl Drop for MacHotkeys {
    fn drop(&mut self) {
        self.stop();
    }
}

fn forward(
    listener: &KeyboardListener,
    matcher: &Mutex<Matcher>,
    running: &AtomicBool,
    on_event: &(dyn Fn(ShortcutEvent) + Send + Sync),
) {
    while running.load(Ordering::Relaxed) {
        match listener.recv_timeout(POLL_INTERVAL) {
            Ok(event) => {
                let matched = lock(matcher).process(&event);
                matched.into_iter().for_each(on_event);
            }
            Err(handy_keys::Error::Timeout) => {}
            Err(err) => {
                log::error!("keyboard hook stopped: {err}");
                return;
            }
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}
