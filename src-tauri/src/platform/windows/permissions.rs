//! Windows asks no permission for keyboard hooks, sending keys or UI
//! Automation, so Accessibility always counts as granted. The microphone
//! follows Settings › Privacy › Microphone: the device switch, "Let apps
//! access your microphone" and "Let desktop apps access your microphone".
//! Desktop apps get no prompt; a denied switch is turned on in Settings.

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::{HSTRING, w};

use super::system::open_settings_page;
use crate::platform::{Permission, PermissionStatus, Permissions, PlatformError};

const CONSENT: &str =
    r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

fn consent(root: HKEY, key: &str) -> Option<String> {
    let mut buffer = [0u16; 32];
    let mut size = std::mem::size_of_val(&buffer) as u32;
    let status = unsafe {
        RegGetValueW(
            root,
            &HSTRING::from(key),
            w!("Value"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let units = size as usize / 2;
    Some(
        String::from_utf16_lossy(&buffer[..units.min(buffer.len())])
            .trim_end_matches('\0')
            .to_owned(),
    )
}

pub fn microphone_status() -> PermissionStatus {
    let switches = [
        consent(HKEY_LOCAL_MACHINE, CONSENT),
        consent(HKEY_CURRENT_USER, CONSENT),
        consent(HKEY_CURRENT_USER, &format!(r"{CONSENT}\NonPackaged")),
    ];
    if switches
        .iter()
        .flatten()
        .any(|value| value.eq_ignore_ascii_case("Deny"))
    {
        PermissionStatus::Denied
    } else {
        PermissionStatus::Granted
    }
}

pub struct WinPermissions;

impl Permissions for WinPermissions {
    fn status(&self, permission: Permission) -> PermissionStatus {
        match permission {
            Permission::Microphone => microphone_status(),
            Permission::Accessibility => PermissionStatus::Granted,
        }
    }

    fn request(&self, permission: Permission) -> Result<(), PlatformError> {
        match permission {
            Permission::Microphone if microphone_status() == PermissionStatus::Denied => {
                self.open_settings(permission)
            }
            _ => Ok(()),
        }
    }

    fn open_settings(&self, permission: Permission) -> Result<(), PlatformError> {
        match permission {
            Permission::Microphone => open_settings_page("privacy-microphone"),
            Permission::Accessibility => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessibility_is_always_granted() {
        assert_eq!(
            WinPermissions.status(Permission::Accessibility),
            PermissionStatus::Granted
        );
        // Reads the real switches; either answer is valid on a test machine.
        let _ = WinPermissions.status(Permission::Microphone);
    }
}
