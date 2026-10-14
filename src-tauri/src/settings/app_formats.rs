//! Style › Formatting per app (FMT-15): a transcription format for an app,
//! keyed by the identity dictations use (the bundle ID on macOS, the
//! executable name on Windows).

use serde::{Deserialize, Serialize};

use crate::edit::format::TranscriptFormat;
use crate::platform::AppIdentity;
use crate::t;

/// At most this many apps have a format of their own.
pub const MAX_APP_FORMATS: usize = 100;
const MAX_ID_CHARS: usize = 255;

/// One app's format. Mirrors `AppFormat` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppFormat {
    /// The bundle ID (macOS) or executable name (Windows).
    pub app_id: String,
    /// The app's name when it was added, for the list.
    pub app_name: String,
    pub format: TranscriptFormat,
}

/// The format dictations into `app` get; automatic when none is set.
pub fn format_for(formats: &[AppFormat], app: Option<&AppIdentity>) -> TranscriptFormat {
    let Some(app) = app else {
        return TranscriptFormat::Automatic;
    };
    formats
        .iter()
        .find(|entry| entry.app_id.eq_ignore_ascii_case(&app.bundle_id))
        .map_or(TranscriptFormat::Automatic, |entry| entry.format)
}

pub fn validate(formats: &[AppFormat]) -> Result<(), String> {
    if formats.len() > MAX_APP_FORMATS {
        return Err(t!(
            "error.settings.tooManyAppFormats",
            max = MAX_APP_FORMATS
        ));
    }
    for (index, entry) in formats.iter().enumerate() {
        let id = entry.app_id.trim();
        if id.is_empty()
            || id.chars().count() > MAX_ID_CHARS
            || entry.app_name.chars().count() > MAX_ID_CHARS
        {
            return Err(t!("error.settings.appFormatApp"));
        }
        if formats[..index]
            .iter()
            .any(|other| other.app_id.eq_ignore_ascii_case(id))
        {
            return Err(t!("error.settings.appFormatTwice", app = entry.app_name));
        }
    }
    Ok(())
}

/// An app the Style page offers to add. Mirrors `KnownApp` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownApp {
    pub id: String,
    pub name: String,
}

/// The apps to offer: the one in front before Shrimp's window opened,
/// then the apps dictated into, most words first; each once, without
/// Shrimp itself.
pub fn candidates(
    previous: Option<AppIdentity>,
    dictated: impl IntoIterator<Item = (String, String)>,
    own_ids: &[String],
) -> Vec<KnownApp> {
    let previous = previous.map(|app| (app.bundle_id, app.name));
    let mut apps: Vec<KnownApp> = Vec::new();
    for (id, name) in previous.into_iter().chain(dictated) {
        let id = id.trim().to_owned();
        let own = own_ids.iter().any(|own| own.eq_ignore_ascii_case(&id));
        if id.is_empty() || own || apps.iter().any(|app| app.id.eq_ignore_ascii_case(&id)) {
            continue;
        }
        let name = if name.trim().is_empty() {
            id.clone()
        } else {
            name
        };
        apps.push(KnownApp { id, name });
    }
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(app_id: &str, format: TranscriptFormat) -> AppFormat {
        AppFormat {
            app_id: app_id.into(),
            app_name: app_id.into(),
            format,
        }
    }

    fn app(bundle_id: &str) -> AppIdentity {
        AppIdentity {
            bundle_id: bundle_id.into(),
            name: String::new(),
        }
    }

    #[test]
    fn an_app_gets_its_own_format_and_others_automatic() {
        let formats = [
            entry("com.apple.Terminal", TranscriptFormat::Unformatted),
            entry("slack.exe", TranscriptFormat::Lowercase),
        ];
        assert_eq!(
            format_for(&formats, Some(&app("com.apple.Terminal"))),
            TranscriptFormat::Unformatted
        );
        assert_eq!(
            format_for(&formats, Some(&app("Slack.exe"))),
            TranscriptFormat::Lowercase,
            "Windows names match in any case"
        );
        assert_eq!(
            format_for(&formats, Some(&app("com.apple.TextEdit"))),
            TranscriptFormat::Automatic
        );
        assert_eq!(format_for(&formats, None), TranscriptFormat::Automatic);
    }

    #[test]
    fn rejects_blank_duplicate_and_too_many_apps() {
        assert!(validate(&[entry("a", TranscriptFormat::Lowercase)]).is_ok());
        assert!(validate(&[entry(" ", TranscriptFormat::Lowercase)]).is_err());
        assert!(
            validate(&[
                entry("com.app.A", TranscriptFormat::Lowercase),
                entry("COM.APP.A", TranscriptFormat::ProperNouns),
            ])
            .is_err()
        );
        let many: Vec<AppFormat> = (0..=MAX_APP_FORMATS)
            .map(|i| entry(&format!("app{i}"), TranscriptFormat::Lowercase))
            .collect();
        assert!(validate(&many).is_err());
    }

    #[test]
    fn offers_the_previous_app_then_dictated_apps_once_without_shrimp() {
        let previous = AppIdentity {
            bundle_id: "com.apple.Notes".into(),
            name: "Notes".into(),
        };
        let dictated = [
            ("com.tinyspeck.slackmacgap".into(), "Slack".into()),
            ("com.apple.notes".into(), "Notes".into()),
            ("com.shrimp.desktop".into(), "Shrimp".into()),
            ("".into(), "Unknown".into()),
            ("notepad.exe".into(), "".into()),
        ];
        let offered = candidates(Some(previous), dictated, &["com.shrimp.desktop".into()]);
        let ids: Vec<&str> = offered.iter().map(|app| app.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "com.apple.Notes",
                "com.tinyspeck.slackmacgap",
                "notepad.exe"
            ]
        );
        assert_eq!(offered[2].name, "notepad.exe");
    }

    #[test]
    fn serializes_for_the_frontend() {
        let json = serde_json::to_value(entry("com.apple.Terminal", TranscriptFormat::ProperNouns))
            .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "appId": "com.apple.Terminal",
                "appName": "com.apple.Terminal",
                "format": "properNouns"
            })
        );
        let known = KnownApp {
            id: "notepad.exe".into(),
            name: "Notepad".into(),
        };
        assert_eq!(
            serde_json::to_value(known).unwrap(),
            serde_json::json!({"id": "notepad.exe", "name": "Notepad"})
        );
    }
}
