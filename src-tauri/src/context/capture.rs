//! Reads the focused field on the blocking pool while recording runs, and
//! hands the result over only if it is ready, so context never delays a
//! dictation (CTX-01).

use std::sync::Arc;

use tokio::sync::oneshot::{self, error::TryRecvError};

use super::{AppKind, DictationContext, apps};
use crate::platform::{AppIdentity, ContextRequest, FieldContext, FocusInspector};

/// Characters read on each side of the cursor.
const AROUND: usize = 1_000;
/// The whole field is read up to this size (code for identifiers).
const TEXT_LIMIT: usize = 20_000;

enum State {
    Waiting(oneshot::Receiver<Option<FieldContext>>),
    Done(Option<Box<FieldContext>>),
}

/// A context read in progress.
pub struct PendingContext {
    app: Option<AppIdentity>,
    state: State,
}

impl std::fmt::Debug for PendingContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingContext")
            .field("app", &self.app)
            .finish_non_exhaustive()
    }
}

/// Starts reading the focused field of `app`.
pub fn capture(focus: Arc<dyn FocusInspector>, app: Option<AppIdentity>) -> PendingContext {
    let request = ContextRequest {
        around: AROUND,
        text_limit: TEXT_LIMIT,
        scan_window: true,
        find_url: apps::kind(app.as_ref()) == AppKind::Browser,
    };
    let (tx, rx) = oneshot::channel();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = tx.send(focus.read_field(&request));
    });
    PendingContext {
        app,
        state: State::Waiting(rx),
    }
}

impl PendingContext {
    /// No field to read (paste last transcript, tests).
    pub fn without_field(app: Option<AppIdentity>) -> Self {
        Self {
            app,
            state: State::Done(None),
        }
    }

    pub fn app(&self) -> Option<&AppIdentity> {
        self.app.as_ref()
    }

    /// What has been read so far; never waits.
    pub fn ready(&mut self) -> DictationContext {
        if let State::Waiting(rx) = &mut self.state {
            match rx.try_recv() {
                Ok(field) => self.state = State::Done(field.map(Box::new)),
                Err(TryRecvError::Closed) => self.state = State::Done(None),
                Err(TryRecvError::Empty) => log::debug!("context not read yet"),
            }
        }
        let field = match &self.state {
            State::Done(field) => field.as_deref().cloned(),
            State::Waiting(_) => None,
        };
        DictationContext::new(self.app.clone(), field)
    }
}
