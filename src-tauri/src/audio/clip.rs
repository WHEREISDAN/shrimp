use std::time::Duration;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use super::{AudioError, AudioFormat};

/// Sample rate every STT engine receives.
pub const STT_SAMPLE_RATE: u32 = 16_000;

const RESAMPLE_CHUNK_FRAMES: usize = 1024;

/// Audio ready for speech-to-text: mono f32 samples at [`STT_SAMPLE_RATE`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioClip {
    samples: Vec<f32>,
}

impl AudioClip {
    pub fn from_mono_16k(samples: Vec<f32>) -> Self {
        Self { samples }
    }

    /// Downmixes interleaved audio to mono and resamples it to 16 kHz.
    pub fn from_interleaved(format: AudioFormat, samples: &[f32]) -> Result<Self, AudioError> {
        if format.channels == 0 || format.sample_rate == 0 {
            return Err(AudioError::UnsupportedFormat(format!(
                "{} channels at {} Hz",
                format.channels, format.sample_rate
            )));
        }
        let mono = downmix(samples, format.channels);
        let samples = if format.sample_rate == STT_SAMPLE_RATE {
            mono
        } else {
            resample_to_stt_rate(&mono, format.sample_rate)?
        };
        Ok(Self { samples })
    }

    /// The clips one after another.
    pub fn concat(clips: &[AudioClip]) -> Self {
        Self {
            samples: clips
                .iter()
                .flat_map(|clip| clip.samples.iter().copied())
                .collect(),
        }
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.samples.len() as f64 / f64::from(STT_SAMPLE_RATE))
    }
}

fn downmix(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    let channels = usize::from(channels);
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn resample_to_stt_rate(mono: &[f32], from_rate: u32) -> Result<Vec<f32>, AudioError> {
    if mono.is_empty() {
        return Ok(Vec::new());
    }
    let resample_error = |err: &dyn std::fmt::Display| AudioError::Resample(err.to_string());
    let mut resampler = Fft::<f32>::new(
        from_rate as usize,
        STT_SAMPLE_RATE as usize,
        RESAMPLE_CHUNK_FRAMES,
        1,
        FixedSync::Input,
    )
    .map_err(|err| resample_error(&err))?;
    let input = InterleavedSlice::new(mono, 1, mono.len()).map_err(|err| resample_error(&err))?;
    let output = resampler
        .process_all(&input, mono.len(), None)
        .map_err(|err| resample_error(&err))?;
    Ok(output.take_data())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, frequency: f32, seconds: f32, amplitude: f32) -> Vec<f32> {
        let len = (rate as f32 * seconds) as usize;
        (0..len)
            .map(|i| {
                amplitude * (2.0 * std::f32::consts::PI * frequency * i as f32 / rate as f32).sin()
            })
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn keeps_16k_mono_untouched() {
        let samples = vec![0.1, -0.2, 0.3];
        let format = AudioFormat {
            sample_rate: 16_000,
            channels: 1,
        };
        let clip = AudioClip::from_interleaved(format, &samples).unwrap();
        assert_eq!(clip.samples(), samples.as_slice());
    }

    #[test]
    fn downmixes_stereo_by_averaging_channels() {
        let format = AudioFormat {
            sample_rate: 16_000,
            channels: 2,
        };
        let clip = AudioClip::from_interleaved(format, &[1.0, 0.0, 0.5, 0.5, -1.0, 1.0]).unwrap();
        assert_eq!(clip.samples(), &[0.5, 0.5, 0.0]);
    }

    #[test]
    fn resamples_common_mic_rates_to_16k_preserving_level() {
        for rate in [44_100, 48_000] {
            let input = sine(rate, 440.0, 1.0, 0.5);
            let format = AudioFormat {
                sample_rate: rate,
                channels: 1,
            };
            let clip = AudioClip::from_interleaved(format, &input).unwrap();

            let expected_len = 16_000;
            assert!(
                clip.samples().len().abs_diff(expected_len) <= 2,
                "{rate} Hz gave {} samples",
                clip.samples().len()
            );
            // Ignore the edges, where the FFT window ramps.
            let middle = &clip.samples()[2_000..14_000];
            assert!(
                (rms(middle) - 0.5 / 2f32.sqrt()).abs() < 0.01,
                "{rate} Hz rms {}",
                rms(middle)
            );
        }
    }

    #[test]
    fn reports_duration_at_16k() {
        let clip = AudioClip::from_mono_16k(vec![0.0; 24_000]);
        assert_eq!(clip.duration(), Duration::from_millis(1_500));
    }

    #[test]
    fn rejects_zero_channel_audio() {
        let format = AudioFormat {
            sample_rate: 16_000,
            channels: 0,
        };
        assert!(AudioClip::from_interleaved(format, &[]).is_err());
    }
}
