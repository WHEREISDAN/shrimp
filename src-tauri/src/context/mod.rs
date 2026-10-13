//! Context awareness (CTX-01..04): what surrounds the cursor when a dictation
//! starts, read in parallel with recording. It decides the category of web
//! apps (STY-05), feeds on-device casing and spacing (FMT-11), and, when the
//! user allows it, gives the speech engine and the AI edit the names and
//! terms on screen (CTX-02).

pub mod apps;
mod capture;
pub mod terms;
pub mod web;
pub mod words;

pub use apps::AppKind;
pub use capture::{PendingContext, capture, capture_for_command};

use crate::edit::{Target, style};
use crate::platform::{AppIdentity, Editable, FieldContext, FieldKind};

/// The text next to the insertion point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Surroundings<'a> {
    pub before: &'a str,
    pub after: &'a str,
    pub selected: &'a str,
}

/// Where a dictation goes and what surrounds it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DictationContext {
    pub app: Option<AppIdentity>,
    /// None until the read finishes, and when nothing could be read.
    pub field: Option<FieldContext>,
}

impl DictationContext {
    pub fn new(app: Option<AppIdentity>, field: Option<FieldContext>) -> Self {
        Self { app, field }
    }

    pub fn app_kind(&self) -> AppKind {
        apps::kind(self.app.as_ref())
    }

    pub fn field_kind(&self) -> FieldKind {
        match (self.app_kind(), &self.field) {
            (_, Some(field)) if field.kind == FieldKind::Secure => FieldKind::Secure,
            (AppKind::Terminal, _) => FieldKind::Terminal,
            (_, Some(field)) => field.kind,
            (_, None) => FieldKind::Text,
        }
    }

    /// The focused field, unless it must not be read: secure fields, browser
    /// address bars and anything behind a remote desktop (CTX-03, INS-10).
    pub fn readable_field(&self) -> Option<&FieldContext> {
        let field = self.field.as_ref()?;
        let kind = self.app_kind();
        let address_bar = kind == AppKind::Browser && !field.in_web_content;
        let unreadable = field.kind == FieldKind::Secure
            || field.editable == Editable::No
            || kind == AppKind::RemoteDesktop
            || address_bar;
        (!unreadable).then_some(field)
    }

    pub fn surroundings(&self) -> Option<Surroundings<'_>> {
        let field = self.readable_field()?;
        field.selection?;
        Some(Surroundings {
            before: &field.before,
            after: &field.after,
            selected: &field.selected_text,
        })
    }

    /// Short texts visible in the window, only while the focused field may
    /// be read: an address bar's suggestions or a password form stay unread.
    fn screen_text(&self) -> &[String] {
        self.readable_field()
            .map_or(&[], |field| field.screen_text.as_slice())
    }

    /// Names visible near the cursor and on screen (CTX-02). In code editors
    /// and terminals the window's labels are tool chrome, not people.
    pub fn names(&self) -> Vec<String> {
        let near = self
            .readable_field()
            .map(|field| {
                [
                    field.before.as_str(),
                    field.selected_text.as_str(),
                    field.after.as_str(),
                ]
            })
            .unwrap_or_default();
        let screen = match self.app_kind() {
            AppKind::CodeEditor | AppKind::Terminal => &[],
            _ => self.screen_text(),
        };
        terms::names(near.into_iter().chain(screen.iter().map(String::as_str)))
    }

    /// Identifiers in the code around the cursor (DEV-02).
    pub fn identifiers(&self) -> Vec<String> {
        match self.readable_field() {
            Some(field) if field.kind == FieldKind::Code => match &field.text {
                Some(text) => terms::identifiers(text),
                None => terms::identifiers(&format!("{}\n{}", field.before, field.after)),
            },
            _ => Vec::new(),
        }
    }

    /// File names visible in the window, such as open editor tabs (DEV-01).
    pub fn file_names(&self) -> Vec<String> {
        let title = self
            .field
            .as_ref()
            .and_then(|field| field.window_title.as_deref());
        terms::file_names(
            title
                .into_iter()
                .chain(self.screen_text().iter().map(String::as_str)),
        )
    }

    /// The app's category, or the website's in a browser (STY-05).
    pub fn target(&self) -> Target {
        let app_target = style::categorize(self.app.as_ref());
        if self.app_kind() != AppKind::Browser {
            return app_target;
        }
        self.field
            .as_ref()
            .and_then(|field| field.url.as_deref())
            .and_then(web::target_for_url)
            .unwrap_or(app_target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::AppCategory;
    use crate::platform::TextRange;

    fn app(bundle_id: &str, name: &str) -> Option<AppIdentity> {
        Some(AppIdentity {
            bundle_id: bundle_id.into(),
            name: name.into(),
        })
    }

    fn field(before: &str) -> FieldContext {
        FieldContext {
            editable: Editable::Yes,
            selection: Some(TextRange { start: 3, len: 0 }),
            before: before.into(),
            in_web_content: true,
            ..FieldContext::default()
        }
    }

    #[test]
    fn secure_fields_address_bars_and_remote_desktops_are_not_read() {
        let mut secure = field("hunter2");
        secure.kind = FieldKind::Secure;
        let context = DictationContext::new(app("com.apple.TextEdit", "TextEdit"), Some(secure));
        assert!(context.surroundings().is_none());
        assert_eq!(context.field_kind(), FieldKind::Secure);

        let mut address = field("https://exa");
        address.in_web_content = false;
        let context = DictationContext::new(app("com.apple.Safari", "Safari"), Some(address));
        assert!(context.surroundings().is_none());

        let context = DictationContext::new(
            app("com.apple.ScreenSharing", "Screen Sharing"),
            Some(field("Hello")),
        );
        assert!(context.surroundings().is_none());
        assert!(context.names().is_empty());

        let context = DictationContext::new(
            app("com.apple.TextEdit", "TextEdit"),
            Some(field("Dear Siobhan")),
        );
        assert_eq!(context.surroundings().unwrap().before, "Dear Siobhan");
        assert_eq!(context.names(), ["Siobhan"]);
    }

    #[test]
    fn the_website_decides_the_category_in_browsers() {
        let mut page = field("");
        page.url = Some("https://mail.google.com/mail/u/0/".into());
        let context = DictationContext::new(
            app("com.google.Chrome", "Google Chrome"),
            Some(page.clone()),
        );
        assert_eq!(context.target().category, AppCategory::Email);
        let context = DictationContext::new(app("com.apple.TextEdit", "TextEdit"), Some(page));
        assert_eq!(context.target().category, AppCategory::Other);
    }

    #[test]
    fn terminals_and_code_editors() {
        let context =
            DictationContext::new(app("com.apple.Terminal", "Terminal"), Some(field("% ")));
        assert_eq!(context.field_kind(), FieldKind::Terminal);
        let mut code = field("");
        code.kind = FieldKind::Code;
        code.text = Some("const userId = 1;".into());
        code.window_title = Some("scratch.ts — ide".into());
        let context =
            DictationContext::new(app("com.todesktop.230313mzl4w4u92", "Cursor"), Some(code));
        assert_eq!(context.identifiers(), ["userId"]);
        assert_eq!(context.file_names(), ["scratch.ts"]);
    }
}
