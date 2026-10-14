//! Keys Shrimp sends with `SendInput`: the paste shortcut, Enter, Select All,
//! Undo and Copy. Every event carries [`SHRIMP_TAG`] so Shrimp's own hooks let
//! it pass. Windows combines sent keys with the modifiers held at the time,
//! so modifiers the user still holds are lifted for the shortcut and pressed
//! again afterwards.

use handy_keys::Modifiers;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VIRTUAL_KEY,
};

use super::hook;
use super::keycode::{
    VK_INSERT, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU_MASK, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN,
};
use crate::platform::PlatformError;

/// Marks input Shrimp sends ("SHRP"), so its hooks tell it from the user's.
pub const SHRIMP_TAG: usize = 0x5348_5250;
/// How long Windows takes to pass sent keys through the hooks.
const RESTORE_DELAY: std::time::Duration = std::time::Duration::from_millis(20);

/// Keys whose scancode has the E0 prefix.
fn extended(vk: u16) -> bool {
    matches!(
        vk,
        VK_INSERT
            | 0x2E // Delete
            | 0x21
            ..=0x28 // Page Up/Down, End, Home, arrows
            | VK_RCONTROL
            | VK_RMENU
            | VK_LWIN
            | VK_RWIN
            | 0x5D // Apps
            | 0x6F // Numpad divide
            | 0x90 // Num Lock
    )
}

fn key(vk: u16, down: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    if extended(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: u16::try_from(scan).unwrap_or_default(),
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SHRIMP_TAG,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), PlatformError> {
    if inputs.is_empty() {
        return Ok(());
    }
    hook::note_sending();
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(PlatformError::Failed(format!(
            "Windows took {sent} of {} key events",
            inputs.len()
        )))
    }
}

/// An inert key press, so releasing Win or Alt doesn't open Start or the
/// active window's menu bar.
pub fn send_menu_mask() {
    if let Err(error) = send(&[key(VK_MENU_MASK, true), key(VK_MENU_MASK, false)]) {
        log::debug!("menu mask: {error}");
    }
}

/// The virtual keys of each side of `modifiers`.
fn modifier_keys(modifiers: Modifiers) -> Vec<u16> {
    [
        (Modifiers::CTRL_LEFT, VK_LCONTROL),
        (Modifiers::CTRL_RIGHT, VK_RCONTROL),
        (Modifiers::SHIFT_LEFT, VK_LSHIFT),
        (Modifiers::SHIFT_RIGHT, VK_RSHIFT),
        (Modifiers::OPT_LEFT, VK_LMENU),
        (Modifiers::OPT_RIGHT, VK_RMENU),
        (Modifiers::CMD_LEFT, VK_LWIN),
        (Modifiers::CMD_RIGHT, VK_RWIN),
    ]
    .into_iter()
    .filter(|(modifier, _)| modifiers.contains(*modifier))
    .map(|(_, vk)| vk)
    .collect()
}

/// One side of each modifier group in `modifiers`.
fn left_sides(modifiers: Modifiers) -> Modifiers {
    [
        (Modifiers::CTRL, Modifiers::CTRL_LEFT),
        (Modifiers::SHIFT, Modifiers::SHIFT_LEFT),
        (Modifiers::OPT, Modifiers::OPT_LEFT),
        (Modifiers::CMD, Modifiers::CMD_LEFT),
    ]
    .into_iter()
    .filter(|(group, _)| modifiers.intersects(*group))
    .fold(Modifiers::empty(), |acc, (_, left)| acc | left)
}

/// The events that press `vk` with exactly `modifiers` held, given that the
/// user holds `held`: lift the extra ones, press the wanted ones, the key,
/// and release those again. A wanted modifier the user holds is pressed
/// again too, as the user may let go of it before the key arrives. Returns
/// the events with the modifiers taken from the user, which go back down
/// afterwards if the user still holds them.
fn shortcut(held: Modifiers, modifiers: Modifiers, vk: u16) -> (Vec<INPUT>, Modifiers) {
    let wanted_groups = [
        Modifiers::CTRL,
        Modifiers::SHIFT,
        Modifiers::OPT,
        Modifiers::CMD,
    ]
    .into_iter()
    .filter(|group| modifiers.intersects(*group))
    .fold(Modifiers::empty(), |acc, group| acc | group);
    let lift = held & !wanted_groups;
    let renew = held & wanted_groups;
    let press = left_sides(modifiers & !held_groups(held)) | renew;
    let mut inputs = Vec::new();
    if lift.intersects(Modifiers::CMD | Modifiers::OPT) {
        inputs.extend([key(VK_MENU_MASK, true), key(VK_MENU_MASK, false)]);
    }
    inputs.extend(modifier_keys(lift).into_iter().map(|vk| key(vk, false)));
    inputs.extend(modifier_keys(press).into_iter().map(|vk| key(vk, true)));
    inputs.extend([key(vk, true), key(vk, false)]);
    inputs.extend(modifier_keys(press).into_iter().map(|vk| key(vk, false)));
    (inputs, lift | renew)
}

/// Presses the modifiers taken from the user again. After Win or Alt comes the
/// inert key: the user's release that follows would otherwise read as Win or
/// Alt pressed alone and open Start or the window's menu.
fn restore(lifted: Modifiers) -> Vec<INPUT> {
    let mut inputs: Vec<INPUT> = modifier_keys(lifted)
        .into_iter()
        .map(|vk| key(vk, true))
        .collect();
    if lifted.intersects(Modifiers::CMD | Modifiers::OPT) {
        inputs.extend([key(VK_MENU_MASK, true), key(VK_MENU_MASK, false)]);
    }
    inputs
}

fn held_groups(held: Modifiers) -> Modifiers {
    [
        Modifiers::CTRL,
        Modifiers::SHIFT,
        Modifiers::OPT,
        Modifiers::CMD,
    ]
    .into_iter()
    .filter(|group| held.intersects(*group))
    .fold(Modifiers::empty(), |acc, group| acc | group)
}

/// Presses `vk` with exactly `modifiers`, whatever the user holds. A modifier
/// taken from the user goes back down only if the user still holds it once the
/// keys went through: one released meanwhile would otherwise stay down in
/// Windows, and the user's next keys would become shortcuts.
pub fn press(modifiers: Modifiers, vk: u16) -> Result<(), PlatformError> {
    let (inputs, lifted) = shortcut(hook::modifiers_windows_holds(), modifiers, vk);
    send(&inputs)?;
    if lifted.is_empty() {
        return Ok(());
    }
    // The hooks see a release that came before these keys once Windows has
    // handled them.
    std::thread::sleep(RESTORE_DELAY);
    send(&restore(lifted & hook::modifiers_windows_holds()))
}

#[cfg(test)]
mod tests {
    use super::super::keycode::VK_RETURN;
    use super::*;

    fn keys(inputs: &[INPUT]) -> Vec<(u16, bool)> {
        inputs
            .iter()
            .map(|input| unsafe {
                let ki = input.Anonymous.ki;
                (ki.wVk.0, ki.dwFlags.0 & KEYEVENTF_KEYUP.0 == 0)
            })
            .collect()
    }

    /// Every event of a shortcut sent while the user keeps holding `held`.
    fn all(held: Modifiers, modifiers: Modifiers, vk: u16) -> Vec<(u16, bool)> {
        let (inputs, lifted) = shortcut(held, modifiers, vk);
        let mut events = keys(&inputs);
        events.extend(keys(&restore(lifted & held)));
        events
    }

    #[test]
    fn plain_ctrl_v() {
        assert_eq!(
            all(Modifiers::empty(), Modifiers::CTRL, 0x56),
            [
                (VK_LCONTROL, true),
                (0x56, true),
                (0x56, false),
                (VK_LCONTROL, false)
            ]
        );
    }

    #[test]
    fn a_held_ctrl_is_pressed_again_and_win_alt_are_lifted() {
        // Held Ctrl is pressed again, so the copy doesn't hang on the user's
        // finger, and goes back down afterwards.
        assert_eq!(
            all(Modifiers::CTRL_LEFT, Modifiers::CTRL, 0x43),
            [
                (VK_LCONTROL, true),
                (0x43, true),
                (0x43, false),
                (VK_LCONTROL, false),
                (VK_LCONTROL, true),
            ]
        );
        assert_eq!(
            all(
                Modifiers::CMD_LEFT | Modifiers::OPT_LEFT,
                Modifiers::CTRL,
                0x43
            ),
            [
                (VK_MENU_MASK, true),
                (VK_MENU_MASK, false),
                (VK_LMENU, false),
                (VK_LWIN, false),
                (VK_LCONTROL, true),
                (0x43, true),
                (0x43, false),
                (VK_LCONTROL, false),
                (VK_LMENU, true),
                (VK_LWIN, true),
                (VK_MENU_MASK, true),
                (VK_MENU_MASK, false),
            ]
        );
    }

    #[test]
    fn a_swallowed_win_is_neither_lifted_nor_pressed_again() {
        // Ctrl+Win held for push to talk: the hooks swallowed Win, so Windows
        // holds only Ctrl. Win is never sent; Ctrl is pressed for the paste and
        // lifted for Enter, then put back.
        let windows_holds = Modifiers::CTRL_LEFT;
        assert_eq!(
            all(windows_holds, Modifiers::CTRL, 0x56),
            [
                (VK_LCONTROL, true),
                (0x56, true),
                (0x56, false),
                (VK_LCONTROL, false),
                (VK_LCONTROL, true),
            ]
        );
        assert_eq!(
            all(windows_holds, Modifiers::empty(), VK_RETURN),
            [
                (VK_LCONTROL, false),
                (VK_RETURN, true),
                (VK_RETURN, false),
                (VK_LCONTROL, true),
            ]
        );
    }

    #[test]
    fn a_ctrl_released_before_the_paste_still_makes_it_ctrl_v() {
        let (inputs, taken) = shortcut(Modifiers::CTRL_LEFT, Modifiers::CTRL, 0x56);
        assert_eq!(
            keys(&inputs),
            [
                (VK_LCONTROL, true),
                (0x56, true),
                (0x56, false),
                (VK_LCONTROL, false),
            ]
        );
        assert!(restore(taken & Modifiers::empty()).is_empty());
    }
    #[test]
    fn a_modifier_released_meanwhile_stays_up() {
        // Alt+Enter for Press Enter, with Alt let go while Enter went out.
        let (inputs, lifted) = shortcut(Modifiers::OPT_LEFT, Modifiers::empty(), VK_RETURN);
        assert_eq!(
            keys(&inputs),
            [
                (VK_MENU_MASK, true),
                (VK_MENU_MASK, false),
                (VK_LMENU, false),
                (VK_RETURN, true),
                (VK_RETURN, false),
            ]
        );
        assert!(restore(lifted & Modifiers::empty()).is_empty());
        // Still held: Alt goes back down, and its release won't open the menu.
        assert_eq!(
            keys(&restore(lifted)),
            [
                (VK_LMENU, true),
                (VK_MENU_MASK, true),
                (VK_MENU_MASK, false)
            ]
        );
    }

    #[test]
    fn every_event_is_tagged() {
        let (inputs, lifted) = shortcut(Modifiers::SHIFT_LEFT, Modifiers::SHIFT, VK_INSERT);
        for input in inputs.into_iter().chain(restore(lifted)) {
            let ki = unsafe { input.Anonymous.ki };
            assert_eq!(ki.dwExtraInfo, SHRIMP_TAG);
            if ki.wVk.0 == VK_INSERT {
                assert!(ki.dwFlags.0 & KEYEVENTF_EXTENDEDKEY.0 != 0);
            }
        }
    }
}
