//! Local Whisper on the WAV fixtures. These need downloaded model files, so
//! they are ignored by default:
//!
//! ```sh
//! SHRIMP_MODELS_DIR=/path/to/models cargo test --test whisper_local -- --ignored --nocapture
//! ```
//!
//! `SHRIMP_MODELS_DIR` defaults to the dev app's model folder;
//! `SHRIMP_WHISPER_MODEL` picks a catalog model (default: the app default).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use shrimp_lib::audio::{AudioClip, Pacing, Recorder, WavFileSource, level};
use shrimp_lib::dictation::transcribe;
use shrimp_lib::stt::models::{DEFAULT_WHISPER_MODEL, ModelStore, VAD_MODEL, find_whisper_model};
use shrimp_lib::stt::{LocalWhisper, SttEngine, SttError, SttOptions};

fn models_dir() -> PathBuf {
    std::env::var_os("SHRIMP_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").expect("HOME is set");
            PathBuf::from(home).join("Library/Application Support/com.shrimp.desktop.dev/models")
        })
}

fn engine() -> LocalWhisper {
    let id = std::env::var("SHRIMP_WHISPER_MODEL").unwrap_or(DEFAULT_WHISPER_MODEL.into());
    let model = find_whisper_model(&id).expect("model id is in the catalog");
    let store = ModelStore::new(models_dir());
    assert!(
        store.is_downloaded(model) && store.is_downloaded(&VAD_MODEL),
        "download {} and {} into {} first",
        model.file_name,
        VAD_MODEL.file_name,
        models_dir().display()
    );
    LocalWhisper::load(&store.path(model), Some(&store.path(&VAD_MODEL))).unwrap()
}

fn english() -> SttOptions {
    SttOptions {
        languages: vec!["en".into()],
        ..SttOptions::default()
    }
}

fn clip(name: &str) -> AudioClip {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/audio")
        .join(name);
    let mut recorder =
        Recorder::start(Box::new(WavFileSource::new(path, Pacing::Instant))).unwrap();
    assert!(recorder.wait_until_ended(Duration::from_secs(10)).unwrap());
    recorder.finish().unwrap()
}

fn words(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn transcribes_the_fixture_offline() {
    let engine = engine();
    let clip = clip("hello-shrimp.wav");

    for (name, options) in [("en", english()), ("auto", SttOptions::default())] {
        let mut latencies = Vec::new();
        let mut text = String::new();
        for _ in 0..3 {
            let started = Instant::now();
            text = transcribe(&engine, &clip, &options).await.unwrap().text;
            latencies.push(started.elapsed().as_millis());
        }
        println!("{name}: {text:?}; latency per run (ms): {latencies:?}");

        let words = words(&text);
        assert!(words.contains("hello from shrimp"), "{words}");
        assert!(words.contains("test of local dictation"), "{words}");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn silence_produces_no_text() {
    let engine = engine();
    let silence = clip("silence.wav");

    assert!(matches!(
        transcribe(&engine, &silence, &english()).await,
        Err(SttError::NoSpeech)
    ));
    // Even without the trim, speech detection keeps Whisper from inventing words.
    let raw = engine.transcribe(&silence, &english()).await;
    assert!(matches!(raw, Err(SttError::NoSpeech)), "{raw:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn leading_and_trailing_silence_is_trimmed_before_stt() {
    let engine = engine();
    let padded = clip("hello-shrimp-padded.wav");
    let plain = clip("hello-shrimp.wav");

    let trimmed = level::trim_silence(&padded).unwrap();
    println!(
        "padded {:.2} s, trimmed to {:.2} s (speech alone is {:.2} s)",
        padded.duration().as_secs_f32(),
        trimmed.duration().as_secs_f32(),
        plain.duration().as_secs_f32()
    );
    assert!(trimmed.duration() < plain.duration() + Duration::from_millis(700));

    let text = transcribe(&engine, &padded, &english()).await.unwrap().text;
    assert!(words(&text).contains("hello from shrimp"), "{text}");
}

/// DIC-02: `ask-nguyen.wav` ("Ask Nguyen to deploy Shrimp with Tauri.", from
/// macOS `say`) comes out as "Ask Wyn to deploy shrimp with Tory." until the
/// dictionary terms reach Whisper's initial prompt.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn dictionary_terms_fix_misrecognised_words() {
    let engine = engine();
    let clip = clip("ask-nguyen.wav");
    let plain = transcribe(&engine, &clip, &english()).await.unwrap().text;
    let options = SttOptions {
        languages: vec!["en".into()],
        vocabulary: vec!["Nguyen".into(), "Shrimp".into(), "Tauri".into()],
        ..SttOptions::default()
    };
    let biased = transcribe(&engine, &clip, &options).await.unwrap().text;
    println!("without the dictionary: {plain}\nwith it: {biased}");
    assert!(
        !plain.contains("Nguyen") && !plain.contains("Tauri"),
        "{plain}"
    );
    for term in ["Nguyen", "Shrimp", "Tauri"] {
        assert!(biased.contains(term), "{biased}");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn whispered_speech_transcribes_after_its_level_is_raised() {
    // `say -v Whisper`, band-limited to 300–3400 Hz, peaking at -42 dBFS over
    // a -84 dBFS noise floor (MIC-09).
    let engine = engine();
    let clip = clip("whispered.wav");
    let peak = clip
        .samples()
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak < 0.01, "the fixture is quiet: peak {peak}");

    let as_is = engine.transcribe(&clip, &english()).await;
    println!("without the level raised: {as_is:?}");

    let started = Instant::now();
    let text = transcribe(&engine, &clip, &english()).await.unwrap().text;
    println!("raised: {text:?} in {:?}", started.elapsed());
    let words = words(&text);
    for expected in ["move the design review", "thursday afternoon", "whole team"] {
        assert!(
            words.contains(expected),
            "{expected:?} missing from {text:?}"
        );
    }
}

fn languages(codes: &[&str]) -> SttOptions {
    SttOptions {
        languages: codes.iter().map(|code| (*code).to_owned()).collect(),
        ..SttOptions::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn several_languages_are_told_apart_among_the_selected_ones() {
    // `say -v Thomas` and `say -v Paulina` (LANG-02).
    let engine = engine();
    let options = languages(&["en", "fr", "es"]);
    for (fixture, language, expected) in [
        ("french.wav", "fr", "rapport"),
        ("spanish.wav", "es", "informe"),
        ("hello-shrimp.wav", "en", "local dictation"),
    ] {
        let started = Instant::now();
        let transcript = transcribe(&engine, &clip(fixture), &options).await.unwrap();
        println!(
            "{fixture}: {:?} as {:?} in {:?}",
            transcript.text,
            transcript.language,
            started.elapsed()
        );
        assert_eq!(transcript.language.as_deref(), Some(language));
        assert!(words(&transcript.text).contains(expected), "{transcript:?}");
    }
    let detected = engine.detect_language(&clip("french.wav")).await.unwrap();
    println!("language check: {detected:?}");
    assert_eq!(detected.0, "fr");
    assert!(detected.1 > 0.75);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs downloaded Whisper and VAD models"]
async fn chinese_variants_steer_the_script() {
    // `say -v Meijia` reading Traditional text and `say -v Sinji` Cantonese (LANG-03).
    let engine = engine();
    let clip = clip("chinese.wav");
    let hinted = |id: &str| SttOptions {
        languages: vec!["zh".into()],
        hints: vec![(
            "zh".into(),
            shrimp_lib::lang::whisper_hint(id).unwrap().into(),
        )],
        ..SttOptions::default()
    };
    let shaped = |text: String, id: &str| shrimp_lib::lang::shape(&text, Some(id));
    let traditional = shaped(
        transcribe(&engine, &clip, &hinted("zh-Hant"))
            .await
            .unwrap()
            .text,
        "zh-Hant",
    );
    let simplified = shaped(
        transcribe(&engine, &clip, &hinted("zh-Hans"))
            .await
            .unwrap()
            .text,
        "zh-Hans",
    );
    println!("Traditional: {traditional:?}\nSimplified: {simplified:?}");
    assert!(
        traditional.contains('們') || traditional.contains('會'),
        "{traditional}"
    );
    assert!(
        simplified.contains('们') || simplified.contains('会'),
        "{simplified}"
    );

    let cantonese = SttOptions {
        languages: vec!["yue".into()],
        hints: vec![(
            "yue".into(),
            shrimp_lib::lang::whisper_hint("yue").unwrap().into(),
        )],
        ..SttOptions::default()
    };
    let text = shaped(
        transcribe(&engine, &self::clip("cantonese.wav"), &cantonese)
            .await
            .unwrap()
            .text,
        "yue",
    );
    println!("Cantonese: {text:?}");
    // Whisper writes Cantonese speech as standard written Chinese; Shrimp
    // gives it Hong Kong's Traditional characters.
    assert!(text.contains('會') && !text.contains('会'), "{text}");
}
