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
            text = transcribe(&engine, &clip, &options).await.unwrap();
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

    let text = transcribe(&engine, &padded, &english()).await.unwrap();
    assert!(words(&text).contains("hello from shrimp"), "{text}");
}
