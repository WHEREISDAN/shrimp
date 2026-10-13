//! Insights (STA-01..05): speed, totals, streaks, the activity heatmap and
//! apps, read from the stats tables that outlive history; word-count and
//! voice-profile milestones; and the voice profile itself (`profile`).

pub mod profile;

use chrono::{Datelike, Days, Months, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;

use crate::notify::{Notice, NoticeAction, NoticeCategory};
use crate::storage::{StorageError, stats};

/// The heatmap covers this many days back from today.
const HEATMAP_DAYS: u64 = 371;
const TOP_APPS: usize = 8;
/// Word counts worth a "Milestones" notification.
const WORD_MILESTONES: [i64; 9] = [
    1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 1_000_000,
];

/// Mirrors `Insights` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    /// Words per minute over the last 100 dictations with words.
    pub wpm: i64,
    pub total_words: i64,
    pub total_dictations: i64,
    /// Words the AI edit removed or changed (fillers, false starts, backtracks).
    pub words_corrected: i64,
    /// Dictionary corrections and snippets.
    pub dictionary_fixes: i64,
    /// This month so far against the same days last month.
    pub month: Option<MonthCompare>,
    pub current_streak: i64,
    pub longest_streak: i64,
    /// Since Monday.
    pub week_words: i64,
    /// Active days of the last year, oldest first.
    pub days: Vec<DayActivity>,
    /// The apps dictated into most.
    pub apps: Vec<AppActivity>,
    pub apps_used: i64,
    pub top_app: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthCompare {
    pub current: i64,
    pub previous: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayActivity {
    /// YYYY-MM-DD.
    pub day: String,
    pub words: i64,
    pub dictations: i64,
    pub apps: i64,
    pub top_app: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppActivity {
    pub name: String,
    pub words: i64,
    pub dictations: i64,
}

/// Everything the Insights page and Home's stats card show, as of `today`.
pub fn insights(db: &Connection, today: NaiveDate) -> Result<Insights, StorageError> {
    let totals = stats::totals(db)?;
    let active: Vec<NaiveDate> = stats::active_days(db)?
        .iter()
        .filter_map(|day| parse(day))
        .collect();
    let (current_streak, longest_streak) = streaks(&active, today);
    let apps = stats::apps(db)?;
    let from = today - Days::new(HEATMAP_DAYS);
    let monday = today - Days::new(u64::from(today.weekday().num_days_from_monday()));
    Ok(Insights {
        wpm: stats::speed(db)?,
        total_words: totals.words,
        total_dictations: totals.dictations,
        words_corrected: totals.fixes,
        dictionary_fixes: totals.dictionary_fixes,
        month: month(db, today)?,
        current_streak,
        longest_streak,
        week_words: stats::words_between(db, &format(monday), &format(today))?,
        days: stats::days_since(db, &format(from))?
            .into_iter()
            .map(|day| DayActivity {
                day: day.day,
                words: day.words,
                dictations: day.dictations,
                apps: day.apps,
                top_app: day.top_app,
            })
            .collect(),
        apps_used: apps.len() as i64,
        top_app: apps.first().map(|app| app.name.clone()),
        apps: apps
            .into_iter()
            .take(TOP_APPS)
            .map(|app| AppActivity {
                name: app.name,
                words: app.words,
                dictations: app.dictations,
            })
            .collect(),
    })
}

fn month(db: &Connection, today: NaiveDate) -> Result<Option<MonthCompare>, StorageError> {
    let first = today.with_day(1).unwrap_or(today);
    let Some(previous_first) = first.checked_sub_months(Months::new(1)) else {
        return Ok(None);
    };
    // The same day last month, or that month's last day.
    let previous_same = (0..4)
        .find_map(|back| previous_first.with_day(today.day().saturating_sub(back)))
        .unwrap_or(previous_first);
    let previous = stats::words_between(db, &format(previous_first), &format(previous_same))?;
    if previous == 0 {
        return Ok(None);
    }
    let current = stats::words_between(db, &format(first), &format(today))?;
    Ok(Some(MonthCompare { current, previous }))
}

/// The current streak (consecutive days up to the last active day, which
/// must be today or yesterday) and the longest one.
pub fn streaks(days: &[NaiveDate], today: NaiveDate) -> (i64, i64) {
    let mut longest = 0;
    let mut run = 0;
    let mut previous: Option<NaiveDate> = None;
    for day in days {
        run = match previous {
            Some(last) if last.succ_opt() == Some(*day) => run + 1,
            Some(last) if last == *day => run,
            _ => 1,
        };
        longest = longest.max(run);
        previous = Some(*day);
    }
    let current = match previous {
        Some(last) if last == today || last.succ_opt() == Some(today) => run,
        _ => 0,
    };
    (current, longest)
}

/// A milestone a dictation reached, for a "Milestones" notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Milestone {
    Words(i64),
    /// The voice profile can be generated (first) or updated.
    VoiceProfile {
        first: bool,
    },
}

/// What crossing from `before` to `after` total words reached; the voice
/// profile wins when both happen.
pub fn milestone(before: i64, after: i64) -> Option<Milestone> {
    let next_profile = profile::next_milestone(before);
    if before < next_profile && after >= next_profile {
        return Some(Milestone::VoiceProfile {
            first: next_profile == profile::FIRST_MILESTONE,
        });
    }
    WORD_MILESTONES
        .iter()
        .rev()
        .find(|&&count| before < count && after >= count)
        .map(|&count| Milestone::Words(count))
}

/// The "Milestones" notification for `milestone`.
pub fn notice(milestone: Milestone) -> Notice {
    let notice =
        match milestone {
            Milestone::Words(count) => Notice::info(format!(
                "You've dictated {} words with Shrimp.",
                thousands(count)
            ))
            .with_action(NoticeAction::OpenInsights),
            Milestone::VoiceProfile { first: true } => Notice::info("Discover your Voice Profile")
                .with_action(NoticeAction::SeeVoiceProfile),
            Milestone::VoiceProfile { first: false } => Notice::info("Update your Voice Profile")
                .with_action(NoticeAction::UpdateVoiceProfile),
        };
    notice.in_category(NoticeCategory::Milestone)
}

/// 25000 as "25,000".
fn thousands(count: i64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn parse(day: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

fn format(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn date(text: &str) -> NaiveDate {
        parse(text).unwrap()
    }

    #[test]
    fn streaks_allow_today_to_be_empty() {
        let days = [
            date("2026-10-01"),
            date("2026-10-02"),
            date("2026-10-03"),
            date("2026-10-10"),
            date("2026-10-11"),
            date("2026-10-12"),
        ];
        assert_eq!(streaks(&days, date("2026-10-12")), (3, 3));
        assert_eq!(
            streaks(&days, date("2026-10-13")),
            (3, 3),
            "yesterday counts"
        );
        assert_eq!(streaks(&days, date("2026-10-14")), (0, 3));
        assert_eq!(streaks(&[], date("2026-10-14")), (0, 0));
    }

    #[test]
    fn milestones_prefer_the_voice_profile() {
        assert_eq!(milestone(900, 1_100), Some(Milestone::Words(1_000)));
        assert_eq!(
            milestone(1_990, 2_010),
            Some(Milestone::VoiceProfile { first: true })
        );
        assert_eq!(
            milestone(6_990, 7_010),
            Some(Milestone::VoiceProfile { first: false })
        );
        assert_eq!(milestone(4_990, 5_010), Some(Milestone::Words(5_000)));
        assert_eq!(milestone(100, 200), None);
    }

    #[test]
    fn milestone_notices_offer_insights() {
        let words = notice(Milestone::Words(25_000));
        assert_eq!(words.message, "You've dictated 25,000 words with Shrimp.");
        assert_eq!(words.action(), Some(NoticeAction::OpenInsights));
        assert_eq!(
            notice(Milestone::VoiceProfile { first: true }).action(),
            Some(NoticeAction::SeeVoiceProfile)
        );
    }

    #[test]
    fn reads_the_stats_into_insights() {
        let database = Database::open_in_memory().unwrap();
        let insights = database
            .with(|db| {
                for day in ["2026-09-10", "2026-10-11", "2026-10-12", "2026-10-13"] {
                    stats::record(db, &stats::counted(day, 100))?;
                }
                insights(db, date("2026-10-13"))
            })
            .unwrap();
        assert_eq!(insights.total_words, 400);
        assert_eq!(insights.current_streak, 3);
        assert_eq!(insights.week_words, 200, "Monday 12 and Tuesday 13");
        assert_eq!(
            insights.month,
            Some(MonthCompare {
                current: 300,
                previous: 100
            })
        );
        assert_eq!(insights.days.len(), 4);
        assert_eq!(insights.top_app.as_deref(), Some("TextEdit"));
        assert_eq!(insights.apps_used, 1);
        assert_eq!(insights.wpm, 750, "100 words in 8 s");
    }
}
