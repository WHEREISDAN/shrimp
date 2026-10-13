//! Dictation cues (SND-01), synthesised in code so no audio assets ship:
//! short sine tones with soft attack and release.

use std::io::Cursor;

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

    const ALL: [Cue; 5] = [Cue::Start, Cue::Stop, Cue::Lock, Cue::Cancel, Cue::Warning];

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
    fn wav_bytes_decode_back_to_the_cue() {
        let bytes = wav_bytes(Cue::Start);
        let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, SAMPLE_RATE);
        assert_eq!(reader.len() as usize, samples(Cue::Start).len());
    }
}
