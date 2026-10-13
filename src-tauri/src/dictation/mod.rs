//! Dictation orchestration: gestures, the controller actor and the
//! record-transcribe-insert pipeline.

mod controller;
pub mod gestures;
mod pipeline;
mod state;

pub use controller::{ControllerHandle, Services, spawn};
pub use pipeline::transcribe;
pub use state::{ControllerInput, Delivered, DictationState, DictationUi, Mode, Timing};
