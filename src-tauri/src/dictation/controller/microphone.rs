//! The source a recording starts from, the microphone's warnings while it
//! records (MIC-07), and what is released when it ends.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::{Controller, lock};
use crate::audio::{AudioError, AudioSource, AudioSourceSpec, InputDevice};
use crate::dictation::state::DictationUi;
use crate::mic::{MicChange, MicMonitor, MicWarning};
use crate::notify::{Notice, NoticeAction};

/// The microphone a recording uses, its monitor and the notices it shows.
pub(super) struct MicSession {
    device: InputDevice,
    monitor: MicMonitor,
    notices: HashMap<MicWarning, u64>,
}

impl MicSession {
    pub(super) fn new(device: InputDevice) -> Self {
        Self {
            device,
            monitor: MicMonitor::new(Instant::now()),
            notices: HashMap::new(),
        }
    }

    pub(super) fn update(&mut self, open: bool, rms: Option<f32>, ui: &dyn DictationUi) {
        for change in self.monitor.update(Instant::now(), open, rms) {
            match change {
                MicChange::Show(warning) => {
                    let notice = warning_notice(warning, &self.device.name);
                    self.notices.insert(warning, notice.id);
                    ui.notice(notice);
                }
                MicChange::Clear(warning) => {
                    if let Some(id) = self.notices.remove(&warning) {
                        ui.dismiss(id);
                    }
                }
            }
        }
    }
}

fn warning_notice(warning: MicWarning, name: &str) -> Notice {
    match warning {
        MicWarning::SlowToOpen => Notice::warning(format!("{name} is taking a long time to open."))
            .with_action(NoticeAction::ChooseMicrophone),
        MicWarning::Muted => Notice::warning(format!(
            "Is your microphone muted? Shrimp can't hear anything from {name}."
        ))
        .with_action(NoticeAction::ChooseMicrophone)
        .with_action(NoticeAction::TroubleshootMicrophone),
        MicWarning::NoAudio => Notice::warning(format!("No audio from {name} for 5 seconds."))
            .with_action(NoticeAction::ChooseMicrophone),
        MicWarning::StillNoAudio => Notice::error(format!(
            "Still no audio from {name}. Is it muted or out of range?"
        ))
        .with_action(NoticeAction::ChooseMicrophone)
        .with_action(NoticeAction::TroubleshootMicrophone),
    }
}

type Opened = (Box<dyn AudioSource>, Option<InputDevice>);

impl Controller {
    /// The source for a recording: a WAV file or named microphone from the
    /// audio source setting, or the microphone the user's choice and ranking
    /// pick. `None` when it can't start yet (no permission, nothing to
    /// record from); the notice has been shown.
    pub(super) fn open_source(&mut self) -> Result<Option<Opened>, AudioError> {
        let spec = lock(&self.source).clone();
        let chooser = match (&spec, &self.microphones) {
            (AudioSourceSpec::Microphone { device: None }, Some(microphones)) => {
                Some(Arc::clone(microphones))
            }
            _ => None,
        };
        if let Some(microphones) = chooser {
            let Ok(opened) = microphones.open() else {
                return Ok(None);
            };
            if !self.microphone_allowed() {
                return Ok(None);
            }
            return Ok(Some((opened.source, Some(opened.device))));
        }
        spec.check_available()?;
        if spec.is_microphone() && !self.microphone_allowed() {
            return Ok(None);
        }
        Ok(Some((spec.open(), None)))
    }

    /// The recording ends: restore muted or paused audio, before the stop
    /// cue, and let a waiting microphone prompt show.
    pub(super) fn release_devices(&mut self) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        if let Some(media) = recording.media.take() {
            media.end();
        }
        if let (true, Some(microphones)) = (recording.microphone.is_some(), &self.microphones) {
            microphones.recording_ended();
        }
    }
}
