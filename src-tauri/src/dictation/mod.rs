//! Dictation orchestration: gestures, the controller actor and the
//! record-transcribe-insert pipeline.

mod bindings;
mod command;
mod controller;
pub mod gestures;
mod outcomes;
mod pipeline;
mod state;

pub use controller::{ControllerHandle, Services, spawn};
pub use pipeline::transcribe;
pub use state::{Answer, ControllerInput, Delivered, DictationState, DictationUi, Mode, Timing};
