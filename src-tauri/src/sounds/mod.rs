//! Dictation and notification cues (SND-01), synthesised in code so no
//! audio assets ship: short sine tones with soft attack and release.

use std::io::Cursor;
use std::sync::Arc;

use crate::dictation::{Answer, DictationState, DictationUi, RecordingStatus, TransformView};
use crate::notify::{Notice, NoticeCategory};
use crate::platform::CuePlayer;
use crate::settings::SettingsStore;

const SAMPLE_RATE: u32 = 44_100;
const VOLUME: f32 = 0.22;
const ATTACK_SECONDS: f32 = 0.006;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cue {
    /// Recording started.
    Start,
    /// Recording stopped; transcribing.
    Stop,
    /// Hands-free lock engaged.
    Lock,
    /// Dictation cancelled.
    Cancel,
    /// The session is about to hit its time limit (CORE-08).
    Warning,
    /// The dictation was pasted.
    Paste,
    /// A warning or error notice appeared.
    Notice,
    /// A word-count or voice-profile milestone (STA-04).
    Achievement,
}

impl Cue {
    /// Dictation cues follow Settings › Sound › Dictation sounds; the notice
    /// and achievement sounds follow Notification sounds (SND-02).
    pub fn is_dictation(self) -> bool {
        !matches!(self, Self::Notice | Self::Achievement)
    }
}

/// One tone: frequency in Hz and duration in seconds, then a pause.
struct Note(f32, f32, f32);

fn notes(cue: Cue) -> &'static [Note] {
    match cue {
        Cue::Start => &[Note(659.25, 0.07, 0.0), Note(987.77, 0.11, 0.0)],
        Cue::Stop => &[Note(987.77, 0.07, 0.0), Note(659.25, 0.11, 0.0)],
        Cue::Lock => &[
            Note(783.99, 0.06, 0.02),
            Note(987.77, 0.06, 0.02),
            Note(1318.51, 0.1, 0.0),
        ],
        Cue::Cancel => &[Note(392.0, 0.08, 0.0), Note(293.66, 0.14, 0.0)],
        Cue::Warning => &[Note(880.0, 0.12, 0.08), Note(880.0, 0.12, 0.0)],
        Cue::Paste => &[Note(1174.66, 0.04, 0.0), Note(1567.98, 0.07, 0.0)],
        Cue::Notice => &[Note(1046.5, 0.05, 0.03), Note(784.0, 0.09, 0.0)],
        Cue::Achievement => &[
            Note(523.25, 0.05, 0.0),
            Note(659.25, 0.05, 0.0),
            Note(783.99, 0.05, 0.0),
            Note(1046.5, 0.14, 0.0),
        ],
    }
}

/// Mono f32 samples at 44.1 kHz.
pub fn samples(cue: Cue) -> Vec<f32> {
    notes(cue)
        .iter()
        .flat_map(|Note(frequency, seconds, pause)| {
            tone(*frequency, *seconds).chain(silence(*pause))
        })
        .collect()
}

fn tone(frequency: f32, seconds: f32) -> impl Iterator<Item = f32> {
    let len = (seconds * SAMPLE_RATE as f32) as usize;
    let attack = (ATTACK_SECONDS * SAMPLE_RATE as f32) as usize;
    (0..len).map(move |i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        let envelope = if i < attack {
            i as f32 / attack as f32
        } else {
            // Exponential release to the end of the note.
            (-4.0 * (i - attack) as f32 / (len - attack) as f32).exp()
        };
        VOLUME * envelope * (std::f32::consts::TAU * frequency * t).sin()
    })
}

fn silence(seconds: f32) -> impl Iterator<Item = f32> {
    std::iter::repeat_n(0.0, (seconds * SAMPLE_RATE as f32) as usize)
}

/// Plays cues only when the sound settings allow them (SND-02).
pub struct SettingsCues {
    inner: Arc<dyn CuePlayer>,
    settings: Arc<SettingsStore>,
}

impl SettingsCues {
    pub fn new(inner: Arc<dyn CuePlayer>, settings: Arc<SettingsStore>) -> Self {
        Self { inner, settings }
    }
}

impl CuePlayer for SettingsCues {
    fn play(&self, cue: Cue) {
        let sounds = self.settings.get().sounds;
        let on = if cue.is_dictation() {
            sounds.dictation
        } else {
            sounds.notifications
        };
        if on {
            self.inner.play(cue);
        }
    }
}

/// Plays the notice sound with every warning and error notice, and the
/// achievement sound with milestones (SND-01, SND-02).
pub struct SoundingNotices {
    inner: Arc<dyn DictationUi>,
    cues: Arc<dyn CuePlayer>,
}

impl SoundingNotices {
    pub fn new(inner: Arc<dyn DictationUi>, cues: Arc<dyn CuePlayer>) -> Self {
        Self { inner, cues }
    }
}

impl DictationUi for SoundingNotices {
    fn state(&self, state: DictationState) {
        self.inner.state(state);
    }

    fn level(&self, level: f32) {
        self.inner.level(level);
    }

    fn notice(&self, notice: Notice) {
        if notice.category == NoticeCategory::Milestone {
            self.cues.play(Cue::Achievement);
        } else if notice.sound {
            self.cues.play(Cue::Notice);
        }
        self.inner.notice(notice);
    }

    fn dismiss(&self, id: u64) {
        self.inner.dismiss(id);
    }

    fn answer(&self, answer: Answer) {
        self.inner.answer(answer);
    }

    fn changes(&self, view: TransformView) {
        self.inner.changes(view);
    }

    fn shortcut_recording(&self, status: RecordingStatus) {
        self.inner.shortcut_recording(status);
    }

    fn last_transcript(&self, available: bool) {
        self.inner.last_transcript(available);
    }
}

/// The cue as a 16-bit PCM WAV file in memory.
pub fn wav_bytes(cue: Cue) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).expect("in-memory WAV writer cannot fail");
        for sample in samples(cue) {
            let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            writer
                .write_sample(value)
                .expect("in-memory WAV write cannot fail");
        }
        writer
            .finalize()
            .expect("in-memory WAV finalize cannot fail");
    }
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Cue; 8] = [
        Cue::Start,
        Cue::Stop,
        Cue::Lock,
        Cue::Cancel,
        Cue::Warning,
        Cue::Paste,
        Cue::Notice,
        Cue::Achievement,
    ];

    #[test]
    fn cues_are_short_audible_and_distinct() {
        let rendered: Vec<Vec<f32>> = ALL.iter().map(|&cue| samples(cue)).collect();
        for (cue, samples) in ALL.iter().zip(&rendered) {
            let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
            assert!((0.1..0.4).contains(&seconds), "{cue:?} lasts {seconds} s");
            let peak = samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
            assert!(peak > 0.1 && peak <= VOLUME, "{cue:?} peak {peak}");
        }
        for (i, a) in rendered.iter().enumerate() {
            for b in &rendered[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn paste_is_a_dictation_sound_and_achievement_a_notification_sound() {
        assert!(Cue::Paste.is_dictation());
        assert!(!Cue::Achievement.is_dictation());
        assert!(!Cue::Notice.is_dictation());
    }

    #[test]
    fn wav_bytes_decode_back_to_the_cue() {
        let bytes = wav_bytes(Cue::Start);
        let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, SAMPLE_RATE);
        assert_eq!(reader.len() as usize, samples(Cue::Start).len());
    }
}
