//! Developer apps through the controller: identifiers on screen, spoken
//! case commands, shell commands in terminals and file tagging in Cursor's
//! chat (DEV-01..03).

mod support;

use shrimp_lib::platform::{Editable, FieldKind};
use shrimp_lib::storage::{DictionaryEntry, EntrySource};
use support::*;

const CURSOR: &str = "com.todesktop.230313mzl4w4u92";

fn editor(code: &str) -> FieldContext {
    FieldContext {
        field_id: 3,
        kind: FieldKind::Code,
        editable: Editable::Yes,
        selection: Some(TextRange {
            start: code.encode_utf16().count(),
            len: 0,
        }),
        before: code.into(),
        text: Some(code.into()),
        length: code.encode_utf16().count(),
        in_web_content: true,
        ..FieldContext::default()
    }
}

fn chat(screen: &[&str]) -> FieldContext {
    FieldContext {
        field_id: 4,
        kind: FieldKind::Text,
        editable: Editable::Yes,
        selection: Some(TextRange { start: 0, len: 0 }),
        in_web_content: true,
        window_title: Some("scratch.ts — ide".into()),
        screen_text: screen.iter().map(|text| (*text).to_owned()).collect(),
        ..FieldContext::default()
    }
}

fn run(app_id: &str, name: &str, field: FieldContext, transcript: &str, setup: Setup) -> Harness {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            app: Some(app(app_id, name)),
            field: Some(field),
            transcript: Some(transcript.into()),
            ..setup
        },
    );
    dictate(&h);
    h
}

#[test]
fn spoken_identifiers_and_case_commands_become_code() {
    let h = run(
        CURSOR,
        "Cursor",
        editor("const userId = 1;\nconst maxRetryCount = 3;\n"),
        "Set the user ID to the max retry count and camel case user profile to the api result.",
        Setup::default(),
    );
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    assert_eq!(
        h.fakes.inserted(),
        ["Set the userId to the maxRetryCount and userProfile to the API result."]
    );
}

#[test]
fn the_ai_edit_is_told_to_keep_identifiers() {
    let h = run(
        CURSOR,
        "Cursor",
        editor("let userId = 1;\n"),
        "check the user ID",
        ai(CleanupLevel::Light, vec![Ok("Check the userId.".into())]),
    );
    wait_until("insertion", || h.fakes.inserted() == ["Check the userId."]);
    let prompt = h.fakes.prompts.lock().unwrap()[0].clone();
    assert!(prompt.messages[0].content.contains("userId"));
    assert!(prompt.system.contains("userId"), "listed to spell exactly");
}

#[test]
fn a_command_in_a_terminal_is_written_as_a_command_without_the_ai_edit() {
    let h = run(
        "com.apple.Terminal",
        "Terminal",
        FieldContext {
            editable: Editable::Yes,
            selection: Some(TextRange { start: 20, len: 0 }),
            before: "dan@mac ~/shrimp % ".into(),
            ..FieldContext::default()
        },
        "Git checkout dash b feature slash login.",
        ai(
            CleanupLevel::Light,
            vec![Ok("Git checkout -b feature/login.".into())],
        ),
    );
    wait_until("insertion", || {
        h.fakes.inserted() == ["git checkout -b feature/login"]
    });
    assert!(h.fakes.prompts.lock().unwrap().is_empty(), "no AI edit");

    let h = run(
        "com.apple.Terminal",
        "Terminal",
        FieldContext::default(),
        "Explain why the build failed.",
        Setup::default(),
    );
    wait_until("prose stays prose", || {
        h.fakes.inserted() == ["Explain why the build failed."]
    });
}

#[test]
fn a_file_name_in_cursors_chat_becomes_a_mention() {
    let h = run(
        CURSOR,
        "Cursor",
        chat(&["index.ts", "my_python_script.py"]),
        "Check index dot ts and tag my python script.",
        Setup::default(),
    );
    wait_until("insertion", || {
        h.fakes.inserted() == ["Check @index.ts and @my_python_script.py."]
    });
}

#[test]
fn file_tagging_can_be_turned_off_and_skips_corrected_dictations() {
    let mut settings = Settings::default();
    settings.vibe_coding.file_tagging = false;
    let h = run(
        CURSOR,
        "Cursor",
        chat(&["index.ts"]),
        "Check index dot ts.",
        Setup {
            settings,
            ..Setup::default()
        },
    );
    wait_until("insertion", || {
        h.fakes.inserted() == ["Check index dot ts."]
    });

    let correction = DictionaryEntry {
        id: 1,
        term: "Shrimp".into(),
        misspelling: Some("shrimpy".into()),
        source: EntrySource::Manual,
        pinned: false,
        uses: 0,
        created_at: 0,
    };
    let h = run(
        CURSOR,
        "Cursor",
        chat(&["index.ts"]),
        "Ask shrimpy about index dot ts.",
        Setup {
            library: Snapshot {
                dictionary: vec![correction],
                snippets: Vec::new(),
            },
            ..Setup::default()
        },
    );
    wait_until("insertion", || {
        h.fakes.inserted() == ["Ask Shrimp about index dot ts."]
    });
}

#[test]
fn the_ide_chat_is_formatted_like_an_ai_app() {
    let h = run(
        CURSOR,
        "Cursor",
        chat(&[]),
        "make a list one apples two pears",
        ai(
            CleanupLevel::Light,
            vec![Ok("Make a list: apples, pears.".into())],
        ),
    );
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    let prompt = h.fakes.prompts.lock().unwrap()[0].clone();
    assert!(prompt.system.contains("plain prose"));
}
