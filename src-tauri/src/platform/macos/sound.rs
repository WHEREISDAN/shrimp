//! Plays dictation cues with `NSSound`, which needs no open audio stream
//! between cues. Sounds are created and played on the main thread.

use std::cell::RefCell;
use std::collections::HashMap;

use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_app_kit::NSSound;
use objc2_foundation::NSData;

use crate::platform::{CuePlayer, MainThread};
use crate::sounds::{Cue, wav_bytes};

thread_local! {
    static SOUNDS: RefCell<HashMap<Cue, Retained<NSSound>>> = RefCell::new(HashMap::new());
}

pub struct MacCuePlayer {
    main_thread: MainThread,
}

impl MacCuePlayer {
    pub fn new(main_thread: MainThread) -> Self {
        Self { main_thread }
    }
}

impl CuePlayer for MacCuePlayer {
    fn play(&self, cue: Cue) {
        (self.main_thread)(Box::new(move || {
            SOUNDS.with_borrow_mut(|sounds| {
                let Some(sound) = sounds.get(&cue).cloned().or_else(|| {
                    let data = NSData::with_bytes(&wav_bytes(cue));
                    let sound = NSSound::initWithData(NSSound::alloc(), &data)?;
                    sounds.insert(cue, sound.clone());
                    Some(sound)
                }) else {
                    log::warn!("could not load the {cue:?} sound");
                    return;
                };
                if sound.isPlaying() {
                    sound.stop();
                }
                let started = sound.play();
                log::debug!("{cue:?} cue started: {started}");
            });
        }));
    }
}
