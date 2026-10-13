//! Debug-only commands for agents driving the dev app through tauri-mcp.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::CommandError;
use crate::audio::AudioSourceSpec;
use crate::context::DictationContext;
use crate::dictation::ControllerHandle;
use crate::edit::{AppCategory, Editor, Target, style};
use crate::platform::{AppIdentity, FocusInspector};

/// The current audio source, as `mic`, `mic:<device>` or `wav:<path>`.
#[tauri::command]
pub fn dev_get_audio_source(controller: State<'_, ControllerHandle>) -> String {
    controller.audio_source().to_string()
}

/// Switches the audio source for the next dictation, e.g. `wav:/abs/path.wav`.
#[tauri::command]
pub fn dev_set_audio_source(
    spec: String,
    controller: State<'_, ControllerHandle>,
) -> Result<String, CommandError> {
    let spec: AudioSourceSpec = spec
        .parse()
        .map_err(|err| CommandError::new("invalidSpec", err))?;
    if let AudioSourceSpec::WavFile(path) = &spec
        && !path.is_file()
    {
        return Err(CommandError::new(
            "notFound",
            format!("{} does not exist", path.display()),
        ));
    }
    controller.set_audio_source(spec.clone());
    Ok(spec.to_string())
}

/// A category forced for one app in end-to-end tests; `ai` marks an AI app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevCategory {
    Personal,
    Work,
    Email,
    Other,
    Ai,
}

/// Treats `app_id` (a bundle ID) as `category` until reset with `null`.
#[tauri::command]
pub fn dev_set_app_category(app_id: String, category: Option<DevCategory>) {
    let target = category.map(|category| match category {
        DevCategory::Ai => Target {
            category: AppCategory::Other,
            ai_app: true,
        },
        other => Target {
            category: match other {
                DevCategory::Personal => AppCategory::Personal,
                DevCategory::Work => AppCategory::Work,
                DevCategory::Email => AppCategory::Email,
                _ => AppCategory::Other,
            },
            ai_app: false,
        },
    });
    style::set_override(&app_id, target);
}

/// The edit pipeline's result for a transcript, without STT or pasting.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevEdited {
    pub plain: String,
    pub html: Option<String>,
    pub formatted: bool,
    pub notice: Option<String>,
    pub llm_ms: Option<u64>,
}

#[tauri::command]
pub async fn dev_edit_text(
    text: String,
    app_id: Option<String>,
    editor: State<'_, Arc<Editor>>,
) -> Result<DevEdited, CommandError> {
    let app = app_id.map(|bundle_id| AppIdentity {
        bundle_id,
        name: String::new(),
    });
    let edited = editor.edit(&text, &DictationContext::new(app, None)).await;
    Ok(DevEdited {
        plain: edited.text.plain,
        html: edited.text.html,
        formatted: edited.formatted,
        notice: edited.notice.map(|notice| notice.message),
        llm_ms: edited.llm_ms,
    })
}

/// The focus inspector, kept for debug commands.
pub struct DevFocus(pub Arc<dyn FocusInspector>);

/// What context capture reads from the frontmost app, shortened for display.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevContext {
    pub app: Option<String>,
    pub app_kind: String,
    pub field_kind: Option<String>,
    pub readable: bool,
    pub selection: Option<(usize, usize)>,
    pub before: String,
    pub after: String,
    pub selected: String,
    pub url: Option<String>,
    pub category: String,
    pub ai_app: bool,
    pub window_title: Option<String>,
    pub names: Vec<String>,
    pub identifiers: Vec<String>,
    pub file_names: Vec<String>,
    pub screen_texts: usize,
    pub read_ms: u64,
}

fn tail(text: &str, chars: usize) -> String {
    let count = text.chars().count();
    text.chars().skip(count.saturating_sub(chars)).collect()
}

/// Reads the frontmost app's field after `delay_ms` (to switch apps first).
#[tauri::command]
pub async fn dev_read_context(
    delay_ms: Option<u64>,
    focus: State<'_, DevFocus>,
) -> Result<DevContext, CommandError> {
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms.unwrap_or(0))).await;
    let focus = Arc::clone(&focus.0);
    let started = std::time::Instant::now();
    let app = focus.frontmost_app();
    let mut pending = crate::context::capture(focus, app);
    let context = loop {
        let context = pending.ready();
        if context.field.is_some() || started.elapsed().as_secs() >= 3 {
            break context;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    let read_ms = started.elapsed().as_millis() as u64;
    let target = context.target();
    let field = context.field.as_ref();
    Ok(DevContext {
        app: context
            .app
            .as_ref()
            .map(|app| format!("{} ({})", app.name, app.bundle_id)),
        app_kind: format!("{:?}", context.app_kind()),
        field_kind: field.map(|_| format!("{:?}", context.field_kind())),
        readable: context.readable_field().is_some(),
        selection: field
            .and_then(|field| field.selection)
            .map(|range| (range.start, range.len)),
        before: context
            .surroundings()
            .map(|around| tail(around.before, 120))
            .unwrap_or_default(),
        after: context
            .surroundings()
            .map(|around| around.after.chars().take(120).collect())
            .unwrap_or_default(),
        selected: context
            .surroundings()
            .map(|around| around.selected.chars().take(120).collect())
            .unwrap_or_default(),
        url: field.and_then(|field| field.url.clone()),
        category: format!("{:?}", target.category),
        ai_app: target.ai_app,
        window_title: field.and_then(|field| field.window_title.clone()),
        names: context.names(),
        identifiers: context.identifiers().into_iter().take(20).collect(),
        file_names: context.file_names().into_iter().take(20).collect(),
        screen_texts: field.map_or(0, |field| field.screen_text.len()),
        read_ms,
    })
}
