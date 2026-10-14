//! What debug builds can pretend about the system, so features that depend
//! on it can be tried on any machine: the region (`SHRIMP_REGION=CA`,
//! LANG-07) and a mouse with extra buttons (`SHRIMP_FAKE_MOUSE_BUTTONS=5`,
//! CORE-18). `SHRIMP_NO_HOTKEYS=1` installs no keyboard or mouse hook, so a
//! dev app can run beside a release Shrimp that owns fn. See docs/DEVELOPMENT.md.

use std::sync::Arc;

use super::{InertHotkeys, MouseDevices, Platform};

const REGION_ENV: &str = "SHRIMP_REGION";
const MOUSE_BUTTONS_ENV: &str = "SHRIMP_FAKE_MOUSE_BUTTONS";
const NO_HOTKEYS_ENV: &str = "SHRIMP_NO_HOTKEYS";

/// One connected mouse with a fixed number of buttons.
struct FakeMouse(u32);

impl MouseDevices for FakeMouse {
    fn most_buttons(&self) -> u32 {
        self.0
    }
}

/// `platform` with what the environment pretends, in debug builds.
pub fn apply(mut platform: Platform) -> Platform {
    if !cfg!(debug_assertions) {
        return platform;
    }
    if let Some(region) = std::env::var(REGION_ENV)
        .ok()
        .and_then(|value| region(&value))
    {
        log::info!("pretending the region is {region}");
        platform.region = Some(region);
    }
    let buttons = std::env::var(MOUSE_BUTTONS_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    if let Some(buttons) = buttons {
        log::info!("pretending a mouse with {buttons} buttons");
        platform.mice = Arc::new(FakeMouse(buttons));
    }
    if std::env::var(NO_HOTKEYS_ENV).is_ok_and(|value| value.trim() == "1") {
        log::info!("global hotkeys are off ({NO_HOTKEYS_ENV}=1)");
        platform.hotkeys = Box::new(InertHotkeys);
    }
    platform
}

/// An ISO 3166 region code: two letters, upper case.
fn region(value: &str) -> Option<String> {
    let value = value.trim();
    (value.len() == 2 && value.chars().all(|c| c.is_ascii_alphabetic()))
        .then(|| value.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_are_two_letter_codes() {
        assert_eq!(region("ca").as_deref(), Some("CA"));
        assert_eq!(region(" GB ").as_deref(), Some("GB"));
        assert_eq!(region("CAN"), None);
        assert_eq!(region("419"), None);
        assert_eq!(region(""), None);
    }

    #[test]
    fn the_fake_mouse_reports_its_buttons() {
        assert_eq!(FakeMouse(5).most_buttons(), 5);
    }
}
