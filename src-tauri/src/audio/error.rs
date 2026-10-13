use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("could not read WAV file {path}: {source}")]
    Wav {
        path: PathBuf,
        #[source]
        source: hound::Error,
    },
    #[error("unsupported audio format: {0}")]
    UnsupportedFormat(String),
    #[error("audio source is already running")]
    AlreadyStarted,
    #[error("audio source stopped unexpectedly")]
    Disconnected,
    #[error("resampling failed: {0}")]
    Resample(String),
    #[error("invalid audio source spec {spec:?}: {reason}")]
    InvalidSpec { spec: String, reason: &'static str },
    #[error("no microphone found")]
    NoInputDevice,
    #[error("{} does not exist", .0.display())]
    MissingFile(PathBuf),
    #[error("{0}")]
    Device(String),
}
