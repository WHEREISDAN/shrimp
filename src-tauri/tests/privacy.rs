//! Privacy mode (PRIV-03): dictations use only Ollama and local Whisper,
//! whatever the provider and engine chosen.

mod support;

use support::*;

#[test]
fn privacy_mode_sends_ai_edits_to_ollama() {
    let mut setup = ai(CleanupLevel::Light, vec![Ok("Hello from Shrimp.".into())]);
    setup.settings.ai.provider = LlmProviderId::Anthropic;
    setup.settings.privacy.privacy_mode = true;
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    let providers = h.fakes.providers.lock().unwrap().clone();
    assert!(!providers.is_empty());
    assert!(
        providers.iter().all(|id| *id == LlmProviderId::Ollama),
        "{providers:?}"
    );
}

#[test]
fn without_privacy_mode_the_chosen_provider_runs() {
    let mut setup = ai(CleanupLevel::Light, vec![Ok("Hello from Shrimp.".into())]);
    setup.settings.ai.provider = LlmProviderId::Groq;
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    assert!(
        h.fakes
            .providers
            .lock()
            .unwrap()
            .contains(&LlmProviderId::Groq)
    );
}
