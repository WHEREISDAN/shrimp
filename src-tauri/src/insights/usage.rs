//! Usage by category (STA-06): Wispr Flow's six desktop categories, with
//! the words dictated into each and a Power, Moderate or Light label.

use rusqlite::Connection;
use serde::Serialize;

use crate::context::DictationContext;
use crate::edit::AppCategory;
use crate::storage::{StorageError, stats};

/// Mirrors `UsageCategory` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageCategory {
    PersonalMessages,
    WorkMessages,
    Emails,
    AiPrompts,
    Documents,
    OtherTasks,
}

impl UsageCategory {
    /// In the order Insights lists categories with the same words.
    pub const ALL: [Self; 6] = [
        Self::PersonalMessages,
        Self::WorkMessages,
        Self::Emails,
        Self::AiPrompts,
        Self::Documents,
        Self::OtherTasks,
    ];

    /// How the stats table names it.
    pub fn key(self) -> &'static str {
        match self {
            Self::PersonalMessages => "personalMessages",
            Self::WorkMessages => "workMessages",
            Self::Emails => "emails",
            Self::AiPrompts => "aiPrompts",
            Self::Documents => "documents",
            Self::OtherTasks => "otherTasks",
        }
    }

    /// What a row counts, as Wispr Flow counts it: dictations, except that
    /// emails count half and documents a fifth, rounded.
    fn count(self, dictations: i64) -> i64 {
        match self {
            Self::Emails => (dictations + 1) / 2,
            Self::Documents => (dictations * 2 + 5) / 10,
            _ => dictations,
        }
    }
}

/// Mirrors `UsageLevel` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageLevel {
    /// 10,000 words or more.
    Power,
    /// 1,000 to 9,999 words.
    Moderate,
    /// Under 1,000 words.
    Light,
}

impl UsageLevel {
    pub fn of(words: i64) -> Self {
        match words {
            10_000.. => Self::Power,
            1_000.. => Self::Moderate,
            _ => Self::Light,
        }
    }
}

/// One category's row on the Insights page. Mirrors `UsageActivity` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageActivity {
    pub category: UsageCategory,
    pub words: i64,
    pub dictations: i64,
    /// Messages, emails, prompts, documents or tasks (see `UsageCategory::count`).
    pub count: i64,
    pub level: UsageLevel,
}

/// The category of a dictation: the app's or website's style category
/// (STY-01, STY-05), AI assistants apart, and writing apps as documents.
pub fn classify(context: &DictationContext) -> UsageCategory {
    let target = context.target();
    if target.ai_app {
        return UsageCategory::AiPrompts;
    }
    match target.category {
        AppCategory::Personal => UsageCategory::PersonalMessages,
        AppCategory::Work => UsageCategory::WorkMessages,
        AppCategory::Email => UsageCategory::Emails,
        AppCategory::Other if context.is_document() => UsageCategory::Documents,
        AppCategory::Other => UsageCategory::OtherTasks,
    }
}

/// All six categories, zeros included, most words first.
pub fn usage(db: &Connection) -> Result<Vec<UsageActivity>, StorageError> {
    let totals = stats::usage(db)?;
    let mut rows: Vec<UsageActivity> = UsageCategory::ALL
        .into_iter()
        .map(|category| {
            let total = totals.iter().find(|total| total.category == category.key());
            let words = total.map_or(0, |total| total.words);
            let dictations = total.map_or(0, |total| total.dictations);
            UsageActivity {
                category,
                words,
                dictations,
                count: category.count(dictations),
                level: UsageLevel::of(words),
            }
        })
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.words));
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{AppIdentity, FieldContext};
    use crate::storage::Database;

    fn context(bundle_id: &str, name: &str, url: Option<&str>) -> DictationContext {
        let field = url.map(|url| FieldContext {
            url: Some(url.into()),
            in_web_content: true,
            ..FieldContext::default()
        });
        let app = AppIdentity {
            bundle_id: bundle_id.into(),
            name: name.into(),
        };
        DictationContext::new(Some(app), field)
    }

    #[test]
    fn apps_and_sites_land_in_their_category() {
        let category = |id, name, url| classify(&context(id, name, url));
        assert_eq!(
            category("com.apple.MobileSMS", "Messages", None),
            UsageCategory::PersonalMessages
        );
        assert_eq!(
            category("com.tinyspeck.slackmacgap", "Slack", None),
            UsageCategory::WorkMessages
        );
        assert_eq!(
            category("com.apple.mail", "Mail", None),
            UsageCategory::Emails
        );
        assert_eq!(
            category("com.openai.chat", "ChatGPT", None),
            UsageCategory::AiPrompts
        );
        assert_eq!(
            category("com.apple.TextEdit", "TextEdit", None),
            UsageCategory::Documents
        );
        assert_eq!(
            category("com.apple.Terminal", "Terminal", None),
            UsageCategory::OtherTasks
        );
        let chrome = ("com.google.Chrome", "Google Chrome");
        assert_eq!(
            category(
                chrome.0,
                chrome.1,
                Some("https://docs.google.com/document/d/1")
            ),
            UsageCategory::Documents
        );
        assert_eq!(
            category(chrome.0, chrome.1, Some("https://claude.ai/new")),
            UsageCategory::AiPrompts
        );
        assert_eq!(
            category(chrome.0, chrome.1, Some("https://example.com/")),
            UsageCategory::OtherTasks
        );
    }

    #[test]
    fn levels_follow_wispr_flows_bands() {
        let levels: Vec<UsageLevel> = [0, 999, 1_000, 9_999, 10_000]
            .into_iter()
            .map(UsageLevel::of)
            .collect();
        use UsageLevel::*;
        assert_eq!(levels, [Light, Light, Moderate, Moderate, Power]);
    }

    #[test]
    fn emails_count_half_and_documents_a_fifth() {
        assert_eq!(UsageCategory::Emails.count(5), 3);
        assert_eq!(UsageCategory::Documents.count(12), 2);
        assert_eq!(UsageCategory::Documents.count(13), 3);
        assert_eq!(UsageCategory::AiPrompts.count(7), 7);
    }

    #[test]
    fn every_category_is_listed_most_words_first() {
        let database = Database::open_in_memory().unwrap();
        let rows = database
            .with(|db| {
                let mut counted = stats::counted("2026-10-13", 1_200);
                counted.usage = Some(UsageCategory::Emails.key());
                stats::record(db, &counted)?;
                counted.words = 300;
                counted.usage = Some(UsageCategory::AiPrompts.key());
                stats::record(db, &counted)?;
                usage(db)
            })
            .unwrap();
        let order: Vec<UsageCategory> = rows.iter().map(|row| row.category).collect();
        use UsageCategory::*;
        assert_eq!(
            order,
            [
                Emails,
                AiPrompts,
                PersonalMessages,
                WorkMessages,
                Documents,
                OtherTasks
            ]
        );
        assert_eq!(rows[0].level, UsageLevel::Moderate);
        assert_eq!((rows[0].dictations, rows[0].count), (1, 1));
        assert_eq!(rows[2].words, 0);
    }

    #[test]
    fn rows_serialize_for_the_insights_page() {
        let row = UsageActivity {
            category: UsageCategory::AiPrompts,
            words: 12_000,
            dictations: 40,
            count: 40,
            level: UsageLevel::Power,
        };
        assert_eq!(
            serde_json::to_value(row).unwrap(),
            serde_json::json!({"category": "aiPrompts", "words": 12000, "dictations": 40,
                "count": 40, "level": "power"})
        );
    }
}
