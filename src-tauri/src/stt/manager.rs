//! Gets the chosen local model ready: downloads it (and the VAD model) with
//! progress, then loads it into the engine slot. Models can be switched,
//! downloaded ahead and deleted (ENG-08).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::models::{
    DEFAULT_WHISPER_MODEL, ModelFile, ModelStore, VAD_MODEL, WHISPER_MODELS, find_whisper_model,
};
use super::{EngineSlot, LocalWhisper};
use crate::t;

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
    /// Shrimp's recommendation.
    pub is_default: bool,
    /// Loaded, so dictations use it.
    pub in_use: bool,
    #[serde(flatten)]
    pub state: ModelState,
}

type OnChange = Box<dyn Fn(Vec<ModelStatus>) + Send + Sync>;

/// Downloads, loads and deletes the local Whisper models (ENG-02, ENG-08).
pub struct ModelManager {
    store: ModelStore,
    client: reqwest::Client,
    slot: Arc<EngineSlot>,
    states: Mutex<HashMap<&'static str, ModelState>>,
    /// The model in the engine slot.
    loaded: Mutex<Option<&'static str>>,
    /// One download or load at a time; a later choice runs after.
    busy: tokio::sync::Mutex<()>,
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
            loaded: Mutex::new(None),
            busy: tokio::sync::Mutex::new(()),
            on_change,
        })
    }

    pub fn statuses(&self) -> Vec<ModelStatus> {
        let states = lock(&self.states);
        let loaded = *lock(&self.loaded);
        WHISPER_MODELS
            .iter()
            .map(|model| ModelStatus {
                id: model.id,
                label: model.label,
                size_bytes: model.size,
                is_default: model.id == DEFAULT_WHISPER_MODEL,
                in_use: loaded == Some(model.id),
                state: states.get(model.id).cloned().unwrap_or(ModelState::Missing),
            })
            .collect()
    }

    /// Downloads whatever `id` needs, then loads it in place of the current
    /// model, which keeps serving dictations until then. Unknown ids fall
    /// back to the default model.
    pub async fn prepare(self: Arc<Self>, id: String) {
        let _busy = self.busy.lock().await;
        let model = find_whisper_model(&id).unwrap_or_else(|| {
            find_whisper_model(DEFAULT_WHISPER_MODEL).expect("default model is in the catalog")
        });
        if *lock(&self.loaded) == Some(model.id) {
            return;
        }
        match self.ensure_files(model).await {
            Ok(()) => self.load(model).await,
            Err(message) => {
                if lock(&self.loaded).is_none() {
                    self.slot
                        .set_status(t!("error.models.downloadFailed", details = message));
                }
                self.set_state(model.id, ModelState::Failed { message });
            }
        }
    }

    /// Downloads `id` without loading it.
    pub async fn download(self: Arc<Self>, id: String) -> Result<(), String> {
        let model = find_whisper_model(&id).ok_or_else(|| t!("error.models.unknown"))?;
        let _busy = self.busy.lock().await;
        self.ensure_files(model).await.inspect_err(|message| {
            self.set_state(
                model.id,
                ModelState::Failed {
                    message: message.clone(),
                },
            );
        })
    }

    /// Deletes a downloaded model that is neither in use nor chosen.
    pub fn delete(&self, id: &str, chosen: &str) -> Result<(), String> {
        let model = find_whisper_model(id).ok_or_else(|| t!("error.models.unknown"))?;
        if *lock(&self.loaded) == Some(model.id) || model.id == chosen {
            return Err(t!("error.models.inUse"));
        }
        self.store
            .delete(model)
            .map_err(|error| t!("error.models.deleteFailed", details = error))?;
        log::info!("deleted {}", model.file_name);
        self.set_state(model.id, ModelState::Missing);
        Ok(())
    }

    async fn ensure_files(&self, model: &'static ModelFile) -> Result<(), String> {
        let first = lock(&self.loaded).is_none();
        if !self.store.is_downloaded(&VAD_MODEL) {
            self.slot.set_status(t!("error.models.downloadingVad"));
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
                    if first {
                        self.slot
                            .set_status(t!("error.models.downloading", percent = percent));
                    }
                    self.set_state(model.id, ModelState::Downloading { received, total });
                }
            })
            .await
            .map_err(|err| err.to_string())?;
        self.set_state(model.id, ModelState::Downloaded);
        Ok(())
    }

    async fn load(&self, model: &'static ModelFile) {
        if lock(&self.loaded).is_none() {
            self.slot.set_status(t!("error.models.loading"));
        }
        let path = self.store.path(model);
        let vad = self.store.path(&VAD_MODEL);
        let loaded =
            tokio::task::spawn_blocking(move || LocalWhisper::load(&path, Some(&vad))).await;
        match loaded {
            Ok(Ok(engine)) => {
                self.slot.set_engine(Arc::new(engine));
                let previous = lock(&self.loaded).replace(model.id);
                if let Some(previous) = previous {
                    lock(&self.states).insert(previous, ModelState::Downloaded);
                }
                self.set_state(model.id, ModelState::Active);
                log::info!("{} is loaded", model.file_name);
            }
            Ok(Err(error)) => self.fail_load(model, error.to_string()),
            Err(error) => self.fail_load(model, error.to_string()),
        }
    }

    fn fail_load(&self, model: &'static ModelFile, message: String) {
        log::error!("could not load {}: {message}", model.file_name);
        if lock(&self.loaded).is_none() {
            self.slot
                .set_status(t!("error.models.loadFailed", details = message));
        }
        self.set_state(model.id, ModelState::Failed { message });
    }

    fn set_state(&self, id: &'static str, state: ModelState) {
        lock(&self.states).insert(id, state);
        (self.on_change)(self.statuses());
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
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
            in_use: false,
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
                "inUse": false,
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

#[cfg(test)]
mod delete_tests {
    use super::*;

    #[test]
    fn only_models_neither_chosen_nor_loaded_can_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let tiny = find_whisper_model("tiny-q5_1").unwrap();
        std::fs::write(dir.path().join(tiny.file_name), b"model").unwrap();
        let slot = Arc::new(EngineSlot::new("starting"));
        let manager = ModelManager::new(dir.path().into(), slot, Box::new(|_| {}));
        assert!(manager.delete("tiny-q5_1", "tiny-q5_1").is_err());
        *lock(&manager.loaded) = Some("tiny-q5_1");
        assert!(manager.delete("tiny-q5_1", DEFAULT_WHISPER_MODEL).is_err());
        *lock(&manager.loaded) = None;
        manager.delete("tiny-q5_1", DEFAULT_WHISPER_MODEL).unwrap();
        assert!(!dir.path().join(tiny.file_name).exists());
        assert!(manager.delete("nope", DEFAULT_WHISPER_MODEL).is_err());
    }
}
