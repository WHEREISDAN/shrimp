//! Speech-to-text behind one trait, so engines are interchangeable per user
//! setting. Local Whisper (whisper-rs, Metal on Apple Silicon) is the default;
//! the OpenAI-compatible cloud engines arrive in phase 2.

mod engine;
pub mod manager;
pub mod models;
mod slot;
mod whisper_local;

pub use engine::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};
pub use manager::{ModelManager, ModelState, ModelStatus};
pub use slot::EngineSlot;
pub use whisper_local::LocalWhisper;
