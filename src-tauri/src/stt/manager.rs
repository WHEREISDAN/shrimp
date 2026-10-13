//! Gets the default local model ready: downloads it (and the VAD model) on
//! first use with progress, then loads it into the engine slot.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::models::{
    DEFAULT_WHISPER_MODEL, ModelFile, ModelStore, VAD_MODEL, WHISPER_MODELS, find_whisper_model,
};
use super::{EngineSlot, LocalWhisper};

/// Mirrors `ModelState` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ModelState {
    Missing,
    Downloading {
        received: u64,
        total: u64,
    },
    Downloaded,
    /// Downloaded and loaded into the engine.
    Active,
    Failed {
        message: String,
    },
}

/// Mirrors `ModelStatus` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub id: &'static str,
    pub label: &'static str,
    pub size_bytes: u64,
    pub is_default: bool,
    #[serde(flatten)]
    pub state: ModelState,
}

type OnChange = Box<dyn Fn(Vec<ModelStatus>) + Send + Sync>;

pub struct ModelManager {
    store: ModelStore,
    client: reqwest::Client,
    slot: Arc<EngineSlot>,
    states: Mutex<HashMap<&'static str, ModelState>>,
    preparing: AtomicBool,
    on_change: OnChange,
}

impl ModelManager {
    pub fn new(dir: PathBuf, slot: Arc<EngineSlot>, on_change: OnChange) -> Arc<Self> {
        let store = ModelStore::new(dir);
        let states = WHISPER_MODELS
            .iter()
            .map(|model| {
                let state = if store.is_downloaded(model) {
                    ModelState::Downloaded
                } else {
                    ModelState::Missing
                };
                (model.id, state)
            })
            .collect();
        Arc::new(Self {
            store,
            client: reqwest::Client::new(),
            slot,
            states: Mutex::new(states),
            preparing: AtomicBool::new(false),
            on_change,
        })
    }

    pub fn statuses(&self) -> Vec<ModelStatus> {
        let states = self.lock_states();
        WHISPER_MODELS
            .iter()
            .map(|model| ModelStatus {
                id: model.id,
                label: model.label,
                size_bytes: model.size,
                is_default: model.id == DEFAULT_WHISPER_MODEL,
                state: states.get(model.id).cloned().unwrap_or(ModelState::Missing),
            })
            .collect()
    }

    /// Downloads whatever is missing, then loads the default model. Safe to
    /// call again (retry); a run already in progress is not duplicated.
    pub async fn prepare_default(self: Arc<Self>) {
        if self.preparing.swap(true, Ordering::SeqCst) {
            return;
        }
        let model =
            find_whisper_model(DEFAULT_WHISPER_MODEL).expect("default model is in the catalog");
        match self.ensure_files(model).await {
            Ok(()) => self.load(model).await,
            Err(message) => {
                self.slot.set_status(format!(
                    "The speech model could not be downloaded: {message}"
                ));
                self.set_state(model.id, ModelState::Failed { message });
            }
        }
        self.preparing.store(false, Ordering::SeqCst);
    }

    async fn ensure_files(&self, model: &'static ModelFile) -> Result<(), String> {
        if !self.store.is_downloaded(&VAD_MODEL) {
            self.slot.set_status("Downloading speech detection…");
            self.store
                .download(&self.client, &VAD_MODEL, |_, _| {})
                .await
                .map_err(|err| err.to_string())?;
        }
        if self.store.is_downloaded(model) {
            return Ok(());
        }
        log::info!("downloading {} ({} bytes)", model.file_name, model.size);
        let mut last_percent = None;
        self.store
            .download(&self.client, model, |received, total| {
                let percent = received * 100 / total.max(1);
                if last_percent != Some(percent) {
                    last_percent = Some(percent);
                    self.slot.set_status(format!(
                        "The speech model is downloading ({percent}%). Try again when it's ready."
                    ));
                    self.set_state(model.id, ModelState::Downloading { received, total });
                }
            })
            .await
            .map_err(|err| err.to_string())?;
        self.set_state(model.id, ModelState::Downloaded);
        Ok(())
    }

    async fn load(&self, model: &'static ModelFile) {
        self.slot
            .set_status("The speech model is loading. Try again in a moment.");
        let path = self.store.path(model);
        let vad = self.store.path(&VAD_MODEL);
        let loaded =
            tokio::task::spawn_blocking(move || LocalWhisper::load(&path, Some(&vad))).await;
        match loaded {
            Ok(Ok(engine)) => {
                self.slot.set_engine(Arc::new(engine));
                self.set_state(model.id, ModelState::Active);
                log::info!("{} is loaded", model.file_name);
            }
            Ok(Err(error)) => self.fail_load(model, error.to_string()),
            Err(error) => self.fail_load(model, error.to_string()),
        }
    }

    fn fail_load(&self, model: &'static ModelFile, message: String) {
        log::error!("could not load {}: {message}", model.file_name);
        self.slot
            .set_status(format!("The speech model could not be loaded: {message}"));
        self.set_state(model.id, ModelState::Failed { message });
    }

    fn set_state(&self, id: &'static str, state: ModelState) {
        self.lock_states().insert(id, state);
        (self.on_change)(self.statuses());
    }

    fn lock_states(&self) -> std::sync::MutexGuard<'_, HashMap<&'static str, ModelState>> {
        self.states.lock().unwrap_or_else(|err| err.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serializes_flat_with_a_state_tag() {
        let status = ModelStatus {
            id: "tiny-q5_1",
            label: "Whisper Tiny",
            size_bytes: 10,
            is_default: false,
            state: ModelState::Downloading {
                received: 5,
                total: 10,
            },
        };
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({
                "id": "tiny-q5_1",
                "label": "Whisper Tiny",
                "sizeBytes": 10,
                "isDefault": false,
                "state": "downloading",
                "received": 5,
                "total": 10
            })
        );
    }

    #[test]
    fn lists_the_catalog_with_the_default_marked() {
        let dir = tempfile::tempdir().unwrap();
        let slot = Arc::new(EngineSlot::new("starting"));
        let manager = ModelManager::new(dir.path().into(), slot, Box::new(|_| {}));
        let statuses = manager.statuses();
        assert_eq!(statuses.len(), WHISPER_MODELS.len());
        let default: Vec<_> = statuses.iter().filter(|status| status.is_default).collect();
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].id, DEFAULT_WHISPER_MODEL);
        assert!(
            statuses
                .iter()
                .all(|status| status.state == ModelState::Missing)
        );
    }
}
