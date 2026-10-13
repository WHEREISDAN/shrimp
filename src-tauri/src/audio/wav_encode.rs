//! 16-bit WAV encoding in memory, for cloud speech-to-text uploads (about
//! 32 KB per second at 16 kHz mono).

use std::io::Cursor;

use super::{AudioClip, STT_SAMPLE_RATE};

pub fn wav_bytes(clip: &AudioClip) -> Result<Vec<u8>, hound::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: STT_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut bytes = Cursor::new(Vec::with_capacity(44 + clip.samples().len() * 2));
    let mut writer = hound::WavWriter::new(&mut bytes, spec)?;
    for sample in clip.samples() {
        writer.write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
    }
    writer.finalize()?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_readable_wav() {
        let clip = AudioClip::from_mono_16k(vec![0.0, 0.5, -0.5, 1.5]);
        let bytes = wav_bytes(&clip).unwrap();
        assert_eq!(bytes.len(), 44 + 8);
        let mut reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, 16_000);
        let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        assert_eq!(samples, [0, 16383, -16383, i16::MAX]);
    }
}
