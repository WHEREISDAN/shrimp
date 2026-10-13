//! Signal level for the Flow Bar waveform, and silence trimming before STT
//! (ENG-03).

use super::{AudioClip, STT_SAMPLE_RATE};

const FRAME_SAMPLES: usize = STT_SAMPLE_RATE as usize / 50;
/// Frames quieter than this (about -50 dBFS) count as silence.
const SILENCE_RMS: f32 = 0.003;
/// Audio kept around the speech so soft word edges survive the trim.
const SPEECH_PADDING_FRAMES: usize = 15;
const DISPLAY_FLOOR_DB: f32 = -60.0;

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Maps RMS to 0..=1 on a -60..0 dBFS scale, which suits a waveform display.
pub fn display_level(rms: f32) -> f32 {
    if rms <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    ((db - DISPLAY_FLOOR_DB) / -DISPLAY_FLOOR_DB).clamp(0.0, 1.0)
}

/// Cuts leading and trailing silence, keeping a little padding. Returns
/// `None` when the clip has no signal at all, so silence never reaches STT.
pub fn trim_silence(clip: &AudioClip) -> Option<AudioClip> {
    let frames: Vec<&[f32]> = clip.samples().chunks(FRAME_SAMPLES).collect();
    let loud = |frame: &&[f32]| rms(frame) > SILENCE_RMS;
    let first = frames.iter().position(loud)?;
    let last = frames.iter().rposition(loud)?;
    let start = first.saturating_sub(SPEECH_PADDING_FRAMES) * FRAME_SAMPLES;
    let end = ((last + 1 + SPEECH_PADDING_FRAMES) * FRAME_SAMPLES).min(clip.samples().len());
    Some(AudioClip::from_mono_16k(
        clip.samples()[start..end].to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(seconds: f32, amplitude: f32) -> Vec<f32> {
        let len = (seconds * STT_SAMPLE_RATE as f32) as usize;
        (0..len)
            .map(|i| amplitude * (i as f32 * 0.2).sin())
            .collect()
    }

    #[test]
    fn digital_silence_and_noise_floor_have_no_signal() {
        assert!(trim_silence(&AudioClip::from_mono_16k(vec![0.0; 48_000])).is_none());
        assert!(trim_silence(&AudioClip::from_mono_16k(tone(2.0, 0.001))).is_none());
    }

    #[test]
    fn trims_leading_and_trailing_silence_with_padding() {
        let mut samples = vec![0.0; 32_000];
        samples.extend(tone(1.0, 0.3));
        samples.extend(vec![0.0; 48_000]);
        let trimmed = trim_silence(&AudioClip::from_mono_16k(samples)).unwrap();

        let seconds = trimmed.duration().as_secs_f32();
        assert!((1.5..1.7).contains(&seconds), "kept {seconds} s");
    }

    #[test]
    fn display_level_spans_minus_60_to_0_dbfs() {
        assert_eq!(display_level(0.0), 0.0);
        assert!((display_level(1.0) - 1.0).abs() < 1e-6);
        assert!((display_level(0.001) - 0.0).abs() < 1e-6);
        assert!((display_level(0.031_622_8) - 0.5).abs() < 1e-3);
    }
}
