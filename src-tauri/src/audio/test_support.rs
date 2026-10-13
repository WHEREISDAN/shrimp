use std::path::Path;

/// Writes 16-bit PCM, the format microphones and `say -o` produce.
pub fn write_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[f32]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &sample in samples {
        writer
            .write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
}
