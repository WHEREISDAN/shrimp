use std::path::PathBuf;
use std::str::FromStr;

use super::AudioError;

/// Debug builds read this variable to replace the microphone, e.g.
/// `SHRIMP_AUDIO_SOURCE=wav:/path/to/speech.wav`. See docs/DEVELOPMENT.md.
pub const AUDIO_SOURCE_ENV: &str = "SHRIMP_AUDIO_SOURCE";

/// Which input a dictation records from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSourceSpec {
    /// A microphone by device name; `None` is the system default input.
    Microphone { device: Option<String> },
    /// A WAV file played at real-time pace in place of the microphone.
    WavFile(PathBuf),
}

impl AudioSourceSpec {
    /// The override from [`AUDIO_SOURCE_ENV`]. Release builds ignore it.
    pub fn from_env() -> Result<Option<Self>, AudioError> {
        if !cfg!(debug_assertions) {
            return Ok(None);
        }
        std::env::var(AUDIO_SOURCE_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.parse())
            .transpose()
    }
}

impl FromStr for AudioSourceSpec {
    type Err = AudioError;

    /// Accepts `mic`, `mic:<device name>` and `wav:<path>`.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = |reason| AudioError::InvalidSpec {
            spec: value.to_string(),
            reason,
        };
        let (kind, argument) = match value.split_once(':') {
            Some((kind, argument)) => (kind, Some(argument.trim())),
            None => (value.trim(), None),
        };
        match (kind, argument) {
            ("mic", None) => Ok(Self::Microphone { device: None }),
            ("mic", Some("")) => Err(invalid("device name is empty")),
            ("mic", Some(device)) => Ok(Self::Microphone {
                device: Some(device.to_string()),
            }),
            ("wav", Some(path)) if !path.is_empty() => Ok(Self::WavFile(PathBuf::from(path))),
            ("wav", _) => Err(invalid("expected wav:<path>")),
            _ => Err(invalid("expected mic, mic:<device> or wav:<path>")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_microphone_specs() {
        assert_eq!(
            "mic".parse::<AudioSourceSpec>().unwrap(),
            AudioSourceSpec::Microphone { device: None }
        );
        assert_eq!(
            "mic:MacBook Pro Microphone"
                .parse::<AudioSourceSpec>()
                .unwrap(),
            AudioSourceSpec::Microphone {
                device: Some("MacBook Pro Microphone".into())
            }
        );
    }

    #[test]
    fn parses_wav_paths_including_colons() {
        assert_eq!(
            "wav:/tmp/a:b.wav".parse::<AudioSourceSpec>().unwrap(),
            AudioSourceSpec::WavFile(PathBuf::from("/tmp/a:b.wav"))
        );
    }

    #[test]
    fn rejects_malformed_specs() {
        for spec in ["", "wav", "wav:", "mic:", "speaker", "file:/tmp/a.wav"] {
            assert!(
                spec.parse::<AudioSourceSpec>().is_err(),
                "{spec:?} should be rejected"
            );
        }
    }
}
