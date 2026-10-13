//! Plays dictation cues with `PlaySound` from memory, which keeps no audio
//! stream open between cues. A new cue stops the one playing.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::core::PCWSTR;

use crate::platform::CuePlayer;
use crate::sounds::{Cue, wav_bytes};

/// Each cue's WAV, kept for the life of the process because playback reads
/// it asynchronously.
fn cue_data(cue: Cue) -> &'static [u8] {
    static CUES: OnceLock<Mutex<HashMap<Cue, &'static [u8]>>> = OnceLock::new();
    let mut cues = CUES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    cues.entry(cue)
        .or_insert_with(|| Box::leak(wav_bytes(cue).into_boxed_slice()))
}

pub struct WinCuePlayer;

impl CuePlayer for WinCuePlayer {
    fn play(&self, cue: Cue) {
        let data = cue_data(cue);
        let started = unsafe {
            PlaySoundW(
                PCWSTR(data.as_ptr().cast()),
                None,
                SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
            )
        };
        log::debug!("{cue:?} cue started: {}", started.as_bool());
    }
}
