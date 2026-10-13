//! Shortcuts that are not dictation gestures: Press Enter (CORE-15), the
//! Command Mode shortcut while it is off (CMD-06), and which shortcuts are
//! bound at each moment.

use std::sync::Arc;

use handy_keys::Modifiers;

use super::Controller;
use crate::dictation::bindings::{self, Moment};
use crate::hotkeys::Action;
use crate::notify::{Notice, NoticeAction};
use crate::platform::SyntheticKey;
use crate::t;

/// "Command mode is toggled off" shows at most this often (CMD-06).
const COMMAND_OFF_NOTICES: u32 = 3;
const SECONDS_PER_DAY: u64 = 86_400;

impl Controller {
    /// The Press Enter shortcut (CORE-15); extra modifiers pass through.
    pub(super) fn press_enter(&self, extra: Modifiers) {
        let inserter = Arc::clone(&self.inserter);
        let ui = Arc::clone(&self.ui);
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) = inserter.press(SyntheticKey::Enter, extra) {
                ui.notice(Notice::error(t!("notice.pressEnterFailed", error = error)));
            }
        });
    }

    /// The Command Mode shortcut while it is off (CMD-06): a notice with a
    /// way to the setting, at most once a day and three times in all.
    pub(super) fn command_mode_off(&self) {
        if self.recording.is_some() || self.gestures.is_busy() {
            return;
        }
        let settings = self.settings.get();
        let today = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
            / SECONDS_PER_DAY)
            .to_string();
        let notices = &settings.notices;
        if notices.command_mode_off_count >= COMMAND_OFF_NOTICES
            || notices.command_mode_off_day == today
        {
            return;
        }
        let patch = serde_json::json!({"notices": {
            "commandModeOffCount": notices.command_mode_off_count + 1,
            "commandModeOffDay": today,
        }});
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not count the notice: {error}");
        }
        self.ui.notice(
            Notice::info(t!("notice.command.off"))
                .with_action(NoticeAction::OpenExperimentalSettings),
        );
    }

    /// Esc (and Backspace in Command Mode) is only swallowed while a session
    /// records (CORE-06, CMD-04), and until its key-up, so no half of the
    /// press reaches the app. The user's own shortcuts come from the settings.
    /// A held dictation's shortcut is never released as stuck (CORE-14).
    pub(super) fn refresh_bindings(&mut self) {
        let moment = Moment {
            cancelable: self.recording.is_some() || self.cancel_held,
            command: self.gestures.is_command(),
        };
        let bindings = bindings::active(&self.bindings, &self.settings.get(), moment);
        self.hotkeys.set_bindings(&bindings);
        let protected: &[Action] = if self.recording.is_some() {
            &[Action::PushToTalk, Action::CommandMode]
        } else {
            &[]
        };
        self.hotkeys.set_protected(protected);
    }
}
