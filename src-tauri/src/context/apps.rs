//! Kinds of apps that change how Shrimp reads context and pastes: terminals
//! (INS-11), remote desktops (INS-10), browsers (STY-05) and code editors
//! (DEV-01..03). Matched by bundle ID (macOS) or executable name (Windows),
//! with the display name as a fallback.

use crate::platform::AppIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKind {
    Terminal,
    RemoteDesktop,
    Browser,
    CodeEditor,
    Other,
}

const TERMINALS: &[(&str, &str)] = &[
    ("com.apple.Terminal", "Terminal"),
    ("com.googlecode.iterm2", "iTerm2"),
    ("dev.warp.Warp-Stable", "Warp"),
    ("dev.warp.Warp", "Warp"),
    ("com.mitchellh.ghostty", "Ghostty"),
    ("net.kovidgoyal.kitty", "kitty"),
    ("org.alacritty", "Alacritty"),
    ("com.github.wez.wezterm", "WezTerm"),
    ("co.zeit.hyper", "Hyper"),
    ("com.raphaelamorim.rio", "Rio"),
    ("WindowsTerminal.exe", "Windows Terminal"),
    ("powershell.exe", "Windows PowerShell"),
    ("pwsh.exe", "PowerShell"),
    ("cmd.exe", "Command Prompt"),
];

const REMOTE_DESKTOPS: &[(&str, &str)] = &[
    ("com.microsoft.rdc.macos", "Windows App"),
    ("com.microsoft.rdc.mac", "Microsoft Remote Desktop"),
    ("com.citrix.receiver.nomas", "Citrix Workspace"),
    ("com.vmware.horizon", "VMware Horizon Client"),
    ("com.omnissa.horizon.client.mac", "Omnissa Horizon Client"),
    ("com.realvnc.vncviewer", "VNC Viewer"),
    ("com.apple.ScreenSharing", "Screen Sharing"),
    ("com.amazon.workspaces", "WorkSpaces"),
    ("com.teamviewer.TeamViewer", "TeamViewer"),
    ("com.splashtop.business", "Splashtop Business"),
    ("com.parallels.client", "Parallels Client"),
    ("mstsc.exe", "Remote Desktop Connection"),
    ("msrdc.exe", "Remote Desktop"),
    ("CDViewer.exe", "Citrix Workspace"),
    ("vmware-view.exe", "VMware Horizon Client"),
];

/// The browsers Wispr reads the website from.
const BROWSERS: &[(&str, &str)] = &[
    ("com.apple.Safari", "Safari"),
    (
        "com.apple.SafariTechnologyPreview",
        "Safari Technology Preview",
    ),
    ("com.google.Chrome", "Google Chrome"),
    ("com.google.Chrome.beta", "Google Chrome Beta"),
    ("com.google.Chrome.canary", "Google Chrome Canary"),
    ("company.thebrowser.Browser", "Arc"),
    ("com.brave.Browser", "Brave Browser"),
    ("com.microsoft.edgemac", "Microsoft Edge"),
    ("org.mozilla.firefox", "Firefox"),
    ("com.operasoftware.Opera", "Opera"),
    ("com.vivaldi.Vivaldi", "Vivaldi"),
    ("app.zen-browser.zen", "Zen"),
    ("ru.yandex.desktop.yandex-browser", "Yandex"),
    ("com.pushplaylabs.sidekick", "Sidekick"),
    ("com.tonsky.shift", "Shift"),
    ("chrome.exe", "Google Chrome"),
    ("msedge.exe", "Microsoft Edge"),
    ("firefox.exe", "Firefox"),
    ("brave.exe", "Brave"),
];

/// IDEs and code editors: developer apps for syntax-aware dictation.
const CODE_EDITORS: &[(&str, &str)] = &[
    ("com.todesktop.230313mzl4w4u92", "Cursor"),
    ("com.exafunction.windsurf", "Windsurf"),
    ("com.microsoft.VSCode", "Code"),
    ("com.microsoft.VSCodeInsiders", "Code - Insiders"),
    ("com.vscodium", "VSCodium"),
    ("dev.zed.Zed", "Zed"),
    ("com.apple.dt.Xcode", "Xcode"),
    ("com.sublimetext.4", "Sublime Text"),
    ("com.panic.Nova", "Nova"),
    ("com.google.android.studio", "Android Studio"),
    ("Cursor.exe", "Cursor"),
    ("Windsurf.exe", "Windsurf"),
    ("Code.exe", "Visual Studio Code"),
];

/// Editors whose AI chat takes @-mentions of files (DEV-01).
const FILE_TAGGING: &[(&str, &str)] = &[
    ("com.todesktop.230313mzl4w4u92", "Cursor"),
    ("com.exafunction.windsurf", "Windsurf"),
    ("Cursor.exe", "Cursor"),
    ("Windsurf.exe", "Windsurf"),
];

fn listed(app: &AppIdentity, list: &[(&str, &str)]) -> bool {
    list.iter().any(|(id, name)| {
        app.bundle_id.eq_ignore_ascii_case(id) || app.name.eq_ignore_ascii_case(name)
    })
}

pub fn kind(app: Option<&AppIdentity>) -> AppKind {
    let Some(app) = app else {
        return AppKind::Other;
    };
    if listed(app, TERMINALS) {
        AppKind::Terminal
    } else if listed(app, REMOTE_DESKTOPS) {
        AppKind::RemoteDesktop
    } else if listed(app, BROWSERS) {
        AppKind::Browser
    } else if listed(app, CODE_EDITORS) || app.bundle_id.starts_with("com.jetbrains.") {
        AppKind::CodeEditor
    } else {
        AppKind::Other
    }
}

pub fn has_file_tagging(app: Option<&AppIdentity>) -> bool {
    app.is_some_and(|app| listed(app, FILE_TAGGING))
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
    fn documented_terminals_are_terminals() {
        for (id, name) in [
            ("com.apple.Terminal", "Terminal"),
            ("com.googlecode.iterm2", "iTerm2"),
            ("dev.warp.Warp-Stable", "Warp"),
            ("com.mitchellh.ghostty", "Ghostty"),
            ("net.kovidgoyal.kitty", "kitty"),
            ("org.alacritty", "Alacritty"),
        ] {
            assert_eq!(kind(Some(&app(id, name))), AppKind::Terminal, "{name}");
        }
    }

    #[test]
    fn remote_desktops_browsers_and_editors() {
        assert_eq!(
            kind(Some(&app("com.apple.ScreenSharing", "Screen Sharing"))),
            AppKind::RemoteDesktop
        );
        assert_eq!(
            kind(Some(&app("x", "Citrix Workspace"))),
            AppKind::RemoteDesktop
        );
        assert_eq!(
            kind(Some(&app("com.microsoft.rdc.macos", "Windows App"))),
            AppKind::RemoteDesktop
        );
        assert_eq!(
            kind(Some(&app("app.zen-browser.zen", "Zen"))),
            AppKind::Browser
        );
        assert_eq!(
            kind(Some(&app("com.jetbrains.intellij", "IntelliJ IDEA"))),
            AppKind::CodeEditor
        );
        assert_eq!(
            kind(Some(&app("com.apple.TextEdit", "TextEdit"))),
            AppKind::Other
        );
        assert!(has_file_tagging(Some(&app(
            "com.todesktop.230313mzl4w4u92",
            "Cursor"
        ))));
        assert!(!has_file_tagging(Some(&app(
            "com.microsoft.VSCode",
            "Code"
        ))));
    }
}
