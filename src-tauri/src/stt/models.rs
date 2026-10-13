//! Whisper and VAD model files: a pinned catalog, downloads with progress and
//! SHA-256 verification, and reuse of verified files (ENG-02).

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// A model file with its pinned size and SHA-256 (from the Hugging Face API).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile {
    pub id: &'static str,
    pub label: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

/// Silero VAD for whisper.cpp: trims silence and stops hallucinated text (ENG-03).
pub const VAD_MODEL: ModelFile = ModelFile {
    id: "silero-v6.2.0",
    label: "Silero VAD 6.2.0",
    file_name: "ggml-silero-v6.2.0.bin",
    url: "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin",
    size: 885_098,
    sha256: "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987",
};

/// A whisper.cpp model from the `ggerganov/whisper.cpp` Hugging Face repository.
macro_rules! whisper_model {
    ($id:literal, $label:literal, $file:literal, $size:literal, $sha256:literal) => {
        ModelFile {
            id: $id,
            label: $label,
            file_name: $file,
            url: concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/",
                $file
            ),
            size: $size,
            sha256: $sha256,
        }
    };
}

pub const WHISPER_MODELS: [ModelFile; 5] = [
    whisper_model!(
        "tiny-q5_1",
        "Whisper Tiny",
        "ggml-tiny-q5_1.bin",
        32_152_673,
        "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7"
    ),
    whisper_model!(
        "base-q5_1",
        "Whisper Base",
        "ggml-base-q5_1.bin",
        59_707_625,
        "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898"
    ),
    whisper_model!(
        "small-q5_1",
        "Whisper Small",
        "ggml-small-q5_1.bin",
        190_085_487,
        "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb"
    ),
    whisper_model!(
        "medium-q5_0",
        "Whisper Medium",
        "ggml-medium-q5_0.bin",
        539_212_467,
        "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f"
    ),
    whisper_model!(
        "large-v3-turbo-q5_0",
        "Whisper Large v3 Turbo",
        "ggml-large-v3-turbo-q5_0.bin",
        574_041_195,
        "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2"
    ),
];

pub const DEFAULT_WHISPER_MODEL: &str = "large-v3-turbo-q5_0";

pub fn find_whisper_model(id: &str) -> Option<&'static ModelFile> {
    WHISPER_MODELS.iter().find(|model| model.id == id)
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("download failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("could not save the model: {0}")]
    Io(#[from] std::io::Error),
    #[error("the downloaded file is corrupt (checksum mismatch)")]
    Checksum,
}

/// Where model files live: `<app data>/models/`.
#[derive(Debug, Clone)]
pub struct ModelStore {
    dir: PathBuf,
}

impl ModelStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn path(&self, model: &ModelFile) -> PathBuf {
        self.dir.join(model.file_name)
    }

    /// Files are renamed into place only after their checksum matched, so a
    /// file with the pinned size is a verified download.
    pub fn is_downloaded(&self, model: &ModelFile) -> bool {
        std::fs::metadata(self.path(model)).is_ok_and(|meta| meta.len() == model.size)
    }

    pub fn delete(&self, model: &ModelFile) -> std::io::Result<()> {
        std::fs::remove_file(self.path(model))
    }

    /// Streams the file to a temporary path while hashing it, then renames it
    /// into place. `on_progress(received, total)` is called as bytes arrive.
    pub async fn download(
        &self,
        client: &reqwest::Client,
        model: &ModelFile,
        mut on_progress: impl FnMut(u64, u64),
    ) -> Result<PathBuf, ModelError> {
        std::fs::create_dir_all(&self.dir)?;
        let target = self.path(model);
        let partial = target.with_extension("part");
        let result = fetch(client, model, &partial, &mut on_progress).await;
        if result.is_err() {
            let _ = std::fs::remove_file(&partial);
        }
        result?;
        std::fs::rename(&partial, &target)?;
        Ok(target)
    }
}

async fn fetch(
    client: &reqwest::Client,
    model: &ModelFile,
    partial: &Path,
    on_progress: &mut impl FnMut(u64, u64),
) -> Result<(), ModelError> {
    let mut response = client.get(model.url).send().await?.error_for_status()?;
    let mut file = File::create(partial)?;
    let mut hasher = Sha256::new();
    let mut received = 0u64;
    on_progress(0, model.size);
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        hasher.update(&chunk);
        received += chunk.len() as u64;
        on_progress(received, model.size);
    }
    file.sync_all()?;
    if received != model.size || hex(&hasher.finalize()) != model.sha256 {
        return Err(ModelError::Checksum);
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_match_file_names() {
        for model in WHISPER_MODELS.iter().chain([&VAD_MODEL]) {
            assert!(model.url.ends_with(model.file_name), "{}", model.id);
            assert_eq!(model.sha256.len(), 64);
        }
    }

    #[test]
    fn the_default_model_is_in_the_catalog() {
        assert!(find_whisper_model(DEFAULT_WHISPER_MODEL).is_some());
    }

    #[test]
    fn only_files_with_the_pinned_size_count_as_downloaded() {
        let dir = tempfile::tempdir().unwrap();
        let store = ModelStore::new(dir.path());
        assert!(!store.is_downloaded(&VAD_MODEL));
        std::fs::write(store.path(&VAD_MODEL), b"partial").unwrap();
        assert!(!store.is_downloaded(&VAD_MODEL));
    }

    #[test]
    fn hex_encodes_lowercase() {
        assert_eq!(hex(&[0x00, 0xab, 0x7f]), "00ab7f");
    }
}
