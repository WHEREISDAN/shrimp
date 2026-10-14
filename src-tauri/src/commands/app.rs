use serde::Serialize;
use tauri::{AppHandle, PackageInfo, State};

use crate::app::window::WindowMaterial;

/// The user's first name, read once at startup.
pub struct UserName(pub Option<String>);

/// Mirrors `AppInfo` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: &'static str,
    pub arch: &'static str,
    /// For Home's greeting.
    pub user_name: Option<String>,
}

impl AppInfo {
    fn from_package(package: &PackageInfo, user_name: Option<String>) -> Self {
        Self {
            name: package.name.clone(),
            version: package.version.to_string(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            user_name,
        }
    }
}

#[tauri::command]
pub fn app_info(app: AppHandle, user: State<'_, UserName>) -> AppInfo {
    AppInfo::from_package(app.package_info(), user.0.clone())
}

/// The native material behind the main window's sidebar: `vibrancy`, `mica` or `none`.
#[tauri::command]
pub fn window_material() -> WindowMaterial {
    crate::app::window::material()
}

/// The main window says whether one of its text fields has focus, so a
/// dictation into Shrimp itself (onboarding's practice field) is pasted.
#[tauri::command]
pub fn own_field_focused(focused: bool) {
    crate::platform::set_own_field_focused(focused);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_serializes_in_camel_case_for_the_frontend() {
        let info = AppInfo {
            name: "Shrimp".into(),
            version: "0.1.0".into(),
            os: "macos",
            arch: "aarch64",
            user_name: Some("Dan".into()),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"name": "Shrimp", "version": "0.1.0", "os": "macos",
                "arch": "aarch64", "userName": "Dan"})
        );
    }
}
