//! Signal level for the Flow Bar waveform, silence trimming before STT
//! (ENG-03), and gain for quiet, whispered speech (MIC-09).

use super::{AudioClip, STT_SAMPLE_RATE};

const FRAME_SAMPLES: usize = STT_SAMPLE_RATE as usize / 50;
/// Frames quieter than this (about -50 dBFS) are always silence...
const SILENCE_RMS: f32 = 0.003;
/// ...frames louder than this (about -74 dBFS) can be whispered speech when
/// they stand out from the recording's noise floor.
const WHISPER_RMS: f32 = 0.0002;
/// Speech is this many times louder than the noise floor.
const ABOVE_FLOOR: f32 = 3.0;
/// The noise floor is the level this share of frames stays under.
const FLOOR_PERCENTILE: f32 = 0.1;
/// Speech quieter than this (about -26 dBFS) is raised to it for STT.
const TARGET_RMS: f32 = 0.05;
/// At most about +36 dB, so noise is not raised into words.
const MAX_GAIN: f32 = 64.0;
const PEAK_CEILING: f32 = 0.9;
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

/// The level above which a frame counts as sound in this recording: the
/// fixed -50 dBFS floor for normal recordings, lower for a quiet microphone
/// or a whisper over a quiet room.
fn speech_threshold(levels: &[f32]) -> f32 {
    let mut sorted = levels.to_vec();
    sorted.sort_by(f32::total_cmp);
    let floor = sorted
        .get((sorted.len() as f32 * FLOOR_PERCENTILE) as usize)
        .copied()
        .unwrap_or(0.0);
    (floor * ABOVE_FLOOR).clamp(WHISPER_RMS, SILENCE_RMS)
}

/// Cuts leading and trailing silence, keeping a little padding. Returns
/// `None` when the clip has no signal at all, so silence never reaches STT.
pub fn trim_silence(clip: &AudioClip) -> Option<AudioClip> {
    let frames: Vec<&[f32]> = clip.samples().chunks(FRAME_SAMPLES).collect();
    let levels: Vec<f32> = frames.iter().map(|frame| rms(frame)).collect();
    let threshold = speech_threshold(&levels);
    let loud = |level: &f32| *level > threshold;
    let first = levels.iter().position(loud)?;
    let last = levels.iter().rposition(loud)?;
    let start = first.saturating_sub(SPEECH_PADDING_FRAMES) * FRAME_SAMPLES;
    let end = ((last + 1 + SPEECH_PADDING_FRAMES) * FRAME_SAMPLES).min(clip.samples().len());
    Some(AudioClip::from_mono_16k(
        clip.samples()[start..end].to_vec(),
    ))
}

/// Raises quiet speech to a normal level, so whispers transcribe like
/// speech (MIC-09). Loud enough speech is left as it is.
pub fn normalize(clip: &AudioClip) -> AudioClip {
    let frames: Vec<f32> = clip.samples().chunks(FRAME_SAMPLES).map(rms).collect();
    let threshold = speech_threshold(&frames);
    let speech: Vec<f32> = frames
        .into_iter()
        .filter(|level| *level > threshold)
        .collect();
    if speech.is_empty() {
        return clip.clone();
    }
    let speech_rms =
        (speech.iter().map(|level| level * level).sum::<f32>() / speech.len() as f32).sqrt();
    let peak = clip
        .samples()
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    let gain = (TARGET_RMS / speech_rms)
        .min(MAX_GAIN)
        .min(PEAK_CEILING / peak.max(f32::EPSILON));
    if gain <= 1.0 {
        return clip.clone();
    }
    log::debug!("raising quiet speech by {:.1} dB", 20.0 * gain.log10());
    AudioClip::from_mono_16k(clip.samples().iter().map(|sample| sample * gain).collect())
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
    fn a_whisper_over_a_quiet_room_is_kept_and_raised() {
        let mut samples = tone(1.0, 0.000_05);
        samples.extend(tone(1.0, 0.0012));
        samples.extend(tone(1.0, 0.000_05));
        let clip = AudioClip::from_mono_16k(samples);
        let trimmed = trim_silence(&clip).expect("the whisper is sound");
        let seconds = trimmed.duration().as_secs_f32();
        assert!((1.5..1.7).contains(&seconds), "kept {seconds} s");
        let raised = normalize(&trimmed);
        let level = rms(raised.samples());
        assert!(level > 0.02, "raised to {level}");
        let loud = AudioClip::from_mono_16k(tone(1.0, 0.3));
        assert_eq!(
            normalize(&loud).samples(),
            loud.samples(),
            "normal speech untouched"
        );
    }

    #[test]
    fn display_level_spans_minus_60_to_0_dbfs() {
        assert_eq!(display_level(0.0), 0.0);
        assert!((display_level(1.0) - 1.0).abs() < 1e-6);
        assert!((display_level(0.001) - 0.0).abs() < 1e-6);
        assert!((display_level(0.031_622_8) - 0.5).abs() < 1e-3);
    }
}
