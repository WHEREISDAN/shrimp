//! Speech-to-text behind one trait, so engines are interchangeable per user
//! setting. Local Whisper (whisper-rs, Metal on Apple Silicon) is the default;
//! OpenAI and Groq are optional cloud engines with the user's own key.

mod engine;
pub mod manager;
pub mod models;
pub mod openai_compat;
mod router;
mod slot;
mod whisper_local;

pub use engine::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};
pub use manager::{ModelManager, ModelState, ModelStatus};
pub use openai_compat::CloudStt;
pub use router::SttRouter;
pub use slot::EngineSlot;
pub use whisper_local::LocalWhisper;
