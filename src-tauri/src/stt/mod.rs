//! Speech-to-text behind one trait, so engines are interchangeable per user
//! setting. Planned engines: local Whisper (whisper-rs, Metal on Apple
//! Silicon) and the OpenAI-compatible transcription APIs of OpenAI and Groq.

mod engine;

pub use engine::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};
