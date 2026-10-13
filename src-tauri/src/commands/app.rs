use serde::Serialize;
use tauri::{AppHandle, PackageInfo};

/// Mirrors `AppInfo` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: &'static str,
    pub arch: &'static str,
}

impl AppInfo {
    fn from_package(package: &PackageInfo) -> Self {
        Self {
            name: package.name.clone(),
            version: package.version.to_string(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        }
    }
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    AppInfo::from_package(app.package_info())
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
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"name": "Shrimp", "version": "0.1.0", "os": "macos", "arch": "aarch64"})
        );
    }
}
