use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use super::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};
use crate::audio::AudioClip;

/// The engine dictations use. It is empty until the model has downloaded
/// and loaded; meanwhile transcription fails with a status message.
pub struct EngineSlot {
    engine: RwLock<Option<Arc<dyn SttEngine>>>,
    status: RwLock<String>,
}

impl EngineSlot {
    pub fn new(status: impl Into<String>) -> Self {
        Self {
            engine: RwLock::new(None),
            status: RwLock::new(status.into()),
        }
    }

    pub fn set_engine(&self, engine: Arc<dyn SttEngine>) {
        *self.engine.write().unwrap_or_else(|err| err.into_inner()) = Some(engine);
    }

    /// Why no engine is available yet, e.g. "Downloading the speech model (42%)".
    pub fn set_status(&self, status: impl Into<String>) {
        *self.status.write().unwrap_or_else(|err| err.into_inner()) = status.into();
    }

    pub fn is_ready(&self) -> bool {
        self.current().is_some()
    }

    fn current(&self) -> Option<Arc<dyn SttEngine>> {
        self.engine
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }
}

#[async_trait]
impl SttEngine for EngineSlot {
    fn id(&self) -> SttEngineId {
        self.current()
            .map_or(SttEngineId::LocalWhisper, |engine| engine.id())
    }

    async fn transcribe(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
    ) -> Result<Transcript, SttError> {
        let Some(engine) = self.current() else {
            let status = self.status.read().unwrap_or_else(|err| err.into_inner());
            return Err(SttError::Unavailable(status.clone()));
        };
        engine.transcribe(clip, options).await
    }
}
