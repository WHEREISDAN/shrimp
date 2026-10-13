use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::CommandError;
use crate::platform::{Permission, PermissionStatus, Permissions};

pub struct AppPermissions(pub Arc<dyn Permissions>);

/// Mirrors `PermissionsInfo` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsInfo {
    pub microphone: PermissionStatus,
    pub accessibility: PermissionStatus,
}

#[tauri::command]
pub fn get_permissions(permissions: State<'_, AppPermissions>) -> PermissionsInfo {
    PermissionsInfo {
        microphone: permissions.0.status(Permission::Microphone),
        accessibility: permissions.0.status(Permission::Accessibility),
    }
}

/// Shows the OS prompt (microphone, first time) or the System Settings pane.
#[tauri::command]
pub fn open_permission_settings(
    permission: Permission,
    permissions: State<'_, AppPermissions>,
) -> Result<(), CommandError> {
    let result = match permissions.0.status(permission) {
        PermissionStatus::NotDetermined => permissions.0.request(permission),
        _ => permissions.0.open_settings(permission),
    };
    result.map_err(|err| CommandError::new("platform", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_statuses_in_camel_case() {
        let info = PermissionsInfo {
            microphone: PermissionStatus::NotDetermined,
            accessibility: PermissionStatus::Granted,
        };
        assert_eq!(
            serde_json::to_value(info).unwrap(),
            serde_json::json!({"microphone": "notDetermined", "accessibility": "granted"})
        );
    }
}
