//! Windows backend. Nothing asks for permission to hook the keyboard, send
//! keys or use UI Automation, but none of them reaches an elevated app from
//! a normal process (UIPI). The microphone follows Settings › Privacy.

mod audio;
mod clipboard;
mod context;
pub mod flow_bar;
mod focus;
#[cfg(feature = "vulkan")]
mod gpu;
mod hook;
mod hotkeys;
mod input;
mod insert;
mod keycode;
mod login;
pub mod main_window;
mod media;
mod mice;
mod permissions;
mod process;
mod sound;
mod system;
mod uia;

use std::sync::Arc;

use super::{GlobeKeyAction, MainThread, Platform};
use crate::hotkeys::{windows_defaults, windows_label, windows_reserved};

pub fn platform(_main_thread: MainThread) -> Platform {
    Platform {
        hotkeys: Box::new(hotkeys::WinHotkeys::default()),
        inserter: Arc::new(insert::WinInserter),
        focus: Arc::new(focus::WinFocus),
        permissions: Arc::new(permissions::WinPermissions),
        sounds: Arc::new(sound::WinCuePlayer),
        launcher: Arc::new(system::WinLauncher),
        devices: Arc::new(audio::WasapiDevices),
        media: Arc::new(media::WinMedia),
        default_bindings: windows_defaults(),
        label: windows_label,
        reserved: windows_reserved,
        language: system::preferred_language(),
        globe_key: GlobeKeyAction::DoNothing,
        user_name: system::first_name(),
        login_item: Arc::new(login::WinLoginItem),
        region: system::region(),
        mice: Arc::new(mice::RawInputMice),
    }
}

pub use focus::set_own_field_focused;
pub use system::{
    free_bytes, is_windows_11, language_name, move_to_applications, open_keyboard_settings,
    open_sound_settings, open_storage_settings, outside_applications, relaunch_after_exit,
};
