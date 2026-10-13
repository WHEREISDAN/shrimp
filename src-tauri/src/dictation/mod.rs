//! Dictation orchestration: gestures, the controller actor and the
//! record-transcribe-insert pipeline.

mod bindings;
mod command;
mod controller;
pub mod gestures;
pub mod learn;
mod outcomes;
mod pipeline;
mod state;
mod transform;

pub use controller::{ControllerHandle, Services, spawn};
pub use pipeline::transcribe;
pub use state::{
    Answer, ControllerInput, Delivered, DictationState, DictationUi, Mode, Timing, TransformAction,
};
pub use transform::TransformView;
