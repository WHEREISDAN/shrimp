//! AI edits between speech-to-text and insertion: spoken commands, snippets,
//! dictionary corrections, the LLM cleanup with app-aware styles, and the
//! rendering of lists and rich snippets for pasting. Command Mode runs a
//! spoken instruction on selected text.

pub mod command;
mod context_format;
pub mod dictionary;
pub mod guard;
pub mod library;
pub mod markdown;
mod names;
mod phrase;
mod pipeline;
pub mod prompts;
pub mod snippets;
pub mod spoken;
pub mod style;

use serde::{Deserialize, Serialize};

pub use library::{FixedLibrary, Library, Snapshot};
pub use markdown::Rendered;
pub use pipeline::{EditTimeout, Edited, Editor, RAW_INSERTED};
pub use style::{AppCategory, Style, Target};

/// How much the AI edits (FMT-01). Mirrors `CleanupLevel` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupLevel {
    /// The raw transcript; only spoken commands, snippets and corrections.
    None,
    /// Fillers, self-corrections, punctuation and grammar.
    #[default]
    Light,
    /// Also rewords for clarity and concision.
    Medium,
}
