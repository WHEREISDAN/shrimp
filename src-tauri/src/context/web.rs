//! Web apps by domain (STY-05): in a browser, the site decides the category,
//! so Gmail formats like email and Slack like work messages. Writing sites
//! count as documents in Insights (STA-06).

use crate::edit::{AppCategory, Target};

/// (host, path prefix, category, AI assistant). A host matches itself and its
/// subdomains.
const SITES: &[(&str, &str, AppCategory, bool)] = &[
    ("mail.google.com", "/chat", AppCategory::Work, false),
    ("mail.google.com", "", AppCategory::Email, false),
    ("gmail.com", "", AppCategory::Email, false),
    ("outlook.live.com", "", AppCategory::Email, false),
    ("outlook.office.com", "", AppCategory::Email, false),
    ("outlook.office365.com", "", AppCategory::Email, false),
    ("outlook.cloud.microsoft", "", AppCategory::Email, false),
    ("mail.yahoo.com", "", AppCategory::Email, false),
    ("mail.proton.me", "", AppCategory::Email, false),
    ("app.fastmail.com", "", AppCategory::Email, false),
    ("mail.superhuman.com", "", AppCategory::Email, false),
    ("mail.missiveapp.com", "", AppCategory::Email, false),
    ("app.hey.com", "", AppCategory::Email, false),
    ("www.icloud.com", "/mail", AppCategory::Email, false),
    ("slack.com", "", AppCategory::Work, false),
    ("teams.microsoft.com", "", AppCategory::Work, false),
    ("teams.live.com", "", AppCategory::Work, false),
    ("teams.cloud.microsoft", "", AppCategory::Work, false),
    ("chat.google.com", "", AppCategory::Work, false),
    ("linkedin.com", "", AppCategory::Work, false),
    ("web.whatsapp.com", "", AppCategory::Personal, false),
    ("discord.com", "", AppCategory::Personal, false),
    ("messenger.com", "", AppCategory::Personal, false),
    (
        "www.facebook.com",
        "/messages",
        AppCategory::Personal,
        false,
    ),
    ("web.telegram.org", "", AppCategory::Personal, false),
    ("instagram.com", "", AppCategory::Personal, false),
    ("chatgpt.com", "", AppCategory::Other, true),
    ("chat.openai.com", "", AppCategory::Other, true),
    ("claude.ai", "", AppCategory::Other, true),
    ("perplexity.ai", "", AppCategory::Other, true),
    ("gemini.google.com", "", AppCategory::Other, true),
    ("aistudio.google.com", "", AppCategory::Other, true),
    ("copilot.microsoft.com", "", AppCategory::Other, true),
    ("chat.mistral.ai", "", AppCategory::Other, true),
    ("grok.com", "", AppCategory::Other, true),
    ("poe.com", "", AppCategory::Other, true),
];

/// Writing sites whose dictations Insights counts as documents (STA-06).
const DOCUMENT_SITES: &[&str] = &["docs.google.com", "notion.so", "obsidian.md"];

/// A host matches its site and the site's subdomains.
fn on_site(host: &str, site: &str) -> bool {
    host == site
        || host
            .strip_suffix(site)
            .is_some_and(|rest| rest.ends_with('.'))
}

/// The host and path of an http(s) URL, lowercased host.
pub fn host_and_path(url: &str) -> Option<(String, &str)> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let host = authority
        .rsplit('@')
        .next()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    (!host.is_empty()).then(|| (host, &rest[end..]))
}

/// The category of a web page, when its site is a known messaging, email or
/// AI app.
pub fn target_for_url(url: &str) -> Option<Target> {
    let (host, path) = host_and_path(url)?;
    SITES
        .iter()
        .find(|(site, prefix, _, _)| on_site(&host, site) && path.starts_with(prefix))
        .map(|(_, _, category, ai_app)| Target {
            category: *category,
            ai_app: *ai_app,
        })
}

/// Whether a web page is on a writing site such as Google Docs.
pub fn is_document_site(url: &str) -> bool {
    host_and_path(url)
        .is_some_and(|(host, _)| DOCUMENT_SITES.iter().any(|site| on_site(&host, site)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(url: &str) -> Option<AppCategory> {
        target_for_url(url).map(|target| target.category)
    }

    #[test]
    fn documented_sites_set_the_category() {
        assert_eq!(
            category("https://mail.google.com/mail/u/0/#inbox"),
            Some(AppCategory::Email)
        );
        assert_eq!(
            category("https://app.slack.com/client/T1/C2"),
            Some(AppCategory::Work)
        );
        assert_eq!(
            category("https://discord.com/login"),
            Some(AppCategory::Personal)
        );
        assert_eq!(
            category("https://mail.google.com/chat/u/0/"),
            Some(AppCategory::Work)
        );
        assert_eq!(
            category("https://www.linkedin.com/feed/"),
            Some(AppCategory::Work)
        );
        assert!(target_for_url("https://claude.ai/new").unwrap().ai_app);
        assert_eq!(category("https://example.com/"), None);
        assert_eq!(category("vscode-file://vscode-app/workbench.html"), None);
        assert_eq!(category("https://notslack.com/"), None);
    }

    #[test]
    fn writing_sites_are_documents() {
        assert!(is_document_site(
            "https://docs.google.com/document/d/1/edit"
        ));
        assert!(is_document_site("https://www.notion.so/Plans-123"));
        assert!(!is_document_site("https://mail.google.com/mail/u/0/"));
        assert!(!is_document_site("https://notnotion.so/"));
    }

    #[test]
    fn parses_hosts() {
        assert_eq!(
            host_and_path("https://User@App.Slack.com:443/client?x=1"),
            Some(("app.slack.com".into(), "/client?x=1"))
        );
        assert_eq!(host_and_path("file:///tmp/a.html"), None);
    }
}
