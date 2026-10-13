//! Formatting tips (FMT-13): the first list, email and backtrack an AI edit
//! makes each get one explanation, then up to 3 shorter reminders, at most
//! one a day. They are Announcements, so that switch mutes them.

use crate::notify::{Notice, NoticeCategory};
use crate::settings::FormattingTips;

const REMINDERS: u32 = 3;
/// Self-corrections whose cue the edit removed.
const BACKTRACK_CUES: [&str; 6] = [
    "scratch that",
    "no wait",
    "actually",
    "i mean",
    "sorry",
    "never mind",
];

/// What smart formatting did to a dictation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Formatting {
    pub list: bool,
    pub email: bool,
    pub backtrack: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tip {
    List,
    Email,
    Backtrack,
}

/// Whether the edit dropped a self-correction cue from the dictation.
pub fn backtracked(raw: &str, edited: &str) -> bool {
    let raw = raw.to_lowercase();
    let edited = edited.to_lowercase();
    BACKTRACK_CUES
        .iter()
        .any(|cue| contains_phrase(&raw, cue) && !contains_phrase(&edited, cue))
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + phrase.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// The tip to show for `formatting`, whether it is the explanation, and the
/// counts after showing it.
pub fn next(
    formatting: Formatting,
    shown: &FormattingTips,
    today: &str,
) -> Option<(Tip, bool, FormattingTips)> {
    let candidates = [
        (formatting.backtrack, Tip::Backtrack, shown.backtrack),
        (formatting.list, Tip::List, shown.list),
        (formatting.email, Tip::Email, shown.email),
    ];
    let (tip, count) = candidates
        .into_iter()
        .filter(|(happened, _, count)| *happened && *count <= REMINDERS)
        .find(|(_, _, count)| *count == 0 || shown.day != today)
        .map(|(_, tip, count)| (tip, count))?;
    let mut after = shown.clone();
    after.day = today.to_owned();
    match tip {
        Tip::List => after.list += 1,
        Tip::Email => after.email += 1,
        Tip::Backtrack => after.backtrack += 1,
    }
    Some((tip, count == 0, after))
}

pub fn notice(tip: Tip, explanation: bool) -> Notice {
    let message = match (tip, explanation) {
        (Tip::List, true) => {
            "Shrimp made that a list. Say “one… two… three…” or “first… second…” and it \
             formats the items for you."
        }
        (Tip::List, false) => "Tip: say “one… two…” to make a list.",
        (Tip::Email, true) => {
            "Shrimp laid that out as an email: the greeting, the body and the sign-off each \
             on their own lines."
        }
        (Tip::Email, false) => "Tip: in email apps, Shrimp lays out greetings and sign-offs.",
        (Tip::Backtrack, true) => {
            "Shrimp kept only your correction. Say “actually”, “no wait” or “scratch that” to \
             change what you just said."
        }
        (Tip::Backtrack, false) => "Tip: say “actually…” to correct yourself as you speak.",
    };
    Notice::info(message).in_category(NoticeCategory::Announcement)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-10-13";
    const LIST: Formatting = Formatting {
        list: true,
        email: false,
        backtrack: false,
    };

    #[test]
    fn one_explanation_then_three_daily_reminders() {
        let mut shown = FormattingTips::default();
        let (tip, first, after) = next(LIST, &shown, TODAY).unwrap();
        assert_eq!((tip, first), (Tip::List, true));
        shown = after;
        assert_eq!(next(LIST, &shown, TODAY), None, "one a day");
        for day in ["2026-10-14", "2026-10-15", "2026-10-16"] {
            let (_, first, after) = next(LIST, &shown, day).unwrap();
            assert!(!first);
            shown = after;
        }
        assert_eq!(shown.list, 4);
        assert_eq!(
            next(LIST, &shown, "2026-10-20"),
            None,
            "no more after 3 reminders"
        );
    }

    #[test]
    fn a_new_kind_explains_itself_the_same_day() {
        let shown = FormattingTips {
            list: 1,
            day: TODAY.into(),
            ..FormattingTips::default()
        };
        let both = Formatting {
            list: true,
            email: true,
            backtrack: false,
        };
        let (tip, first, _) = next(both, &shown, TODAY).unwrap();
        assert_eq!((tip, first), (Tip::Email, true));
    }

    #[test]
    fn finds_dropped_corrections() {
        assert!(backtracked("coffee at two actually three", "Coffee at 3."));
        assert!(!backtracked(
            "I actually enjoyed it",
            "I actually enjoyed it."
        ));
        assert!(!backtracked("factually correct", "Factually correct."));
    }

    #[test]
    fn tips_are_announcements() {
        assert_eq!(
            notice(Tip::List, true).category,
            NoticeCategory::Announcement
        );
    }
}
