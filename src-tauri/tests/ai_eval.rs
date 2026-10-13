//! The AI edit eval: dictation-style cases (fillers, self-corrections, lists,
//! spoken punctuation, questions and injection attempts, code, dictionary
//! terms, snippets, styles) run through the real edit pipeline and a real
//! provider, with cleanup latency per case. Opt-in, because it calls a model:
//!
//! ```sh
//! SHRIMP_EVAL_PROVIDER=ollama SHRIMP_EVAL_MODEL=gemma3:4b \
//!   cargo test --test ai_eval -- --ignored --nocapture
//! ```
//!
//! Cloud providers read their key from ANTHROPIC_API_KEY, OPENAI_API_KEY or
//! GROQ_API_KEY. SHRIMP_EVAL_RUNS repeats each case (latency is the median);
//! SHRIMP_EVAL_STRICT=1 fails the test on any failed check.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use shrimp_lib::context::DictationContext;
use shrimp_lib::edit::{CleanupLevel, EditTimeout, Edited, Editor, FixedLibrary, Snapshot, Style};
use shrimp_lib::llm::{LlmProviderId, Providers, http};
use shrimp_lib::platform::AppIdentity;
use shrimp_lib::secrets::{KeyAccount, MemorySecrets, Secrets};
use shrimp_lib::settings::{Settings, SettingsStore};
use shrimp_lib::storage::{DictionaryEntry, EntrySource, Snippet};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    id: String,
    input: String,
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    level: Option<CleanupLevel>,
    #[serde(default)]
    styles: HashMap<String, Style>,
    #[serde(default)]
    dictionary: Vec<Word>,
    #[serde(default)]
    snippets: Vec<SnippetCase>,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
struct Word {
    term: String,
    #[serde(default)]
    misspelling: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SnippetCase {
    trigger: String,
    expansion: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Expect {
    equals: Option<String>,
    contains: Vec<String>,
    contains_ignore_case: Vec<String>,
    /// Each group needs at least one of its strings.
    any_of: Vec<Vec<String>>,
    not_contains: Vec<String>,
    not_contains_words: Vec<String>,
    starts_with: Option<String>,
    ends_with: Option<String>,
    not_ends_with: Option<String>,
    min_words: Option<usize>,
    max_words: Option<usize>,
    min_lines: Option<usize>,
    max_lines: Option<usize>,
    html: Option<bool>,
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn check(expect: &Expect, edited: &Edited) -> Vec<String> {
    let text = edited.text.plain.as_str();
    let lower = text.to_lowercase();
    let found = words(text);
    let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
    let mut failures = Vec::new();
    let mut fail = |ok: bool, what: String| {
        if !ok {
            failures.push(what);
        }
    };
    if let Some(equals) = &expect.equals {
        fail(text == equals, format!("equals {equals:?}"));
    }
    for needle in &expect.contains {
        fail(
            text.contains(needle.as_str()),
            format!("contains {needle:?}"),
        );
    }
    for needle in &expect.contains_ignore_case {
        fail(
            lower.contains(&needle.to_lowercase()),
            format!("contains {needle:?} (any case)"),
        );
    }
    for group in &expect.any_of {
        let ok = group.iter().any(|needle| text.contains(needle.as_str()));
        fail(ok, format!("contains one of {group:?}"));
    }
    for needle in &expect.not_contains {
        fail(!text.contains(needle.as_str()), format!("lacks {needle:?}"));
    }
    for word in &expect.not_contains_words {
        fail(
            !found.contains(&word.to_lowercase()),
            format!("lacks the word {word:?}"),
        );
    }
    if let Some(prefix) = &expect.starts_with {
        fail(
            text.starts_with(prefix.as_str()),
            format!("starts with {prefix:?}"),
        );
    }
    if let Some(suffix) = &expect.ends_with {
        fail(
            text.ends_with(suffix.as_str()),
            format!("ends with {suffix:?}"),
        );
    }
    if let Some(suffix) = &expect.not_ends_with {
        fail(
            !text.ends_with(suffix.as_str()),
            format!("doesn't end with {suffix:?}"),
        );
    }
    if let Some(min) = expect.min_words {
        fail(found.len() >= min, format!("at least {min} words"));
    }
    if let Some(max) = expect.max_words {
        fail(found.len() <= max, format!("at most {max} words"));
    }
    if let Some(min) = expect.min_lines {
        fail(lines >= min, format!("at least {min} lines"));
    }
    if let Some(max) = expect.max_lines {
        fail(lines <= max, format!("at most {max} lines"));
    }
    if let Some(html) = expect.html {
        fail(
            edited.text.html.is_some() == html,
            format!("rich text {html}"),
        );
    }
    failures
}

fn snapshot(case: &Case) -> Snapshot {
    let dictionary = case
        .dictionary
        .iter()
        .enumerate()
        .map(|(index, word)| DictionaryEntry {
            id: index as i64 + 1,
            term: word.term.clone(),
            misspelling: word.misspelling.clone(),
            source: EntrySource::Manual,
            pinned: false,
            uses: 0,
            created_at: 0,
        })
        .collect();
    let snippets = case
        .snippets
        .iter()
        .enumerate()
        .map(|(index, snippet)| Snippet {
            id: index as i64 + 1,
            trigger: snippet.trigger.clone(),
            expansion: snippet.expansion.clone(),
            created_at: 0,
            updated_at: 0,
        })
        .collect();
    Snapshot {
        dictionary,
        snippets,
    }
}

fn settings(provider: LlmProviderId, model: &str, case: &Case) -> Settings {
    let mut settings = Settings::default();
    settings.ai.provider = provider;
    settings.ai.cleanup = case.level.unwrap_or(CleanupLevel::Light);
    match provider {
        LlmProviderId::Anthropic => settings.ai.anthropic_model = model.into(),
        LlmProviderId::OpenAi => settings.ai.openai_model = model.into(),
        LlmProviderId::Groq => settings.ai.groq_model = model.into(),
        LlmProviderId::Ollama => settings.ai.ollama_model = model.into(),
    }
    for (category, style) in &case.styles {
        match category.as_str() {
            "personal" => settings.styles.personal = *style,
            "work" => settings.styles.work = *style,
            "email" => settings.styles.email = *style,
            _ => settings.styles.other = *style,
        }
    }
    settings
}

fn provider_from_env() -> (LlmProviderId, String) {
    let provider = match std::env::var("SHRIMP_EVAL_PROVIDER").as_deref() {
        Ok("anthropic") => LlmProviderId::Anthropic,
        Ok("openai") => LlmProviderId::OpenAi,
        Ok("groq") => LlmProviderId::Groq,
        _ => LlmProviderId::Ollama,
    };
    let model = std::env::var("SHRIMP_EVAL_MODEL").unwrap_or_default();
    (provider, model)
}

fn secrets() -> Arc<Secrets> {
    let keys = [
        (KeyAccount::Anthropic, "ANTHROPIC_API_KEY"),
        (KeyAccount::OpenAi, "OPENAI_API_KEY"),
        (KeyAccount::Groq, "GROQ_API_KEY"),
    ]
    .into_iter()
    .filter_map(|(account, variable)| Some((account, std::env::var(variable).ok()?)));
    Arc::new(Secrets::new(Box::new(MemorySecrets::with(keys))))
}

/// Prints the pipeline's own log lines (why an edit was not used).
struct PrintLogger;

impl log::Log for PrintLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Debug && metadata.target().starts_with("shrimp_lib::edit")
    }

    fn log(&self, record: &log::Record<'_>) {
        let message = record.args().to_string();
        if self.enabled(record.metadata()) && !message.starts_with("AI edit by") {
            println!("    log: {message}");
        }
    }

    fn flush(&self) {}
}

fn median(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "calls a real LLM provider; see the module docs"]
async fn ai_edits_pass_the_eval_set() {
    let _ = log::set_logger(&PrintLogger).map(|()| log::set_max_level(log::LevelFilter::Debug));
    let (provider, model) = provider_from_env();
    let runs: usize = std::env::var("SHRIMP_EVAL_RUNS")
        .ok()
        .and_then(|runs| runs.parse().ok())
        .unwrap_or(1);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval/cases.json");
    let mut cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    if let Ok(only) = std::env::var("SHRIMP_EVAL_CASE") {
        cases.retain(|case| only.split(',').any(|id| id == case.id));
    }
    let client = http::client();
    let secrets = secrets();
    let timeout = EditTimeout {
        base: Duration::from_secs(30),
        per_word: Duration::ZERO,
        max: Duration::from_secs(30),
    };

    println!("\n{} {model}, {runs} run(s) per case\n", provider.label());
    println!("| case | ms | result | output |");
    println!("| --- | ---: | --- | --- |");
    let mut all_latencies = Vec::new();
    let mut failed = Vec::new();
    let mut fallbacks = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let editor = Editor::new(
            Arc::new(SettingsStore::in_memory(settings(provider, &model, case))),
            Arc::new(Providers::new(client.clone(), Arc::clone(&secrets))),
            Arc::new(FixedLibrary(snapshot(case))),
            timeout,
        );
        let app = AppIdentity {
            bundle_id: case
                .app
                .clone()
                .unwrap_or_else(|| "com.apple.TextEdit".into()),
            name: String::new(),
        };
        if index == 0 {
            // Loads a local model and opens connections; not measured.
            editor.warm_up().await;
            let _ = editor
                .edit("warm up", &DictationContext::new(Some(app.clone()), None))
                .await;
        }
        let mut latencies = Vec::new();
        let mut last = None;
        for _ in 0..runs {
            let edited = editor
                .edit(&case.input, &DictationContext::new(Some(app.clone()), None))
                .await;
            latencies.extend(edited.llm_ms);
            last = Some(edited);
        }
        let edited = last.unwrap();
        let failures = check(&case.expect, &edited);
        // A rejected or failed edit inserts the unedited transcript: safe,
        // but not a cleanup.
        let fell_back = !edited.formatted && case.expect.equals.is_none();
        let latency = median(&mut latencies);
        all_latencies.extend(latencies);
        let result = if fell_back {
            fallbacks.push(case.id.clone());
            "raw fallback".to_owned()
        } else if failures.is_empty() {
            "pass".to_owned()
        } else {
            failed.push(case.id.clone());
            failures.join("; ")
        };
        println!(
            "| {} | {latency} | {result} | {:?} |",
            case.id,
            edited.text.plain.replace('\n', "⏎")
        );
    }
    let p90 = {
        all_latencies.sort_unstable();
        all_latencies
            .get(all_latencies.len() * 9 / 10)
            .copied()
            .unwrap_or_default()
    };
    println!(
        "\n{} {model}: {}/{} passed, {} raw fallback ({}), {} failed ({}); cleanup latency median {} ms, p90 {p90} ms",
        provider.label(),
        cases.len() - failed.len() - fallbacks.len(),
        cases.len(),
        fallbacks.len(),
        fallbacks.join(", "),
        failed.len(),
        failed.join(", "),
        median(&mut all_latencies.clone())
    );
    if std::env::var("SHRIMP_EVAL_STRICT").as_deref() == Ok("1") {
        assert!(failed.is_empty(), "failed: {failed:?}");
    }
}
