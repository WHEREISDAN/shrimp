//! Local speech-to-text with whisper.cpp (Metal on Apple Silicon). The model
//! stays loaded; each dictation runs on a fresh `WhisperState`. Silero VAD
//! runs first: no speech means no Whisper call, and the audio is cut to the
//! speech so Whisper never sees long silences (ENG-03).

use std::path::Path;
use std::sync::{Arc, Mutex, Once};

use async_trait::async_trait;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperVadContext,
    WhisperVadContextParams, WhisperVadParams,
};

use super::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};
use crate::audio::{AudioClip, STT_SAMPLE_RATE};

/// whisper.cpp skips input shorter than one second, so short clips are padded.
const MIN_SAMPLES: usize = STT_SAMPLE_RATE as usize * 6 / 5;
const MAX_THREADS: usize = 8;
/// Silence kept around detected speech.
const VAD_PADDING_SAMPLES: usize = STT_SAMPLE_RATE as usize / 4;
/// Whisper's encoder covers 30 s as 1500 frames (50 per second).
const FULL_AUDIO_CTX: usize = 1500;
const AUDIO_CTX_PER_SECOND: usize = 50;
/// Extra encoder frames beyond the audio. Large v3 Turbo repeats itself on
/// short clips encoded with much less context than this floor.
const AUDIO_CTX_MARGIN: usize = 256;
const MIN_AUDIO_CTX: usize = 768;
/// Non-speech markers Whisper sometimes emits instead of words.
const NON_SPEECH: [&str; 6] = [
    "[BLANK_AUDIO]",
    "[ Silence ]",
    "[silence]",
    "(silence)",
    "[MUSIC]",
    "[NO SPEECH]",
];

pub struct LocalWhisper {
    context: Arc<WhisperContext>,
    vad: Option<Arc<Mutex<WhisperVadContext>>>,
}

impl LocalWhisper {
    /// Loads the model once (a second or two for the default model).
    pub fn load(model: &Path, vad_model: Option<&Path>) -> Result<Self, SttError> {
        static LOGGING: Once = Once::new();
        // Keeps whisper.cpp and ggml from printing to stdout.
        LOGGING.call_once(whisper_rs::install_logging_hooks);

        let mut params = WhisperContextParameters::default();
        params.flash_attn(true);
        let context = WhisperContext::new_with_params(utf8(model)?, params)
            .map_err(|err| SttError::Unavailable(format!("could not load the model: {err}")))?;
        let vad = vad_model
            .map(|path| {
                let mut params = WhisperVadContextParams::new();
                params.set_use_gpu(false);
                WhisperVadContext::new(utf8(path)?, params).map_err(|err| {
                    SttError::Unavailable(format!("could not load speech detection: {err}"))
                })
            })
            .transpose()?;
        Ok(Self {
            context: Arc::new(context),
            vad: vad.map(|vad| Arc::new(Mutex::new(vad))),
        })
    }
}

#[async_trait]
impl SttEngine for LocalWhisper {
    fn id(&self) -> SttEngineId {
        SttEngineId::LocalWhisper
    }

    async fn transcribe(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
    ) -> Result<Transcript, SttError> {
        let samples = clip.samples().to_vec();
        let context = Arc::clone(&self.context);
        let vad = self.vad.clone();
        let options = options.clone();
        tokio::task::spawn_blocking(move || {
            let speech = match vad {
                Some(vad) => speech_only(&vad, samples)?,
                None => samples,
            };
            run(&context, speech, &options)
        })
        .await
        .map_err(|err| SttError::Failed(format!("transcription task failed: {err}")))?
    }
}

fn utf8(path: &Path) -> Result<&str, SttError> {
    path.to_str()
        .ok_or_else(|| SttError::Unavailable(format!("{} is not a UTF-8 path", path.display())))
}

/// Cuts the clip to the span from the first to the last detected speech.
fn speech_only(vad: &Mutex<WhisperVadContext>, samples: Vec<f32>) -> Result<Vec<f32>, SttError> {
    let mut vad = vad.lock().unwrap_or_else(|err| err.into_inner());
    let segments = vad
        .segments_from_samples(WhisperVadParams::new(), &samples)
        .map_err(|err| SttError::Failed(format!("speech detection failed: {err}")))?;
    let count = segments.num_segments();
    if count == 0 {
        return Err(SttError::NoSpeech);
    }
    // Timestamps are in centiseconds.
    let to_sample = |centiseconds: f32| (centiseconds / 100.0 * STT_SAMPLE_RATE as f32) as usize;
    let start = segments.get_segment_start_timestamp(0).map_or(0, to_sample);
    let end = segments
        .get_segment_end_timestamp(count - 1)
        .map_or(samples.len(), to_sample);
    let start = start.saturating_sub(VAD_PADDING_SAMPLES);
    let end = (end + VAD_PADDING_SAMPLES).min(samples.len());
    Ok(samples[start..end.max(start)].to_vec())
}

/// Encoder context sized to the clip: a short dictation encodes far less
/// than Whisper's fixed 30 s window, which is most of the latency. Language
/// detection needs the full window (0), or it misreads the language.
fn audio_ctx(samples: usize, language_known: bool) -> usize {
    let frames = samples.div_ceil(STT_SAMPLE_RATE as usize / AUDIO_CTX_PER_SECOND);
    let ctx = (frames + AUDIO_CTX_MARGIN).max(MIN_AUDIO_CTX);
    if !language_known || ctx >= FULL_AUDIO_CTX {
        0
    } else {
        ctx
    }
}

fn run(
    context: &WhisperContext,
    mut samples: Vec<f32>,
    options: &SttOptions,
) -> Result<Transcript, SttError> {
    if samples.len() < MIN_SAMPLES {
        samples.resize(MIN_SAMPLES, 0.0);
    }
    let failed = |err: whisper_rs::WhisperError| SttError::Failed(err.to_string());
    let mut state = context.create_state().map_err(failed)?;

    let language = match options.languages.as_slice() {
        [only] if whisper_rs::get_lang_id(only).is_some() => only.as_str(),
        _ => "auto",
    };
    let prompt = vocabulary_prompt(&options.vocabulary);
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    params.set_n_threads(threads());
    params.set_audio_ctx(audio_ctx(samples.len(), language != "auto") as i32);
    params.set_no_context(true);
    params.set_no_timestamps(true);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    if !prompt.is_empty() {
        params.set_initial_prompt(&prompt);
    }

    state.full(params, &samples).map_err(failed)?;
    let raw: String = state
        .as_iter()
        .filter_map(|segment| segment.to_str_lossy().ok().map(|text| text.into_owned()))
        .collect();
    let text = clean(&raw);
    if text.is_empty() {
        return Err(SttError::NoSpeech);
    }
    let language = whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_owned);
    Ok(Transcript { text, language })
}

/// The dictionary terms as Whisper's initial prompt. Whisper copies the
/// prompt's style, so it ends with a period: without one, transcripts lose
/// their sentence punctuation.
pub fn vocabulary_prompt(terms: &[String]) -> String {
    if terms.is_empty() {
        return String::new();
    }
    let mut prompt = terms.join(", ");
    if !prompt.ends_with(['.', '!', '?']) {
        prompt.push('.');
    }
    prompt
}

fn threads() -> i32 {
    std::thread::available_parallelism().map_or(4, |count| count.get().min(MAX_THREADS)) as i32
}

/// Drops non-speech markers and joins segments with single spaces.
fn clean(raw: &str) -> String {
    let without_markers = NON_SPEECH
        .iter()
        .fold(raw.to_owned(), |text, marker| text.replace(marker, " "));
    without_markers
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_ctx_covers_the_clip_plus_a_margin() {
        assert_eq!(audio_ctx(16_000 * 2, true), MIN_AUDIO_CTX);
        assert_eq!(audio_ctx(16_000 * 12, true), 600 + AUDIO_CTX_MARGIN);
        assert_eq!(
            audio_ctx(16_000 * 26, true),
            0,
            "long clips use the full window"
        );
        assert_eq!(
            audio_ctx(16_000 * 2, false),
            0,
            "detection needs the full window"
        );
    }

    #[test]
    fn the_vocabulary_prompt_ends_like_a_sentence() {
        assert_eq!(vocabulary_prompt(&[]), "");
        let terms = vec!["Nguyen".to_owned(), "Tauri".to_owned()];
        assert_eq!(vocabulary_prompt(&terms), "Nguyen, Tauri.");
    }

    #[test]
    fn clean_removes_markers_and_extra_spaces() {
        assert_eq!(
            clean(" Hello  there.\n How are you?"),
            "Hello there. How are you?"
        );
        assert_eq!(clean(" [BLANK_AUDIO]"), "");
        assert_eq!(clean("Hi [MUSIC] again"), "Hi again");
    }
}
