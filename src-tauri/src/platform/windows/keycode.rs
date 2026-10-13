//! Windows virtual keys to handy-keys' `Key` and `Modifiers`, the names the
//! matcher and the stored shortcuts use, and back for keys Shrimp sends.
//! Punctuation is read from its scancode (the physical position), because
//! its virtual key moves with the layout.

use handy_keys::{Key, Modifiers};

pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_CAPITAL: u16 = 0x14;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;
pub const VK_LSHIFT: u16 = 0xA0;
pub const VK_RSHIFT: u16 = 0xA1;
pub const VK_LCONTROL: u16 = 0xA2;
pub const VK_RCONTROL: u16 = 0xA3;
pub const VK_LMENU: u16 = 0xA4;
pub const VK_RMENU: u16 = 0xA5;
/// Unassigned; pressed in place of a swallowed key so releasing Win or Alt
/// doesn't open Start or the menu bar.
pub const VK_MENU_MASK: u16 = 0xE8;

/// The Left Ctrl Windows adds around Right Alt on AltGr layouts.
const ALTGR_PHANTOM_CTRL_SCANCODE: u32 = 0x21D;

pub fn is_altgr_phantom_ctrl(vk: u16, scan_code: u32) -> bool {
    vk == VK_LCONTROL && scan_code == ALTGR_PHANTOM_CTRL_SCANCODE
}

pub fn modifier(vk: u16) -> Option<Modifiers> {
    Some(match vk {
        VK_LSHIFT | VK_SHIFT => Modifiers::SHIFT_LEFT,
        VK_RSHIFT => Modifiers::SHIFT_RIGHT,
        VK_LCONTROL | VK_CONTROL => Modifiers::CTRL_LEFT,
        VK_RCONTROL => Modifiers::CTRL_RIGHT,
        VK_LMENU | VK_MENU => Modifiers::OPT_LEFT,
        VK_RMENU => Modifiers::OPT_RIGHT,
        VK_LWIN => Modifiers::CMD_LEFT,
        VK_RWIN => Modifiers::CMD_RIGHT,
        _ => return None,
    })
}

/// Each side of each modifier with its virtual key.
pub const MODIFIER_KEYS: [(u16, Modifiers); 8] = [
    (VK_LCONTROL, Modifiers::CTRL_LEFT),
    (VK_RCONTROL, Modifiers::CTRL_RIGHT),
    (VK_LSHIFT, Modifiers::SHIFT_LEFT),
    (VK_RSHIFT, Modifiers::SHIFT_RIGHT),
    (VK_LMENU, Modifiers::OPT_LEFT),
    (VK_RMENU, Modifiers::OPT_RIGHT),
    (VK_LWIN, Modifiers::CMD_LEFT),
    (VK_RWIN, Modifiers::CMD_RIGHT),
];

const LETTERS: [Key; 26] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
];

const DIGITS: [Key; 10] = [
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

const KEYPAD_DIGITS: [Key; 10] = [
    Key::Keypad0,
    Key::Keypad1,
    Key::Keypad2,
    Key::Keypad3,
    Key::Keypad4,
    Key::Keypad5,
    Key::Keypad6,
    Key::Keypad7,
    Key::Keypad8,
    Key::Keypad9,
];

const FUNCTION_KEYS: [Key; 24] = [
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::F13,
    Key::F14,
    Key::F15,
    Key::F16,
    Key::F17,
    Key::F18,
    Key::F19,
    Key::F20,
    Key::F21,
    Key::F22,
    Key::F23,
    Key::F24,
];

/// Punctuation by Scan Code Set 1 position, named after the US legend.
const PUNCTUATION: [(u32, Key); 12] = [
    (0x29, Key::Grave),
    (0x0C, Key::Minus),
    (0x0D, Key::Equal),
    (0x1A, Key::LeftBracket),
    (0x1B, Key::RightBracket),
    (0x2B, Key::Backslash),
    (0x27, Key::Semicolon),
    (0x28, Key::Quote),
    (0x33, Key::Comma),
    (0x34, Key::Period),
    (0x35, Key::Slash),
    (0x56, Key::Section),
];

/// `VK_OEM_*` keys, whose meaning depends on the layout.
fn is_punctuation(vk: u16) -> bool {
    matches!(vk, 0xBA..=0xC0 | 0xDB..=0xDF | 0xE2)
}

/// The key of a keyboard event, or None for keys Shrimp doesn't bind.
pub fn key(vk: u16, scan_code: u32, extended: bool) -> Option<Key> {
    if !extended
        && is_punctuation(vk)
        && let Some((_, key)) = PUNCTUATION.iter().find(|(code, _)| *code == scan_code)
    {
        return Some(*key);
    }
    Some(match vk {
        0x41..=0x5A => LETTERS[usize::from(vk - 0x41)],
        0x30..=0x39 => DIGITS[usize::from(vk - 0x30)],
        0x60..=0x69 => KEYPAD_DIGITS[usize::from(vk - 0x60)],
        0x70..=0x87 => FUNCTION_KEYS[usize::from(vk - 0x70)],
        0x6A => Key::KeypadMultiply,
        0x6B => Key::KeypadPlus,
        0x6D => Key::KeypadMinus,
        0x6E => Key::KeypadDecimal,
        0x6F => Key::KeypadDivide,
        0x0C => Key::KeypadClear,
        VK_RETURN if extended => Key::KeypadEnter,
        VK_RETURN => Key::Return,
        VK_BACK => Key::Delete,
        0x2E => Key::ForwardDelete,
        VK_INSERT => Key::Insert,
        0x13 | 0x03 => Key::Pause,
        VK_TAB => Key::Tab,
        VK_ESCAPE => Key::Escape,
        VK_SPACE => Key::Space,
        0x21 => Key::PageUp,
        0x22 => Key::PageDown,
        0x23 => Key::End,
        0x24 => Key::Home,
        0x25 => Key::LeftArrow,
        0x26 => Key::UpArrow,
        0x27 => Key::RightArrow,
        0x28 => Key::DownArrow,
        0x2C => Key::PrintScreen,
        0x5D => Key::ContextMenu,
        0xB3 => Key::PlayPause,
        0xB2 => Key::Stop,
        0xB1 => Key::PrevTrack,
        0xB0 => Key::NextTrack,
        VK_CAPITAL => Key::CapsLock,
        0x90 => Key::NumLock,
        0x91 => Key::ScrollLock,
        // US meanings, for punctuation on a position the table doesn't know.
        0xBA => Key::Semicolon,
        0xBB => Key::Equal,
        0xBC => Key::Comma,
        0xBD => Key::Minus,
        0xBE => Key::Period,
        0xBF => Key::Slash,
        0xC0 | 0xDF => Key::Grave,
        0xDB => Key::LeftBracket,
        0xDC => Key::Backslash,
        0xDD => Key::RightBracket,
        0xDE => Key::Quote,
        0xE2 => Key::Section,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_function_keys() {
        assert_eq!(key(0x41, 0x1E, false), Some(Key::A));
        assert_eq!(key(0x5A, 0x2C, false), Some(Key::Z));
        assert_eq!(key(0x31, 0x02, false), Some(Key::Num1));
        assert_eq!(key(0x70, 0x3B, false), Some(Key::F1));
        assert_eq!(key(0x87, 0x76, false), Some(Key::F24));
        assert_eq!(key(VK_SPACE, 0x39, false), Some(Key::Space));
        assert_eq!(key(VK_RETURN, 0x1C, true), Some(Key::KeypadEnter));
        assert_eq!(key(VK_MENU_MASK, 0, false), None);
    }

    #[test]
    fn punctuation_follows_the_position() {
        // UK '@ sits where US '" is and reports VK_OEM_3.
        assert_eq!(key(0xC0, 0x28, false), Some(Key::Quote));
        assert_eq!(key(0xDF, 0x29, false), Some(Key::Grave));
        assert_eq!(key(0xBF, 0x35, true), Some(Key::Slash));
    }

    #[test]
    fn modifiers_by_side() {
        assert_eq!(modifier(VK_LWIN), Some(Modifiers::CMD_LEFT));
        assert_eq!(modifier(VK_RMENU), Some(Modifiers::OPT_RIGHT));
        assert_eq!(modifier(VK_CONTROL), Some(Modifiers::CTRL_LEFT));
        assert_eq!(modifier(0x41), None);
        assert!(is_altgr_phantom_ctrl(VK_LCONTROL, 0x21D));
        assert!(!is_altgr_phantom_ctrl(VK_LCONTROL, 0x1D));
    }
}
