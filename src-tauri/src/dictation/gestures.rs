//! Turns shortcut presses and Flow Bar clicks into dictation commands: hold to
//! talk, double-tap or Fn+Space to lock hands-free, the short-tap guard and the
//! busy guard. Pure and clock-free (callers pass `now`), so it is unit-tested.

use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::hotkeys::Action;

/// A press shorter than this is a tap, and a second press within this window
/// after a tap locks hands-free (CORE-03, CORE-05, CORE-09).
pub const TAP_WINDOW: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingMode {
    /// Recording while the push-to-talk shortcut is held.
    Hold,
    /// Hands-free: recording continues until stopped.
    Locked,
}

/// Flow Bar buttons. `Start` is a click on the resting bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BarControl {
    Start,
    Stop,
    Cancel,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Press(Action),
    Release(Action),
    Bar(BarControl),
    /// The wait for a second tap ran out; carries the token from `ArmTapTimeout`.
    TapTimeout(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    User,
    /// Push to talk was tapped instead of held (CORE-03).
    ShortTap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start(RecordingMode),
    /// Switch the running recording to hands-free.
    Lock,
    /// Stop recording, then transcribe and insert.
    Finish,
    Cancel(CancelReason),
    /// Deliver `Input::TapTimeout(token)` after the given delay.
    ArmTapTimeout {
        token: u64,
        after: Duration,
    },
    /// A shortcut arrived while a transcript is processing (CORE-07).
    RejectBusy,
    PasteLast,
    CopyLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Holding {
        pressed_at: Instant,
    },
    AwaitingSecondTap {
        token: u64,
    },
    Locked {
        since: Instant,
        /// Locked by a double tap: a quick third press cancels.
        by_double_tap: bool,
        /// Started from the Flow Bar: a quick second click cancels.
        by_click: bool,
    },
    Busy {
        /// The push-to-talk press that stopped a hands-free recording is still
        /// held, so its Fn+Space continuation is not a new shortcut.
        chord: bool,
    },
}

#[derive(Debug)]
pub struct Gestures {
    phase: Phase,
    push_to_talk_down: bool,
    next_token: u64,
}

impl Default for Gestures {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            push_to_talk_down: false,
            next_token: 0,
        }
    }
}

impl Gestures {
    pub fn is_recording(&self) -> bool {
        self.mode().is_some()
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.phase, Phase::Busy { .. })
    }

    pub fn mode(&self) -> Option<RecordingMode> {
        match self.phase {
            Phase::Holding { .. } | Phase::AwaitingSecondTap { .. } => Some(RecordingMode::Hold),
            Phase::Locked { .. } => Some(RecordingMode::Locked),
            Phase::Idle | Phase::Busy { .. } => None,
        }
    }

    pub fn handle(&mut self, input: Input, now: Instant) -> Option<Command> {
        match input {
            Input::Press(Action::PushToTalk) => self.push_to_talk_down = true,
            Input::Release(Action::PushToTalk) => self.push_to_talk_down = false,
            _ => {}
        }
        let (phase, command) = self.next(input, now);
        self.phase = phase;
        command
    }

    /// The session limit was reached (CORE-08). Returns whether a recording
    /// was running and must now finish.
    pub fn force_finish(&mut self) -> bool {
        let recording = self.is_recording();
        if recording {
            self.phase = Phase::Busy { chord: false };
        }
        recording
    }

    /// Work outside a recording (paste-last retry) also blocks shortcuts.
    pub fn begin_busy(&mut self) {
        self.phase = Phase::Busy { chord: false };
    }

    /// Processing ended, or a recording could not start.
    pub fn reset(&mut self) {
        self.phase = Phase::Idle;
    }

    fn next(&mut self, input: Input, now: Instant) -> (Phase, Option<Command>) {
        use Action::{CopyLast, HandsFree, PasteLast, PushToTalk};
        use Input::{Bar, Press, Release, TapTimeout};

        let busy = Phase::Busy { chord: false };
        let idle = Phase::Idle;
        let cancel = Some(Command::Cancel(CancelReason::User));
        let finish = Some(Command::Finish);

        match (self.phase, input) {
            (Phase::Idle, Press(PushToTalk)) => (
                Phase::Holding { pressed_at: now },
                Some(Command::Start(RecordingMode::Hold)),
            ),
            (Phase::Idle, Press(HandsFree)) => (
                self.locked(now, false, false),
                Some(Command::Start(RecordingMode::Locked)),
            ),
            (Phase::Idle, Bar(BarControl::Start)) => (
                self.locked(now, false, true),
                Some(Command::Start(RecordingMode::Locked)),
            ),
            (Phase::Idle, Press(PasteLast)) => (idle, Some(Command::PasteLast)),
            (Phase::Idle, Press(CopyLast)) => (idle, Some(Command::CopyLast)),

            (Phase::Holding { pressed_at }, Release(PushToTalk)) => {
                if now.duration_since(pressed_at) < TAP_WINDOW {
                    let token = self.take_token();
                    (
                        Phase::AwaitingSecondTap { token },
                        Some(Command::ArmTapTimeout {
                            token,
                            after: TAP_WINDOW,
                        }),
                    )
                } else {
                    (busy, finish)
                }
            }
            (Phase::AwaitingSecondTap { .. }, Press(PushToTalk)) => {
                (self.locked(now, true, false), Some(Command::Lock))
            }
            (Phase::AwaitingSecondTap { token }, TapTimeout(fired)) if fired == token => {
                (idle, Some(Command::Cancel(CancelReason::ShortTap)))
            }
            (Phase::Holding { .. } | Phase::AwaitingSecondTap { .. }, Press(HandsFree)) => {
                (self.locked(now, false, false), Some(Command::Lock))
            }

            (
                Phase::Locked {
                    since,
                    by_double_tap,
                    ..
                },
                Press(PushToTalk),
            ) => {
                if by_double_tap && now.duration_since(since) < TAP_WINDOW {
                    (idle, cancel)
                } else {
                    (Phase::Busy { chord: true }, finish)
                }
            }
            (Phase::Locked { .. }, Press(HandsFree)) => (busy, finish),
            (
                Phase::Locked {
                    since,
                    by_click: true,
                    ..
                },
                Bar(BarControl::Start | BarControl::Stop | BarControl::Confirm),
            ) if now.duration_since(since) < TAP_WINDOW => (idle, cancel),

            (
                Phase::Holding { .. } | Phase::AwaitingSecondTap { .. } | Phase::Locked { .. },
                Press(Action::Cancel) | Bar(BarControl::Cancel),
            ) => (idle, cancel),
            (
                Phase::Holding { .. } | Phase::AwaitingSecondTap { .. } | Phase::Locked { .. },
                Bar(BarControl::Stop | BarControl::Confirm),
            ) => (busy, finish),

            (Phase::Busy { chord: true }, Press(HandsFree)) if self.push_to_talk_down => {
                (self.phase, None)
            }
            (
                Phase::Busy { .. },
                Press(PushToTalk | HandsFree | PasteLast | CopyLast) | Bar(BarControl::Start),
            ) => (busy, Some(Command::RejectBusy)),

            (phase, _) => (phase, None),
        }
    }

    fn locked(&self, since: Instant, by_double_tap: bool, by_click: bool) -> Phase {
        Phase::Locked {
            since,
            by_double_tap,
            by_click,
        }
    }

    fn take_token(&mut self) -> u64 {
        self.next_token += 1;
        self.next_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PTT: Action = Action::PushToTalk;

    struct Script {
        gestures: Gestures,
        start: Instant,
    }

    impl Script {
        fn new() -> Self {
            Self {
                gestures: Gestures::default(),
                start: Instant::now(),
            }
        }

        fn at(&mut self, ms: u64, input: Input) -> Option<Command> {
            let now = self.start + Duration::from_millis(ms);
            self.gestures.handle(input, now)
        }
    }

    #[test]
    fn hold_records_and_release_finishes() {
        let mut s = Script::new();
        assert_eq!(
            s.at(0, Input::Press(PTT)),
            Some(Command::Start(RecordingMode::Hold))
        );
        assert_eq!(s.at(1_200, Input::Release(PTT)), Some(Command::Finish));
        assert!(s.gestures.is_busy());
    }

    #[test]
    fn short_tap_waits_then_cancels_with_the_tip() {
        let mut s = Script::new();
        s.at(0, Input::Press(PTT));
        let Some(Command::ArmTapTimeout { token, after }) = s.at(200, Input::Release(PTT)) else {
            panic!("expected a tap timeout");
        };
        assert_eq!(after, TAP_WINDOW);
        assert!(
            s.gestures.is_recording(),
            "audio keeps flowing while waiting"
        );
        assert_eq!(
            s.at(700, Input::TapTimeout(token)),
            Some(Command::Cancel(CancelReason::ShortTap))
        );
        assert!(!s.gestures.is_recording());
    }

    #[test]
    fn double_tap_locks_and_a_later_press_finishes() {
        let mut s = Script::new();
        s.at(0, Input::Press(PTT));
        let Some(Command::ArmTapTimeout { token, .. }) = s.at(150, Input::Release(PTT)) else {
            panic!("expected a tap timeout");
        };
        assert_eq!(s.at(300, Input::Press(PTT)), Some(Command::Lock));
        assert_eq!(s.at(400, Input::Release(PTT)), None);
        assert_eq!(s.at(650, Input::TapTimeout(token)), None, "stale timeout");
        assert_eq!(s.gestures.mode(), Some(RecordingMode::Locked));
        assert_eq!(s.at(5_000, Input::Press(PTT)), Some(Command::Finish));
    }

    #[test]
    fn a_quick_third_press_cancels_a_double_tap_lock() {
        let mut s = Script::new();
        s.at(0, Input::Press(PTT));
        s.at(100, Input::Release(PTT));
        s.at(200, Input::Press(PTT));
        s.at(250, Input::Release(PTT));
        assert_eq!(
            s.at(400, Input::Press(PTT)),
            Some(Command::Cancel(CancelReason::User))
        );
    }

    #[test]
    fn fn_space_locks_and_fn_space_again_finishes_without_a_busy_notice() {
        let mut s = Script::new();
        assert_eq!(
            s.at(0, Input::Press(PTT)),
            Some(Command::Start(RecordingMode::Hold))
        );
        assert_eq!(
            s.at(80, Input::Press(Action::HandsFree)),
            Some(Command::Lock)
        );
        s.at(150, Input::Release(Action::HandsFree));
        assert_eq!(s.at(200, Input::Release(PTT)), None);
        assert_eq!(s.gestures.mode(), Some(RecordingMode::Locked));

        assert_eq!(s.at(4_000, Input::Press(PTT)), Some(Command::Finish));
        assert_eq!(s.at(4_060, Input::Press(Action::HandsFree)), None);
    }

    #[test]
    fn hands_free_alone_starts_locked_and_stops() {
        let mut s = Script::new();
        assert_eq!(
            s.at(0, Input::Press(Action::HandsFree)),
            Some(Command::Start(RecordingMode::Locked))
        );
        assert_eq!(
            s.at(3_000, Input::Press(Action::HandsFree)),
            Some(Command::Finish)
        );
    }

    #[test]
    fn escape_and_the_bar_cancel_cancel_any_recording() {
        for cancel in [Input::Press(Action::Cancel), Input::Bar(BarControl::Cancel)] {
            let mut s = Script::new();
            s.at(0, Input::Press(PTT));
            assert_eq!(s.at(900, cancel), Some(Command::Cancel(CancelReason::User)));
            assert!(!s.gestures.is_recording());
        }
    }

    #[test]
    fn shortcuts_while_processing_are_rejected() {
        let mut s = Script::new();
        s.at(0, Input::Press(PTT));
        s.at(1_000, Input::Release(PTT));
        assert_eq!(s.at(1_200, Input::Press(PTT)), Some(Command::RejectBusy));
        assert_eq!(s.at(1_300, Input::Release(PTT)), None);
        assert_eq!(
            s.at(1_400, Input::Press(Action::PasteLast)),
            Some(Command::RejectBusy)
        );
        assert_eq!(
            s.at(1_500, Input::Bar(BarControl::Start)),
            Some(Command::RejectBusy)
        );
        s.gestures.reset();
        assert_eq!(
            s.at(2_000, Input::Press(PTT)),
            Some(Command::Start(RecordingMode::Hold))
        );
    }

    #[test]
    fn bar_click_starts_hands_free_and_stop_finishes() {
        let mut s = Script::new();
        assert_eq!(
            s.at(0, Input::Bar(BarControl::Start)),
            Some(Command::Start(RecordingMode::Locked))
        );
        assert_eq!(
            s.at(2_000, Input::Bar(BarControl::Stop)),
            Some(Command::Finish)
        );
    }

    #[test]
    fn a_bar_click_within_half_a_second_of_starting_cancels() {
        let mut s = Script::new();
        s.at(0, Input::Bar(BarControl::Start));
        assert_eq!(
            s.at(300, Input::Bar(BarControl::Start)),
            Some(Command::Cancel(CancelReason::User))
        );
        let mut s = Script::new();
        s.at(0, Input::Bar(BarControl::Start));
        assert_eq!(
            s.at(300, Input::Bar(BarControl::Stop)),
            Some(Command::Cancel(CancelReason::User))
        );
    }

    #[test]
    fn confirm_finishes_a_held_recording() {
        let mut s = Script::new();
        s.at(0, Input::Press(PTT));
        assert_eq!(
            s.at(2_000, Input::Bar(BarControl::Confirm)),
            Some(Command::Finish)
        );
    }

    #[test]
    fn last_transcript_shortcuts_only_act_when_idle() {
        let mut s = Script::new();
        assert_eq!(
            s.at(0, Input::Press(Action::PasteLast)),
            Some(Command::PasteLast)
        );
        assert_eq!(
            s.at(10, Input::Press(Action::CopyLast)),
            Some(Command::CopyLast)
        );
        s.at(20, Input::Press(PTT));
        assert_eq!(s.at(30, Input::Press(Action::PasteLast)), None);
    }

    #[test]
    fn session_limit_finishes_a_running_recording() {
        let mut s = Script::new();
        assert!(!s.gestures.force_finish());
        s.at(0, Input::Press(PTT));
        assert!(s.gestures.force_finish());
        assert!(s.gestures.is_busy());
        assert_eq!(s.at(10, Input::Release(PTT)), None);
    }
}
