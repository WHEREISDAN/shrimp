//! App categories and styles (STY-01..04) and the deterministic style rules:
//! trailing periods (FMT-07) and very casual lowercase.

use serde::{Deserialize, Serialize};

use crate::platform::AppIdentity;

/// Mirrors `AppCategory` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppCategory {
    Personal,
    Work,
    Email,
    Other,
}

/// Mirrors `Style` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Style {
    Formal,
    Casual,
    VeryCasual,
    Excited,
}

impl AppCategory {
    /// Very casual is for personal messages only; Excited for the rest.
    pub fn styles(self) -> &'static [Style] {
        match self {
            Self::Personal => &[Style::Formal, Style::Casual, Style::VeryCasual],
            Self::Work | Self::Email | Self::Other => {
                &[Style::Formal, Style::Casual, Style::Excited]
            }
        }
    }

    pub fn is_messaging(self) -> bool {
        matches!(self, Self::Personal | Self::Work)
    }
}

/// Where a dictation goes, as far as formatting is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub category: AppCategory,
    /// AI assistants get no style formatting (FMT-09).
    pub ai_app: bool,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            category: AppCategory::Other,
            ai_app: false,
        }
    }
}

/// Built-in apps by bundle ID (macOS) with their display names as a fallback,
/// so Windows executables and renamed bundles still match by name.
const APPS: &[(&str, &str, AppCategory)] = &[
    ("com.apple.MobileSMS", "Messages", AppCategory::Personal),
    ("net.whatsapp.WhatsApp", "WhatsApp", AppCategory::Personal),
    ("desktop.WhatsApp", "WhatsApp", AppCategory::Personal),
    ("com.hnc.Discord", "Discord", AppCategory::Personal),
    (
        "org.whispersystems.signal-desktop",
        "Signal",
        AppCategory::Personal,
    ),
    ("ru.keepcoder.Telegram", "Telegram", AppCategory::Personal),
    (
        "org.telegram.desktop",
        "Telegram Desktop",
        AppCategory::Personal,
    ),
    ("com.burbn.instagram", "Instagram", AppCategory::Personal),
    ("com.facebook.archon", "Messenger", AppCategory::Personal),
    ("com.tinyspeck.slackmacgap", "Slack", AppCategory::Work),
    ("com.microsoft.teams2", "Microsoft Teams", AppCategory::Work),
    (
        "com.microsoft.teams",
        "Microsoft Teams classic",
        AppCategory::Work,
    ),
    ("com.linkedin.LinkedIn", "LinkedIn", AppCategory::Work),
    ("com.apple.mail", "Mail", AppCategory::Email),
    (
        "com.microsoft.Outlook",
        "Microsoft Outlook",
        AppCategory::Email,
    ),
    ("com.missiveapp.osx", "Missive", AppCategory::Email),
    ("com.superhuman.electron", "Superhuman", AppCategory::Email),
    (
        "com.readdle.SparkDesktop",
        "Spark Desktop",
        AppCategory::Email,
    ),
    ("org.mozilla.thunderbird", "Thunderbird", AppCategory::Email),
    (
        "com.mimestream.Mimestream",
        "Mimestream",
        AppCategory::Email,
    ),
];

/// AI assistants: Other, and formatting is skipped (FMT-09).
const AI_APPS: &[(&str, &str)] = &[
    ("com.openai.chat", "ChatGPT"),
    ("com.openai.codex", "Codex"),
    ("com.anthropic.claudefordesktop", "Claude"),
    ("ai.perplexity.mac", "Perplexity"),
];

/// Debug builds only: categories an agent sets for end-to-end tests
/// (`dev_set_app_category`), keyed by bundle ID.
#[cfg(debug_assertions)]
static OVERRIDES: std::sync::Mutex<Vec<(String, Target)>> = std::sync::Mutex::new(Vec::new());

#[cfg(debug_assertions)]
pub fn set_override(bundle_id: &str, target: Option<Target>) {
    let mut overrides = OVERRIDES.lock().unwrap_or_else(|err| err.into_inner());
    overrides.retain(|(id, _)| id != bundle_id);
    if let Some(target) = target {
        overrides.push((bundle_id.to_owned(), target));
    }
}

pub fn categorize(app: Option<&AppIdentity>) -> Target {
    let Some(app) = app else {
        return Target::default();
    };
    #[cfg(debug_assertions)]
    if let Some((_, target)) = OVERRIDES
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .iter()
        .find(|(id, _)| *id == app.bundle_id)
    {
        return *target;
    }
    let matches = |id: &str, name: &str| {
        app.bundle_id.eq_ignore_ascii_case(id) || app.name.eq_ignore_ascii_case(name)
    };
    if AI_APPS.iter().any(|(id, name)| matches(id, name)) {
        return Target {
            category: AppCategory::Other,
            ai_app: true,
        };
    }
    let category = APPS
        .iter()
        .find(|(id, name, _)| matches(id, name))
        .map_or(AppCategory::Other, |(_, _, category)| *category);
    Target {
        category,
        ai_app: false,
    }
}

/// Removes the final period (and any ellipsis) when the style calls for it,
/// keeping `!` and `?` (FMT-07).
pub fn trailing_periods(text: &str, style: Style, category: AppCategory) -> String {
    let limit = match style {
        Style::Formal => return text.to_owned(),
        Style::Casual => 10,
        Style::VeryCasual => usize::MAX,
        Style::Excited if category.is_messaging() => 2,
        Style::Excited => return text.to_owned(),
    };
    if sentence_count(text) > limit {
        return text.to_owned();
    }
    let body = text.trim_end();
    let stripped = body.trim_end_matches(['.', '…']);
    format!("{stripped}{}", &text[body.len()..])
}

/// Sentences end with `.`, `!`, `?` or `…` followed by a space or the end.
fn sentence_count(text: &str) -> usize {
    let chars: Vec<char> = text.trim_end().chars().collect();
    let ends = |c: char| matches!(c, '.' | '!' | '?' | '…');
    let mut count = 0;
    for (i, c) in chars.iter().enumerate() {
        let next = chars.get(i + 1);
        if ends(*c) && next.is_none_or(|next| next.is_whitespace()) {
            count += 1;
        }
    }
    if chars.last().is_some_and(|c| !ends(*c)) {
        count += 1;
    }
    count
}

/// Lowercases each sentence's first word unless it is protected (a
/// dictionary spelling), "I" or an acronym.
pub fn very_casual(text: &str, protected: &[String]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at_start = true;
    for (index, c) in text.char_indices() {
        if at_start && c.is_alphabetic() {
            let word: String = text[index..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '\'' || *c == '’')
                .collect();
            let keep = word == "I"
                || word.starts_with("I'")
                || word.starts_with("I’")
                || word.chars().filter(|c| c.is_uppercase()).count() > 1
                || protected
                    .iter()
                    .any(|term| term.split_whitespace().next() == Some(&word));
            if keep {
                out.push(c);
            } else {
                out.extend(c.to_lowercase());
            }
            at_start = false;
            continue;
        }
        out.push(c);
        if matches!(c, '.' | '!' | '?' | '\n') {
            at_start = true;
        } else if !c.is_whitespace()
            && !matches!(c, '-' | '*' | '•' | '"' | '“' | '(')
            && !c.is_ascii_digit()
        {
            at_start = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(bundle_id: &str, name: &str) -> AppIdentity {
        AppIdentity {
            bundle_id: bundle_id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn documented_apps_land_in_their_category() {
        let category = |id: &str, name: &str| categorize(Some(&app(id, name))).category;
        assert_eq!(
            category("com.apple.MobileSMS", "Messages"),
            AppCategory::Personal
        );
        assert_eq!(
            category("net.whatsapp.WhatsApp", "WhatsApp"),
            AppCategory::Personal
        );
        assert_eq!(
            category("com.hnc.Discord", "Discord"),
            AppCategory::Personal
        );
        assert_eq!(category("x", "Signal"), AppCategory::Personal);
        assert_eq!(
            category("com.burbn.instagram", "Instagram"),
            AppCategory::Personal
        );
        assert_eq!(
            category("com.tinyspeck.slackmacgap", "Slack"),
            AppCategory::Work
        );
        assert_eq!(
            category("com.microsoft.teams2", "Microsoft Teams"),
            AppCategory::Work
        );
        assert_eq!(category("x", "LinkedIn"), AppCategory::Work);
        assert_eq!(category("com.apple.mail", "Mail"), AppCategory::Email);
        assert_eq!(
            category("com.microsoft.Outlook", "Microsoft Outlook"),
            AppCategory::Email
        );
        assert_eq!(
            category("com.missiveapp.osx", "Missive"),
            AppCategory::Email
        );
        assert_eq!(
            category("com.apple.TextEdit", "TextEdit"),
            AppCategory::Other
        );
        assert_eq!(
            category("com.apple.Terminal", "Terminal"),
            AppCategory::Other
        );
        assert_eq!(categorize(None).category, AppCategory::Other);
    }

    #[test]
    fn ai_assistants_are_other_and_unformatted() {
        let target = categorize(Some(&app("com.anthropic.claudefordesktop", "Claude")));
        assert_eq!(target.category, AppCategory::Other);
        assert!(target.ai_app);
        assert!(categorize(Some(&app("com.openai.chat", "ChatGPT"))).ai_app);
        assert!(!categorize(Some(&app("dev.warp.Warp-Stable", "Warp"))).ai_app);
    }

    #[test]
    fn personal_offers_very_casual_and_the_rest_excited() {
        assert!(AppCategory::Personal.styles().contains(&Style::VeryCasual));
        assert!(!AppCategory::Personal.styles().contains(&Style::Excited));
        for category in [AppCategory::Work, AppCategory::Email, AppCategory::Other] {
            assert!(category.styles().contains(&Style::Excited));
            assert!(!category.styles().contains(&Style::VeryCasual));
        }
    }

    #[test]
    fn trailing_periods_follow_the_style() {
        let one = "See you soon.";
        assert_eq!(
            trailing_periods(one, Style::Formal, AppCategory::Personal),
            one
        );
        assert_eq!(
            trailing_periods(one, Style::Casual, AppCategory::Other),
            "See you soon"
        );
        assert_eq!(
            trailing_periods("Wait...", Style::VeryCasual, AppCategory::Personal),
            "Wait"
        );
        assert_eq!(
            trailing_periods("Really?", Style::VeryCasual, AppCategory::Personal),
            "Really?"
        );
        let three = "One. Two! Three.";
        assert_eq!(
            trailing_periods(three, Style::Excited, AppCategory::Work),
            three
        );
        assert_eq!(
            trailing_periods("Great news. Ship it.", Style::Excited, AppCategory::Work),
            "Great news. Ship it"
        );
        assert_eq!(
            trailing_periods("Ship it.", Style::Excited, AppCategory::Email),
            "Ship it."
        );
        let eleven = "A. ".repeat(11);
        assert_eq!(
            trailing_periods(&eleven, Style::Casual, AppCategory::Other),
            eleven
        );
    }

    #[test]
    fn very_casual_lowercases_sentence_starts_only() {
        let protected = vec!["Shrimp".to_owned()];
        assert_eq!(
            very_casual(
                "Sounds good. See you at Mia's? I think so. Shrimp rocks. NASA too",
                &protected
            ),
            "sounds good. see you at Mia's? I think so. Shrimp rocks. NASA too"
        );
        assert_eq!(very_casual("- Milk\n- Eggs", &[]), "- milk\n- eggs");
    }
}
