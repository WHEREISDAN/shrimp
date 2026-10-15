//! Formatting per app (FMT-15) through the controller: unformatted
//! transcription, proper-noun capitals only and all lowercase.

mod support;

use shrimp_lib::edit::TranscriptFormat;
use shrimp_lib::platform::Editable;
use shrimp_lib::settings::AppFormat;
use support::*;

fn notes() -> AppIdentity {
    app("com.apple.Notes", "Notes")
}

fn formatted(format: TranscriptFormat) -> Settings {
    Settings {
        app_formats: vec![AppFormat {
            app_id: "com.apple.Notes".into(),
            app_name: "Notes".into(),
            format,
        }],
        ..Settings::default()
    }
}

fn library() -> Snapshot {
    Snapshot {
        dictionary: vec![DictionaryEntry {
            id: 1,
            term: "Kubernetes".into(),
            misspelling: Some("cooper netties".into()),
            source: EntrySource::Manual,
            pinned: false,
            uses: 0,
            created_at: 0,
        }],
        snippets: vec![Snippet {
            id: 1,
            trigger: "my sig".into(),
            expansion: "Best, Alex".into(),
            created_at: 0,
            updated_at: 0,
        }],
    }
}

/// The cursor after `before` in a plain text field.
fn after(before: &str) -> FieldContext {
    FieldContext {
        field_id: 3,
        editable: Editable::Yes,
        selection: Some(TextRange {
            start: before.encode_utf16().count(),
            len: 0,
        }),
        before: before.into(),
        ..FieldContext::default()
    }
}

fn run(setup: Setup) -> Harness {
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    h
}

#[test]
fn unformatted_skips_the_ai_edit_style_and_context_casing() {
    let h = run(Setup {
        cleanup: CleanupLevel::Light,
        replies: vec![Ok("Edited by the model.".into())],
        app: Some(notes()),
        field: Some(after("We moved")),
        settings: formatted(TranscriptFormat::Unformatted),
        library: library(),
        transcript: Some("Onto cooper netties comma finally new line my sig".into()),
        ..Setup::default()
    });
    assert_eq!(
        h.fakes.inserted(),
        [" Onto Kubernetes comma finally\nBest, Alex"],
        "line commands, corrections and snippets apply; no casing, punctuation words or model"
    );
    assert!(h.fakes.prompts.lock().unwrap().is_empty(), "no AI edit");
}

#[test]
fn other_apps_keep_automatic_formatting() {
    let h = run(Setup {
        cleanup: CleanupLevel::Light,
        replies: vec![Ok("Onto Kubernetes, finally.".into())],
        app: Some(app("com.apple.TextEdit", "TextEdit")),
        settings: formatted(TranscriptFormat::Unformatted),
        library: library(),
        transcript: Some("onto cooper netties comma finally".into()),
        ..Setup::default()
    });
    assert_eq!(h.fakes.inserted(), ["Onto Kubernetes, finally."]);
    assert_eq!(h.fakes.prompts.lock().unwrap().len(), 1);
}

#[test]
fn proper_noun_capitals_only_lowercases_everything_else() {
    let h = run(Setup {
        cleanup: CleanupLevel::Light,
        replies: vec![Ok(
            "Sounds good. Ask Siobhan about Kubernetes. I think so.".into()
        )],
        app: Some(notes()),
        settings: formatted(TranscriptFormat::ProperNouns),
        library: library(),
        transcript: Some("sounds good ask Siobhan about cooper netties i think so".into()),
        ..Setup::default()
    });
    assert_eq!(
        h.fakes.inserted(),
        ["sounds good. ask Siobhan about Kubernetes. I think so."]
    );
}

#[test]
fn all_lowercase_covers_the_whole_dictation_but_not_snippets() {
    let h = run(Setup {
        app: Some(notes()),
        settings: formatted(TranscriptFormat::Lowercase),
        library: library(),
        transcript: Some("Thanks Siobhan. I will send it. My sig".into()),
        ..Setup::default()
    });
    assert_eq!(
        h.fakes.inserted(),
        ["thanks siobhan. i will send it. Best, Alex"]
    );
}

#[test]
fn a_format_for_an_app_that_was_added_twice_is_rejected() {
    let store = SettingsStore::in_memory(Settings::default());
    let entry =
        serde_json::json!({"appId": "com.apple.Notes", "appName": "Notes", "format": "lowercase"});
    assert!(
        store
            .update(&serde_json::json!({"appFormats": [entry.clone(), entry]}))
            .is_err()
    );
    let saved = store
        .update(&serde_json::json!({"appFormats": [
            {"appId": "com.apple.Notes", "appName": "Notes", "format": "properNouns"}
        ]}))
        .unwrap();
    assert_eq!(saved.app_formats[0].format, TranscriptFormat::ProperNouns);
}
