//! Context awareness through the controller: casing and spacing from the
//! field, names on screen and the privacy switch, websites in browsers, and
//! how terminals and remote desktops are pasted into.

mod support;

use std::time::{Duration, Instant};

use shrimp_lib::platform::{Editable, PasteKey, TextRange};
use support::*;

fn field(before: &str, screen: &[&str]) -> FieldContext {
    FieldContext {
        editable: Editable::Yes,
        selection: Some(TextRange {
            start: before.encode_utf16().count(),
            len: 0,
        }),
        before: before.into(),
        in_web_content: true,
        screen_text: screen.iter().map(|text| (*text).to_owned()).collect(),
        ..FieldContext::default()
    }
}

fn run(setup: Setup) -> Harness {
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    dictate(&h);
    h
}

fn text_edit() -> Option<AppIdentity> {
    Some(app("com.apple.TextEdit", "TextEdit"))
}

#[test]
fn a_dictation_continues_the_sentence_before_the_cursor() {
    let h = run(Setup {
        app: text_edit(),
        field: Some(field("we should talk about it", &[])),
        ..Setup::default()
    });
    wait_until("insertion", || {
        h.fakes.inserted() == [" hello from Shrimp."]
    });
}

#[test]
fn names_on_screen_reach_providers_only_with_context_awareness() {
    for aware in [true, false] {
        let mut settings = Settings::default();
        settings.privacy.context_awareness = aware;
        let h = run(Setup {
            app: text_edit(),
            settings,
            field: Some(field("Hi Kathryn,", &["To: Siobhan Nguyen"])),
            ..ai(CleanupLevel::Light, vec![Ok("Hello from Shrimp.".into())])
        });
        // Casing and spacing use the field either way (CTX-04).
        wait_until("insertion", || {
            h.fakes.inserted() == [" hello from Shrimp."]
        });
        let prompt = h.fakes.prompts.lock().unwrap()[0].clone();
        assert_eq!(prompt.system.contains("Kathryn"), aware, "aware: {aware}");
        assert_eq!(prompt.system.contains("Nguyen"), aware, "aware: {aware}");
        assert!(
            !prompt.messages[0].content.contains("Kathryn"),
            "the field text itself is never sent"
        );
        let vocabulary = h.fakes.vocabulary.lock().unwrap()[0].clone();
        assert_eq!(vocabulary.contains(&"Nguyen".to_owned()), aware);
    }
}

#[test]
fn a_name_on_screen_fixes_the_spelling() {
    let h = run(Setup {
        app: text_edit(),
        field: Some(field("", &["To: Kathryn Shaw"])),
        transcript: Some("Please forward this to Catherine.".into()),
        ..Setup::default()
    });
    wait_until("insertion", || {
        h.fakes.inserted() == ["Please forward this to Kathryn."]
    });
}

#[test]
fn a_slow_read_never_delays_the_dictation() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            app: text_edit(),
            field: Some(field("we should talk about it", &[])),
            ..Setup::default()
        },
    );
    *h.fakes.field_delay.lock().unwrap() = Duration::from_secs(3);
    dictate(&h);
    let released = Instant::now();
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    assert!(released.elapsed() < Duration::from_secs(2));
}

#[test]
fn a_website_sets_the_category_in_a_browser() {
    let mut page = field("", &[]);
    page.url = Some("https://mail.google.com/mail/u/0/#inbox".into());
    let h = run(Setup {
        app: Some(app("com.google.Chrome", "Google Chrome")),
        field: Some(page),
        ..ai(CleanupLevel::Light, vec![Ok("Hello from Shrimp.".into())])
    });
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    assert!(
        h.fakes.prompts.lock().unwrap()[0]
            .system
            .contains("This is an email")
    );
}

#[test]
fn terminals_paste_in_parts_without_a_field_check() {
    let h = run(Setup {
        app: Some(app("com.apple.Terminal", "Terminal")),
        ..Setup::default()
    });
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    let options = h.fakes.insert_options.lock().unwrap()[0];
    assert!(!options.require_field);
    let split = options.split.expect("terminal pastes are split");
    assert!(split.max_chars < 800 && split.max_breaks <= 2);
}

#[test]
fn windows_editors_and_terminals_paste_with_shift_insert() {
    for (exe, name, key) in [
        ("Code.exe", "Visual Studio Code", PasteKey::ShiftInsert),
        (
            "WindowsTerminal.exe",
            "Windows Terminal",
            PasteKey::ShiftInsert,
        ),
        ("cmd.exe", "Command Prompt", PasteKey::ShiftInsert),
        ("notepad.exe", "Notepad", PasteKey::Default),
    ] {
        let h = run(Setup {
            app: Some(app(exe, name)),
            field: Some(field("", &[])),
            ..Setup::default()
        });
        wait_until("insertion", || !h.fakes.inserted().is_empty());
        assert_eq!(
            h.fakes.insert_options.lock().unwrap()[0].paste_key,
            key,
            "{exe}"
        );
    }
}

#[test]
fn late_clipboard_readers_hold_the_clipboard_for_three_seconds() {
    for exe in ["OUTLOOK.EXE", "WINWORD.EXE", "WhatsApp.Root.exe"] {
        let h = run(Setup {
            app: Some(app(exe, exe)),
            field: Some(field("", &[])),
            ..Setup::default()
        });
        wait_until("insertion", || !h.fakes.inserted().is_empty());
        assert_eq!(
            h.fakes.insert_options.lock().unwrap()[0].hold,
            Duration::from_secs(3),
            "{exe}"
        );
    }
}

#[test]
fn remote_desktops_hold_the_clipboard_and_skip_context_spacing() {
    let h = run(Setup {
        app: Some(app("com.apple.ScreenSharing", "Screen Sharing")),
        field: Some(field("we should talk about it", &[])),
        ..Setup::default()
    });
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    let options = h.fakes.insert_options.lock().unwrap()[0];
    assert_eq!(options.hold, Duration::from_secs(5));
    assert!(!options.require_field);
    assert!(options.split.is_none());
}
