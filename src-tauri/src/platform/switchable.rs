//! Permission states a debug build can pretend to have (`dev_set_permissions`),
//! so onboarding and the re-check after an update can be tested without
//! revoking the real grants. Requests and settings panes for a pretended
//! permission do nothing.

use std::sync::{Arc, Mutex};

use super::{Permission, PermissionStatus, Permissions, PlatformError};

pub struct SwitchablePermissions {
    real: Arc<dyn Permissions>,
    microphone: Mutex<Option<PermissionStatus>>,
    accessibility: Mutex<Option<PermissionStatus>>,
}

impl SwitchablePermissions {
    pub fn new(real: Arc<dyn Permissions>) -> Self {
        Self {
            real,
            microphone: Mutex::new(None),
            accessibility: Mutex::new(None),
        }
    }

    /// Reads `SHRIMP_PRETEND_PERMISSIONS`, e.g. `accessibility=denied,microphone=notDetermined`,
    /// so a launch can start with pretended states (the check after an update runs then).
    pub fn pretend_from_env(&self) {
        let Ok(spec) = std::env::var("SHRIMP_PRETEND_PERMISSIONS") else {
            return;
        };
        for pair in spec.split(',') {
            let Some((name, status)) = pair.split_once('=') else {
                continue;
            };
            let permission = serde_json::from_value(serde_json::json!(name.trim()));
            let status = serde_json::from_value(serde_json::json!(status.trim()));
            if let (Ok(permission), Ok(status)) = (permission, status) {
                self.pretend(permission, Some(status));
            }
        }
    }

    /// Pretends `permission` is `status`, or goes back to the real one with `None`.
    pub fn pretend(&self, permission: Permission, status: Option<PermissionStatus>) {
        *self
            .slot(permission)
            .lock()
            .unwrap_or_else(|err| err.into_inner()) = status;
    }

    fn slot(&self, permission: Permission) -> &Mutex<Option<PermissionStatus>> {
        match permission {
            Permission::Microphone => &self.microphone,
            Permission::Accessibility => &self.accessibility,
        }
    }

    fn pretended(&self, permission: Permission) -> Option<PermissionStatus> {
        *self
            .slot(permission)
            .lock()
            .unwrap_or_else(|err| err.into_inner())
    }
}

impl Permissions for SwitchablePermissions {
    fn status(&self, permission: Permission) -> PermissionStatus {
        self.pretended(permission)
            .unwrap_or_else(|| self.real.status(permission))
    }

    fn request(&self, permission: Permission) -> Result<(), PlatformError> {
        match self.pretended(permission) {
            Some(_) => {
                log::info!("pretended {permission:?}: no request");
                Ok(())
            }
            None => self.real.request(permission),
        }
    }

    fn open_settings(&self, permission: Permission) -> Result<(), PlatformError> {
        match self.pretended(permission) {
            Some(_) => {
                log::info!("pretended {permission:?}: System Settings not opened");
                Ok(())
            }
            None => self.real.open_settings(permission),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Granted;

    impl Permissions for Granted {
        fn status(&self, _: Permission) -> PermissionStatus {
            PermissionStatus::Granted
        }

        fn request(&self, _: Permission) -> Result<(), PlatformError> {
            Err(PlatformError::Unsupported)
        }

        fn open_settings(&self, _: Permission) -> Result<(), PlatformError> {
            Err(PlatformError::Unsupported)
        }
    }

    #[test]
    fn pretends_until_told_to_stop() {
        let permissions = SwitchablePermissions::new(Arc::new(Granted));
        permissions.pretend(Permission::Accessibility, Some(PermissionStatus::Denied));
        assert_eq!(
            permissions.status(Permission::Accessibility),
            PermissionStatus::Denied
        );
        assert!(permissions.open_settings(Permission::Accessibility).is_ok());
        assert_eq!(
            permissions.status(Permission::Microphone),
            PermissionStatus::Granted
        );
        permissions.pretend(Permission::Accessibility, None);
        assert_eq!(
            permissions.status(Permission::Accessibility),
            PermissionStatus::Granted
        );
        assert!(
            permissions
                .open_settings(Permission::Accessibility)
                .is_err()
        );
    }
}
