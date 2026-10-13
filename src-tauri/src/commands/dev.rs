//! Debug-only commands for agents driving the dev app through tauri-mcp.

use tauri::State;

use super::CommandError;
use crate::audio::AudioSourceSpec;
use crate::dictation::ControllerHandle;

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
