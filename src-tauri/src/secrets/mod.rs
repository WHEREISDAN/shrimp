//! API keys in the OS keychain (PRIV-02). Keys are never written to settings,
//! logs or the webview; the UI only learns whether one is set.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::t;

const MAX_KEY_LENGTH: usize = 512;

/// The keychain account a key is stored under. One key serves both a
/// provider's LLM and its speech-to-text API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyAccount {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    Groq,
}

impl KeyAccount {
    pub const ALL: [KeyAccount; 3] = [Self::Anthropic, Self::OpenAi, Self::Groq];

    pub fn account(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Groq => "groq",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::Groq => "Groq",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("{0}")]
    Invalid(String),
    #[error("keychain error: {0}")]
    Keychain(String),
}

/// Where keys live: the OS keychain in the app, memory in tests.
pub trait SecretBackend: Send + Sync {
    fn read(&self, account: KeyAccount) -> Result<Option<String>, SecretError>;
    fn write(&self, account: KeyAccount, key: &str) -> Result<(), SecretError>;
    fn remove(&self, account: KeyAccount) -> Result<(), SecretError>;
}

/// Keys with an in-memory cache, so dictations don't hit the keychain each time.
pub struct Secrets {
    backend: Box<dyn SecretBackend>,
    cache: Mutex<HashMap<KeyAccount, Option<String>>>,
}

impl Secrets {
    pub fn new(backend: Box<dyn SecretBackend>) -> Self {
        Self {
            backend,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, account: KeyAccount) -> Option<String> {
        let mut cache = self.cache.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(cached) = cache.get(&account) {
            return cached.clone();
        }
        match self.backend.read(account) {
            Ok(key) => {
                cache.insert(account, key.clone());
                key
            }
            Err(error) => {
                log::warn!("could not read the {} key: {error}", account.label());
                None
            }
        }
    }

    pub fn has(&self, account: KeyAccount) -> bool {
        self.get(account).is_some()
    }

    pub fn set(&self, account: KeyAccount, key: &str) -> Result<(), SecretError> {
        let key = key.trim();
        if key.is_empty() {
            return Err(SecretError::Invalid(t!("error.secrets.empty")));
        }
        if key.len() > MAX_KEY_LENGTH || key.chars().any(char::is_whitespace) {
            return Err(SecretError::Invalid(t!("error.secrets.malformed")));
        }
        self.backend.write(account, key)?;
        self.cache
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(account, Some(key.to_owned()));
        Ok(())
    }

    pub fn delete(&self, account: KeyAccount) -> Result<(), SecretError> {
        self.backend.remove(account)?;
        self.cache
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(account, None);
        Ok(())
    }
}

/// The OS keychain through keyring: service = app identifier, account = provider.
pub struct Keychain {
    service: String,
}

impl Keychain {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, account: KeyAccount) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(&self.service, account.account())
            .map_err(|err| SecretError::Keychain(err.to_string()))
    }
}

impl SecretBackend for Keychain {
    fn read(&self, account: KeyAccount) -> Result<Option<String>, SecretError> {
        match self.entry(account)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(SecretError::Keychain(error.to_string())),
        }
    }

    fn write(&self, account: KeyAccount, key: &str) -> Result<(), SecretError> {
        self.entry(account)?
            .set_password(key)
            .map_err(|err| SecretError::Keychain(err.to_string()))
    }

    fn remove(&self, account: KeyAccount) -> Result<(), SecretError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretError::Keychain(error.to_string())),
        }
    }
}

/// Keys held in memory only, for tests and evaluation runs.
#[derive(Default)]
pub struct MemorySecrets(Mutex<HashMap<KeyAccount, String>>);

impl MemorySecrets {
    pub fn with(keys: impl IntoIterator<Item = (KeyAccount, String)>) -> Self {
        Self(Mutex::new(keys.into_iter().collect()))
    }
}

impl SecretBackend for MemorySecrets {
    fn read(&self, account: KeyAccount) -> Result<Option<String>, SecretError> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .get(&account)
            .cloned())
    }

    fn write(&self, account: KeyAccount, key: &str) -> Result<(), SecretError> {
        self.0
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(account, key.to_owned());
        Ok(())
    }

    fn remove(&self, account: KeyAccount) -> Result<(), SecretError> {
        self.0
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .remove(&account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_trimmed_validated_and_cached() {
        let secrets = Secrets::new(Box::new(MemorySecrets::default()));
        assert!(!secrets.has(KeyAccount::Groq));
        assert!(secrets.set(KeyAccount::Groq, "   ").is_err());
        assert!(secrets.set(KeyAccount::Groq, "gsk abc").is_err());
        secrets.set(KeyAccount::Groq, " gsk_abc \n").unwrap();
        assert_eq!(secrets.get(KeyAccount::Groq).as_deref(), Some("gsk_abc"));
        secrets.delete(KeyAccount::Groq).unwrap();
        assert!(!secrets.has(KeyAccount::Groq));
    }

    #[test]
    fn accounts_serialize_as_provider_names() {
        assert_eq!(
            serde_json::to_value(KeyAccount::OpenAi).unwrap(),
            serde_json::json!("openai")
        );
        assert_eq!(KeyAccount::OpenAi.account(), "openai");
    }
}
