//! The Command Mode and transforms eval: instructions on selections and
//! documents (edits, answers, injection attempts in the text) and the
//! built-in transforms, through the real prompts and a real provider, with
//! latency per case. Opt-in, because it calls a model:
//!
//! ```sh
//! SHRIMP_EVAL_PROVIDER=ollama SHRIMP_EVAL_MODEL=qwen3.5:4b \
//!   cargo test --test command_eval -- --ignored --nocapture
//! ```
//!
//! The same variables as `ai_eval` apply, except `SHRIMP_EVAL_RUNS`.

mod eval_support;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use eval_support::{median, p90, provider_from_env, secrets, settings_for};
use serde::Deserialize;
use shrimp_lib::edit::command::{CommandInput, CommandReply, drops_text};
use shrimp_lib::edit::transforms::Transform;
use shrimp_lib::edit::{EditTimeout, Editor, FixedLibrary};
use shrimp_lib::llm::{Providers, http};
use shrimp_lib::settings::{CustomTransform, SettingsStore};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    id: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    selection: Option<String>,
    #[serde(default)]
    document: Option<String>,
    #[serde(default)]
    transform: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    expect: Expect,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Expect {
    kind: Option<String>,
    any_kind: bool,
    contains: Vec<String>,
    contains_ignore_case: Vec<String>,
    any_of: Vec<Vec<String>>,
    not_contains: Vec<String>,
    not_contains_words: Vec<String>,
    min_words: Option<usize>,
    max_words: Option<usize>,
    min_lines: Option<usize>,
    /// `[a, b]`: the text may hold `a` only next to `b`, as when an injected
    /// instruction is kept as content rather than obeyed.
    only_with: Vec<[String; 2]>,
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn check(expect: &Expect, kind: Option<&str>, text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let found = words(text);
    let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
    let mut failures = Vec::new();
    let mut fail = |ok: bool, what: String| {
        if !ok {
            failures.push(what);
        }
    };
    if let (Some(wanted), Some(kind)) = (&expect.kind, kind)
        && !expect.any_kind
    {
        fail(wanted == kind, format!("wanted {wanted}, got {kind}"));
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
    if let Some(min) = expect.min_words {
        fail(found.len() >= min, format!("at least {min} words"));
    }
    if let Some(max) = expect.max_words {
        fail(found.len() <= max, format!("at most {max} words"));
    }
    if let Some(min) = expect.min_lines {
        fail(lines >= min, format!("at least {min} lines"));
    }
    for [needle, company] in &expect.only_with {
        let ok = !lower.contains(&needle.to_lowercase()) || lower.contains(&company.to_lowercase());
        fail(ok, format!("holds {needle:?} only with {company:?}"));
    }
    failures
}

/// The case's output, its kind (edit, answer, or "withheld": an edit the app
/// shows instead of pasting because it drops most of the text) and the
/// model's time.
async fn run(
    editor: &Editor,
    settings: &shrimp_lib::settings::Settings,
    case: &Case,
) -> Result<(String, Option<&'static str>, u64), String> {
    if let Some(instruction) = &case.command {
        let input = match (&case.selection, &case.document) {
            (Some(text), _) => CommandInput::Selection(text),
            (None, Some(text)) => CommandInput::Document(text),
            (None, None) => CommandInput::Nothing,
        };
        let (reply, ms) = editor
            .command(instruction, input)
            .await
            .map_err(|err| err.to_string())?;
        let original = case.selection.as_deref().or(case.document.as_deref());
        return Ok(match reply {
            CommandReply::Edit(text)
                if original.is_some_and(|original| drops_text(instruction, original, &text)) =>
            {
                (text, Some("withheld"), ms)
            }
            CommandReply::Edit(text) => (text, Some("edit"), ms),
            CommandReply::Answer(text) => (text, Some("answer"), ms),
        });
    }
    let transform = match case.transform.as_deref() {
        Some("polish") => Transform::Polish(settings.transforms.polish.clone()),
        Some("promptEngineer") => {
            Transform::PromptEngineer(settings.transforms.prompt_engineer.prompt.clone())
        }
        Some("turnToList") => Transform::TurnToList,
        Some("translateToEnglish") => Transform::TranslateToEnglish,
        Some("empathize") => Transform::Empathize,
        _ => Transform::Custom(CustomTransform {
            id: case.id.clone(),
            name: case.id.clone(),
            prompt: case.prompt.clone().unwrap_or_default(),
            shortcut: None,
            samples: Vec::new(),
        }),
    };
    let text = case.text.as_deref().unwrap_or_default();
    let (result, ms) = editor
        .transform(&transform, text, None)
        .await
        .map_err(|err| err.to_string())?;
    Ok((result, None, ms))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "calls a real LLM provider; see the module docs"]
async fn commands_and_transforms_pass_the_eval_set() {
    let (provider, model) = provider_from_env();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval/commands.json");
    let mut cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    if let Ok(only) = std::env::var("SHRIMP_EVAL_CASE") {
        cases.retain(|case| only.split(',').any(|id| id == case.id));
    }
    let settings = settings_for(provider, &model);
    let editor = Editor::new(
        Arc::new(SettingsStore::in_memory(settings.clone())),
        Arc::new(Providers::new(http::client(), secrets())),
        Arc::new(FixedLibrary::default()),
        EditTimeout {
            base: Duration::from_secs(30),
            per_word: Duration::ZERO,
            max: Duration::from_secs(30),
        },
    );
    editor.warm_up().await;
    let _ = editor.command("Say hi.", CommandInput::Nothing).await;

    println!("\n{} {model}\n", provider.label());
    println!("| case | ms | kind | result | output |");
    println!("| --- | ---: | --- | --- | --- |");
    let mut latencies = Vec::new();
    let mut failed = Vec::new();
    for case in &cases {
        let (result, kind, output, ms) = match run(&editor, &settings, case).await {
            Ok((output, kind, ms)) => {
                latencies.push(ms);
                let failures = check(&case.expect, kind, &output);
                let result = if failures.is_empty() {
                    "pass".to_owned()
                } else {
                    failed.push(case.id.clone());
                    failures.join("; ")
                };
                (result, kind.unwrap_or("text"), output, ms)
            }
            Err(error) => {
                failed.push(case.id.clone());
                (format!("error: {error}"), "-", String::new(), 0)
            }
        };
        println!(
            "| {} | {ms} | {kind} | {result} | {:?} |",
            case.id,
            output.replace('\n', "⏎")
        );
    }
    println!(
        "\n{} {model}: {}/{} passed, failed: {}; latency median {} ms, p90 {} ms",
        provider.label(),
        cases.len() - failed.len(),
        cases.len(),
        if failed.is_empty() {
            "none".to_owned()
        } else {
            failed.join(", ")
        },
        median(&mut latencies.clone()),
        p90(&mut latencies)
    );
    if std::env::var("SHRIMP_EVAL_STRICT").as_deref() == Ok("1") {
        assert!(failed.is_empty(), "failed: {failed:?}");
    }
}
