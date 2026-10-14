//! One-time suggestions after a dictation: English (Canada) for people in
//! Canada who dictate in another English (LANG-07), and a mouse button for
//! dictating when a mouse with extra buttons is connected (CORE-18). Each
//! shows once ever; a muted suggestion isn't counted as shown.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;

use super::Controller;
use crate::dictation::bindings;
use crate::dictation::state::{ControllerInput, StartedBy};
use crate::hotkeys::Trigger;
use crate::lang::region;
use crate::notify::{Notice, NoticeAction, NoticeCategory};
use crate::platform::MouseDevices;
use crate::t;

/// Mice with more buttons than this have one to spare: Wispr Flow offers
/// Mouse Flow for "more than three buttons".
const BASIC_MOUSE_BUTTONS: u32 = 3;
/// The connected mice are read at most this often.
const MOUSE_CHECK_EVERY: Duration = Duration::from_secs(10 * 60);

/// What the suggestions need from the system.
pub struct Suggestions {
    /// The system region ("CA").
    region: Option<String>,
    mice: Arc<dyn MouseDevices>,
    mouse_checked_at: Option<Instant>,
    mouse_checking: bool,
}

impl Suggestions {
    pub fn new(region: Option<String>, mice: Arc<dyn MouseDevices>) -> Self {
        Self {
            region,
            mice,
            mouse_checked_at: None,
            mouse_checking: false,
        }
    }
}

impl Controller {
    /// After an inserted dictation: one suggestion at most, once setup is done.
    pub(super) fn suggest(&mut self, started_by: StartedBy) {
        let settings = self.settings.get();
        if !settings.onboarding.completed
            || !NoticeCategory::Suggestion.allowed(&settings.notifications)
        {
            return;
        }
        if self.suggest_canadian_english() {
            return;
        }
        // A dictation started from the Flow Bar says nothing about the keyboard.
        if started_by == StartedBy::Shortcut {
            self.check_mouse();
        }
    }

    fn suggest_canadian_english(&self) -> bool {
        let settings = self.settings.get();
        let region = self.suggestions.region.as_deref();
        if settings.notices.canadian_english_shown
            || !region::suggests_canadian_english(region, &settings.languages)
        {
            return false;
        }
        let patch = json!({"notices": {"canadianEnglishShown": true}});
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not note the Canadian English suggestion: {error}");
        }
        self.ui.notice(
            Notice::info(t!("notice.language.canadianEnglish"))
                .with_action(NoticeAction::SwitchToCanadianEnglish)
                .with_action(NoticeAction::NotNow)
                .in_category(NoticeCategory::Suggestion),
        );
        true
    }

    /// The suggestion's Switch: English (Canada) replaces the other English.
    pub(super) fn switch_to_canadian_english(&self) {
        let Some(languages) = region::with_canadian_english(&self.settings.get().languages) else {
            return;
        };
        if let Err(error) = self.settings.update(&json!({"languages": languages})) {
            log::warn!("could not switch to Canadian English: {error}");
        }
    }

    /// Not shown yet, and no mouse button is bound to anything.
    fn wants_mouse_tip(&self) -> bool {
        let settings = self.settings.get();
        !settings.notices.mouse_buttons_shown
            && !bindings::configured(&self.bindings, &settings)
                .iter()
                .any(|binding| matches!(binding.trigger, Trigger::Mouse { .. }))
    }

    /// Reads the connected mice in the background, after the dictation.
    fn check_mouse(&mut self) {
        let suggestions = &self.suggestions;
        let due = suggestions
            .mouse_checked_at
            .is_none_or(|at| at.elapsed() >= MOUSE_CHECK_EVERY);
        if suggestions.mouse_checking || !due || !self.wants_mouse_tip() {
            return;
        }
        self.suggestions.mouse_checking = true;
        self.suggestions.mouse_checked_at = Some(Instant::now());
        let mice = Arc::clone(&self.suggestions.mice);
        let tx = self.tx.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = tx.send(ControllerInput::MouseButtons(mice.most_buttons()));
        });
    }

    pub(super) fn mouse_buttons(&mut self, buttons: u32) {
        self.suggestions.mouse_checking = false;
        log::info!("the connected mice have up to {buttons} buttons");
        let allowed = NoticeCategory::Suggestion.allowed(&self.settings.get().notifications);
        if buttons <= BASIC_MOUSE_BUTTONS || !allowed || !self.wants_mouse_tip() {
            return;
        }
        let patch = json!({"notices": {"mouseButtonsShown": true}});
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not note the mouse suggestion: {error}");
        }
        self.ui.notice(
            Notice::info(t!("notice.tip.mouseButtons"))
                .with_action(NoticeAction::SetUpMouseButton)
                .with_action(NoticeAction::NotNow)
                .in_category(NoticeCategory::Suggestion),
        );
    }
}
