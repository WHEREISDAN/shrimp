use std::process::Command;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_application_services::AXIsProcessTrusted;
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

use crate::platform::{Permission, PermissionStatus, Permissions, PlatformError};

pub fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

fn microphone_status() -> PermissionStatus {
    let Some(audio) = (unsafe { AVMediaTypeAudio }) else {
        return PermissionStatus::Denied;
    };
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(audio) } {
        AVAuthorizationStatus::Authorized => PermissionStatus::Granted,
        AVAuthorizationStatus::NotDetermined => PermissionStatus::NotDetermined,
        _ => PermissionStatus::Denied,
    }
}

pub struct MacPermissions;

impl Permissions for MacPermissions {
    fn status(&self, permission: Permission) -> PermissionStatus {
        match permission {
            Permission::Microphone => microphone_status(),
            Permission::Accessibility if accessibility_trusted() => PermissionStatus::Granted,
            Permission::Accessibility => PermissionStatus::Denied,
        }
    }

    fn request(&self, permission: Permission) -> Result<(), PlatformError> {
        match permission {
            Permission::Microphone => {
                let audio = unsafe { AVMediaTypeAudio }.ok_or(PlatformError::Unsupported)?;
                let handler = RcBlock::new(|granted: Bool| {
                    log::info!("microphone access granted: {}", granted.as_bool());
                });
                unsafe {
                    AVCaptureDevice::requestAccessForMediaType_completionHandler(audio, &handler);
                }
                Ok(())
            }
            // macOS protects this pane; the user switches Shrimp on in Settings.
            Permission::Accessibility => self.open_settings(permission),
        }
    }

    fn open_settings(&self, permission: Permission) -> Result<(), PlatformError> {
        let pane = match permission {
            Permission::Microphone => "Privacy_Microphone",
            Permission::Accessibility => "Privacy_Accessibility",
        };
        Command::new("open")
            .arg(format!(
                "x-apple.systempreferences:com.apple.preference.security?{pane}"
            ))
            .spawn()
            .map(|_| ())
            .map_err(|err| PlatformError::Failed(format!("could not open System Settings: {err}")))
    }
}
