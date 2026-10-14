//! User settings: one typed struct, saved as JSON in the app data folder.
//! The webview reads them with `get_settings` and changes them with
//! `update_settings(patch)`; Rust validates, saves and announces each change.

pub mod audio;
pub mod flow_bar;
pub mod languages;
mod model;
pub mod shortcuts;
pub mod transforms;

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde_json::Value;

use crate::t;

pub use audio::{KnownMicrophone, MediaSettings, MicrophoneSettings, SoundSettings};
pub use flow_bar::FlowBarSettings;
pub use languages::LanguageSettings;
pub use model::{
    APP_LANGUAGES, AiSettings, AppSettings, DictionarySettings, DictionarySort,
    ExperimentalSettings, FormattingTips, GrantedPermissions, HistoryStorage, NoticeSettings,
    NotificationSettings, OnboardingSettings, OnboardingStep, PrivacySettings, SETTINGS_VERSION,
    Settings, SttSettings, StyleSettings, VibeCodingSettings,
};
pub use shortcuts::ShortcutSettings;
pub use transforms::{CustomTransform, PolishRules, TransformSettings};

type OnChange = Box<dyn Fn(&Settings) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{0}")]
    Invalid(String),
    #[error("{}", t!("error.settings.saveFailed", details = .0))]
    Io(#[from] std::io::Error),
}

pub struct SettingsStore {
    path: Option<PathBuf>,
    current: RwLock<Settings>,
    on_change: RwLock<Vec<OnChange>>,
}

impl SettingsStore {
    /// Loads `path`, or the defaults when it is missing or unreadable.
    pub fn load(path: PathBuf) -> Self {
        let settings = read(&path).unwrap_or_default();
        Self {
            path: Some(path),
            current: RwLock::new(settings),
            on_change: RwLock::new(Vec::new()),
        }
    }

    /// Settings that live only in memory, for tests.
    pub fn in_memory(mut settings: Settings) -> Self {
        settings.normalize();
        Self {
            path: None,
            current: RwLock::new(settings),
            on_change: RwLock::new(Vec::new()),
        }
    }

    /// Adds a listener called after every saved change.
    pub fn on_change(&self, callback: OnChange) {
        self.on_change
            .write()
            .unwrap_or_else(|err| err.into_inner())
            .push(callback);
    }

    pub fn get(&self) -> Settings {
        self.current
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }

    /// Applies a JSON merge patch (RFC 7396), validates, saves and notifies.
    pub fn update(&self, patch: &Value) -> Result<Settings, SettingsError> {
        let mut current = self.current.write().unwrap_or_else(|err| err.into_inner());
        let mut merged = serde_json::to_value(&*current)
            .map_err(|err| SettingsError::Invalid(err.to_string()))?;
        merge(&mut merged, patch);
        let mut next: Settings = serde_json::from_value(merged)
            .map_err(|err| SettingsError::Invalid(err.to_string()))?;
        next.revision = current.revision + 1;
        next.normalize();
        next.validate().map_err(SettingsError::Invalid)?;
        if let Some(path) = &self.path {
            write(path, &next)?;
        }
        *current = next.clone();
        drop(current);
        for callback in self
            .on_change
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .iter()
        {
            callback(&next);
        }
        Ok(next)
    }
}

fn read(path: &Path) -> Option<Settings> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut value: Value = serde_json::from_str(&text).ok()?;
    // A file written before onboarding existed is a set-up install (ONB-07).
    if let Value::Object(fields) = &mut value
        && !fields.contains_key("onboarding")
    {
        let done = serde_json::to_value(OnboardingSettings::done()).ok()?;
        fields.insert("onboarding".into(), done);
    }
    match serde_json::from_value::<Settings>(value) {
        Ok(mut settings) => {
            settings.normalize();
            if let Err(error) = settings.validate() {
                log::warn!(
                    "{} is invalid ({error}); using the default settings",
                    path.display()
                );
                return None;
            }
            Some(settings)
        }
        Err(_) => {
            log::warn!("{} is invalid; using the default settings", path.display());
            None
        }
    }
}

/// Writes through a temporary file, so a crash never leaves half a file.
fn write(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path)
}

fn merge(target: &mut Value, patch: &Value) {
    let Value::Object(patch) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(serde_json::Map::new());
    }
    let Value::Object(target) = target else {
        return;
    };
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            merge(target.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;

    use super::*;
    use crate::edit::{CleanupLevel, Style};

    #[test]
    fn a_patch_changes_only_what_it_names_and_is_saved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(path.clone());
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        store.on_change(Box::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));

        let updated = store
            .update(&json!({"ai": {"cleanup": "medium"}, "styles": {"work": "excited"}}))
            .unwrap();
        assert_eq!(updated.ai.cleanup, CleanupLevel::Medium);
        assert_eq!(updated.styles.work, Style::Excited);
        assert_eq!(updated.styles.personal, Style::Formal);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let reloaded = SettingsStore::load(path);
        assert_eq!(reloaded.get(), updated);
        assert_eq!(updated.revision, 1);
        let next = reloaded.update(&json!({"revision": 0})).unwrap();
        assert_eq!(next.revision, 2, "a patch can't set the revision");
    }

    #[test]
    fn invalid_patches_change_nothing() {
        let store = SettingsStore::in_memory(Settings::default());
        assert!(store.update(&json!({"ai": {"cleanup": "loud"}})).is_err());
        assert!(
            store
                .update(&json!({"styles": {"email": "veryCasual"}}))
                .is_err()
        );
        assert_eq!(store.get(), Settings::default());
    }

    #[test]
    fn null_restores_a_default() {
        let store = SettingsStore::in_memory(Settings::default());
        store.update(&json!({"ai": {"cleanup": "none"}})).unwrap();
        let restored = store.update(&json!({"ai": {"cleanup": null}})).unwrap();
        assert_eq!(restored.ai.cleanup, CleanupLevel::Light);
        assert_eq!(restored.revision, 2);
    }

    #[test]
    fn an_install_from_before_onboarding_counts_as_set_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"ai": {"cleanup": "medium"}}"#).unwrap();
        assert!(SettingsStore::load(path).get().onboarding.completed);
        let fresh = SettingsStore::load(dir.path().join("missing.json"));
        assert!(!fresh.get().onboarding.completed);
    }

    #[test]
    fn a_corrupt_file_loads_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(SettingsStore::load(path).get(), Settings::default());
    }
}
