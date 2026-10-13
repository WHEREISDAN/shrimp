//! Mute or pause while dictating (SND-03, SND-04): shortly after recording
//! starts (so the start cue is heard), and undone when it ends, before the
//! stop cue.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::platform::{MediaControl, Restore};
use crate::settings::MediaSettings;

/// After the start cue, and within about 300 ms of the start.
pub const QUIET_AFTER: Duration = Duration::from_millis(200);

enum State {
    Waiting,
    Quieted(Restore),
    Ended,
}

pub struct MediaSession {
    state: Arc<Mutex<State>>,
}

impl MediaSession {
    /// Mutes or pauses after `delay` per the settings; None when both are off.
    pub fn begin(
        media: &Arc<dyn MediaControl>,
        settings: MediaSettings,
        delay: Duration,
    ) -> Option<Self> {
        if !settings.mute && !settings.pause {
            return None;
        }
        let state = Arc::new(Mutex::new(State::Waiting));
        let shared = Arc::clone(&state);
        let media = Arc::clone(media);
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            if !matches!(*lock(&shared), State::Waiting) {
                return;
            }
            let restore = if settings.mute {
                media.mute()
            } else {
                media.pause()
            };
            let mut state = lock(&shared);
            match (&*state, restore) {
                (State::Waiting, Some(restore)) => *state = State::Quieted(restore),
                // The recording ended meanwhile.
                (_, Some(restore)) => restore(),
                (_, None) => {}
            }
        });
        Some(Self { state })
    }

    /// Unmutes or resumes what this session quieted.
    pub fn end(self) {
        let previous = std::mem::replace(&mut *lock(&self.state), State::Ended);
        if let State::Quieted(restore) = previous {
            restore();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[derive(Default)]
    struct Counting {
        muted: AtomicUsize,
        restored: Arc<AtomicUsize>,
    }

    impl MediaControl for Counting {
        fn mute(&self) -> Option<Restore> {
            self.muted.fetch_add(1, Ordering::SeqCst);
            let restored = Arc::clone(&self.restored);
            Some(Box::new(move || {
                restored.fetch_add(1, Ordering::SeqCst);
            }))
        }

        fn pause(&self) -> Option<Restore> {
            None
        }
    }

    fn mute_on() -> MediaSettings {
        MediaSettings {
            mute: true,
            pause: false,
        }
    }

    #[test]
    fn mutes_after_the_delay_and_restores_at_the_end() {
        let counting = Arc::new(Counting::default());
        let media: Arc<dyn MediaControl> = Arc::clone(&counting) as Arc<dyn MediaControl>;
        let session = MediaSession::begin(&media, mute_on(), Duration::from_millis(10)).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(counting.muted.load(Ordering::SeqCst), 1);
        session.end();
        assert_eq!(counting.restored.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_recording_that_ends_first_never_mutes() {
        let counting = Arc::new(Counting::default());
        let media: Arc<dyn MediaControl> = Arc::clone(&counting) as Arc<dyn MediaControl>;
        let session = MediaSession::begin(&media, mute_on(), Duration::from_millis(100)).unwrap();
        session.end();
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(counting.muted.load(Ordering::SeqCst), 0);
        let off = MediaSettings {
            mute: false,
            pause: false,
        };
        assert!(MediaSession::begin(&media, off, Duration::ZERO).is_none());
    }
}
