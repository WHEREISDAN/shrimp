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
    ("wezterm-gui.exe", "WezTerm"),
    ("co.zeit.hyper", "Hyper"),
    ("Hyper.exe", "Hyper"),
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
    ("com.philandro.anydesk", "AnyDesk"),
    ("com.splashtop.stb.macosx", "Splashtop Business"),
    ("com.splashtop.stp.macosx", "Splashtop Personal"),
    ("com.parallels.client", "Parallels Client"),
    ("mstsc.exe", "Remote Desktop Connection"),
    ("msrdc.exe", "Remote Desktop"),
    ("msrdcw.exe", "Remote Desktop"),
    ("AnyDesk.exe", "AnyDesk"),
    ("TeamViewer.exe", "TeamViewer"),
    ("workspaces.exe", "Amazon WorkSpaces"),
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
    ("com.rdbrck.shift", "Shift"),
    ("chrome.exe", "Google Chrome"),
    ("msedge.exe", "Microsoft Edge"),
    ("firefox.exe", "Firefox"),
    ("brave.exe", "Brave"),
    ("Arc.exe", "Arc"),
    ("opera.exe", "Opera"),
    ("vivaldi.exe", "Vivaldi"),
    ("zen.exe", "Zen"),
    ("browser.exe", "Yandex"),
    ("Shift.exe", "Shift"),
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
    ("Code - Insiders.exe", "Visual Studio Code - Insiders"),
    ("VSCodium.exe", "VSCodium"),
    ("Zed.exe", "Zed"),
    ("sublime_text.exe", "Sublime Text"),
    ("devenv.exe", "Microsoft Visual Studio"),
    ("idea64.exe", "IntelliJ IDEA"),
    ("pycharm64.exe", "PyCharm"),
    ("webstorm64.exe", "WebStorm"),
    ("clion64.exe", "CLion"),
    ("goland64.exe", "GoLand"),
    ("rider64.exe", "JetBrains Rider"),
    ("phpstorm64.exe", "PhpStorm"),
    ("rubymine64.exe", "RubyMine"),
    ("studio64.exe", "Android Studio"),
];

/// Windows apps that read a pasted clipboard late, so it is held longer (WIN-04).
const LATE_CLIPBOARD_READERS: &[(&str, &str)] = &[
    ("OUTLOOK.EXE", "Microsoft Outlook"),
    ("olk.exe", "Outlook"),
    ("WINWORD.EXE", "Microsoft Word"),
    ("WhatsApp.exe", "WhatsApp"),
    ("WhatsApp.Root.exe", "WhatsApp"),
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

pub fn reads_clipboard_late(app: Option<&AppIdentity>) -> bool {
    app.is_some_and(|app| {
        LATE_CLIPBOARD_READERS
            .iter()
            .any(|(id, _)| app.bundle_id.eq_ignore_ascii_case(id))
    })
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

    #[test]
    fn remote_desktops_terminals_and_browsers_on_both_platforms() {
        let kind_of = |id: &str| kind(Some(&app(id, "")));
        for id in [
            "com.philandro.anydesk",
            "com.teamviewer.TeamViewer",
            "com.amazon.workspaces",
            "com.splashtop.stb.macosx",
            "com.splashtop.stp.macosx",
            "mstsc.exe",
            "msrdc.exe",
            "msrdcw.exe",
            "AnyDesk.exe",
            "TeamViewer.exe",
            "workspaces.exe",
        ] {
            assert_eq!(kind_of(id), AppKind::RemoteDesktop, "{id}");
        }
        for id in [
            "co.zeit.hyper",
            "Hyper.exe",
            "com.github.wez.wezterm",
            "wezterm-gui.exe",
            "powershell.exe",
            "pwsh.exe",
        ] {
            assert_eq!(kind_of(id), AppKind::Terminal, "{id}");
        }
        for id in [
            "com.google.Chrome",
            "com.apple.Safari",
            "company.thebrowser.Browser",
            "com.brave.Browser",
            "com.microsoft.edgemac",
            "org.mozilla.firefox",
            "com.operasoftware.Opera",
            "com.vivaldi.Vivaldi",
            "app.zen-browser.zen",
            "ru.yandex.desktop.yandex-browser",
            "com.rdbrck.shift",
            "chrome.exe",
            "msedge.exe",
            "firefox.exe",
            "brave.exe",
            "Arc.exe",
            "opera.exe",
            "vivaldi.exe",
            "zen.exe",
            "browser.exe",
            "Shift.exe",
        ] {
            assert_eq!(kind_of(id), AppKind::Browser, "{id}");
        }
    }

    #[test]
    fn windows_editors_and_late_clipboard_readers() {
        assert_eq!(
            kind(Some(&app("Code.exe", "Visual Studio Code"))),
            AppKind::CodeEditor
        );
        assert_eq!(
            kind(Some(&app("idea64.exe", "IntelliJ IDEA"))),
            AppKind::CodeEditor
        );
        assert_eq!(
            kind(Some(&app("WindowsTerminal.exe", "Windows Terminal"))),
            AppKind::Terminal
        );
        assert!(reads_clipboard_late(Some(&app(
            "WINWORD.EXE",
            "Microsoft Word"
        ))));
        assert!(reads_clipboard_late(Some(&app(
            "outlook.exe",
            "Microsoft Outlook"
        ))));
        assert!(!reads_clipboard_late(Some(&app("notepad.exe", "Notepad"))));
        assert!(
            !reads_clipboard_late(Some(&app("com.microsoft.Word", "Microsoft Word"))),
            "macOS keeps its hold"
        );
    }
}
