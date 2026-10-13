use std::fmt;
use std::path::PathBuf;

use crate::t;

/// WAV sources and specs are developer tools, so their errors stay in English.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    Wav {
        path: PathBuf,
        #[source]
        source: hound::Error,
    },
    UnsupportedFormat(String),
    AlreadyStarted,
    Disconnected,
    Resample(String),
    InvalidSpec {
        spec: String,
        reason: &'static str,
    },
    NoInputDevice,
    MissingFile(PathBuf),
    Device(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wav { path, source } => {
                write!(f, "could not read WAV file {}: {source}", path.display())
            }
            Self::UnsupportedFormat(format) => {
                f.write_str(&t!("notice.audio.unsupportedFormat", format = format))
            }
            Self::AlreadyStarted => f.write_str(&t!("notice.audio.alreadyStarted")),
            Self::Disconnected => f.write_str(&t!("notice.audio.disconnected")),
            Self::Resample(error) => f.write_str(&t!("notice.audio.resampleFailed", error = error)),
            Self::InvalidSpec { spec, reason } => {
                write!(f, "invalid audio source spec {spec:?}: {reason}")
            }
            Self::NoInputDevice => f.write_str(&t!("notice.audio.noMicrophone")),
            Self::MissingFile(path) => write!(f, "{} does not exist", path.display()),
            Self::Device(message) => f.write_str(message),
        }
    }
}
