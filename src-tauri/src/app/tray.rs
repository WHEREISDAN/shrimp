use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, include_image};

use super::events::{NAVIGATE_EVENT, TauriUi};
use super::{updates, window};
use crate::commands::system;
use crate::dictation::{ControllerHandle, ControllerInput, DictationUi};
use crate::hotkeys::Action;
use crate::notify::Notice;
use crate::platform::{TRAY_ACCELERATORS, TRAY_CLICK_OPENS_WINDOW};
use crate::settings::{Settings, SettingsStore};
use crate::t;

const TRAY_ID: &str = "main";
/// macOS: a black template image, which the menu bar tints.
#[cfg(not(target_os = "windows"))]
const TRAY_ICON: Image<'_> = include_image!("icons/tray.png");
/// Windows: the coral mark, which reads on light and dark taskbars.
#[cfg(target_os = "windows")]
const TRAY_ICON: Image<'_> = include_image!("icons/tray-windows.png");
/// Privacy mode's dot: black in the macOS template, green on Windows' coloured icon.
const PRIVACY_DOT: [u8; 3] = if cfg!(target_os = "windows") {
    [48, 209, 88]
} else {
    [0, 0, 0]
};
const OPEN_ID: &str = "open";
pub const SHORTCUTS_ID: &str = "shortcuts";
const PASTE_LAST_ID: &str = "paste-last";
pub const COPY_LAST_ID: &str = "copy-last";
const PRIVACY_ID: &str = "privacy-mode";
const RESTART_ID: &str = "restart";
pub const QUIT_ID: &str = "quit";

/// Paste last transcript is greyed out until there is one (WIN-09).
static PASTE_LAST_AVAILABLE: AtomicBool = AtomicBool::new(false);
/// The last transcript has text, for the menu bar's Copy last transcript (APP-10).
static COPY_LAST_AVAILABLE: AtomicBool = AtomicBool::new(false);

pub fn create<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> tauri::Result<()> {
    let privacy = settings.privacy.privacy_mode;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(privacy))
        .icon_as_template(true)
        .tooltip(tooltip(privacy))
        .menu(&menu(app, settings)?)
        .show_menu_on_left_click(!TRAY_CLICK_OPENS_WINDOW)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_ID => window::show_main(app),
            SHORTCUTS_ID => {
                window::show_main(app);
                let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "shortcuts");
            }
            PASTE_LAST_ID => send(app, Action::PasteLast),
            COPY_LAST_ID => send(app, Action::CopyLast),
            PRIVACY_ID => toggle_privacy_mode(app),
            updates::MENU_ID => updates::menu_clicked(app),
            RESTART_ID => restart(app),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && TRAY_CLICK_OPENS_WINDOW
            {
                window::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Shows Privacy mode in the menu, the icon and the tooltip (PRIV-03). The
/// menu bar shares the menu's texts and states, so it follows.
pub fn refresh<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    #[cfg(target_os = "macos")]
    super::menu_bar::refresh(app);
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let privacy = settings.privacy.privacy_mode;
    let updated = menu(app, settings)
        .and_then(|menu| tray.set_menu(Some(menu)))
        .and_then(|()| tray.set_icon_with_as_template(Some(icon(privacy)), true))
        .and_then(|()| tray.set_tooltip(Some(tooltip(privacy))));
    if let Err(error) = updated {
        log::warn!("could not update the tray: {error}");
    }
}

/// Enables or greys out Paste last transcript, and the menu bar's Copy last
/// transcript.
pub fn set_last_transcript<R: Runtime>(app: &AppHandle<R>, paste: bool, copy: bool) {
    let paste_changed = PASTE_LAST_AVAILABLE.swap(paste, Ordering::Relaxed) != paste;
    let copy_changed = COPY_LAST_AVAILABLE.swap(copy, Ordering::Relaxed) != copy;
    if !paste_changed && !copy_changed {
        return;
    }
    if let Some(settings) = app.try_state::<Arc<SettingsStore>>() {
        let settings = settings.get();
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || refresh(&handle, &settings));
    }
}

fn menu<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> tauri::Result<Menu<R>> {
    let item = |id, text, enabled, accelerator: Option<&str>| {
        MenuItem::with_id(app, id, text, enabled, accelerator)
    };
    let [paste_keys, copy_keys, quit_keys] = TRAY_ACCELERATORS;
    let paste_available = PASTE_LAST_AVAILABLE.load(Ordering::Relaxed);
    log::debug!("tray menu: paste last transcript enabled: {paste_available}");
    let open = item(OPEN_ID, t!("tray.open"), true, None)?;
    let shortcuts = item(SHORTCUTS_ID, t!("tray.shortcuts"), true, None)?;
    let update = updates::menu_item(app)?;
    let paste_last = item(
        PASTE_LAST_ID,
        t!("tray.pasteLast"),
        paste_available,
        paste_keys,
    )?;
    let copy_last = item(COPY_LAST_ID, t!("tray.copyLast"), true, copy_keys)?;
    let privacy = CheckMenuItem::with_id(
        app,
        PRIVACY_ID,
        t!("tray.privacyMode"),
        true,
        settings.privacy.privacy_mode,
        None::<&str>,
    )?;
    let restart = item(RESTART_ID, t!("tray.restart"), true, None)?;
    let quit = item(QUIT_ID, t!("tray.quit"), true, quit_keys)?;
    Menu::with_items(
        app,
        &[
            &open,
            &shortcuts,
            &update,
            &PredefinedMenuItem::separator(app)?,
            &paste_last,
            &copy_last,
            &PredefinedMenuItem::separator(app)?,
            &privacy,
            &PredefinedMenuItem::separator(app)?,
            &restart,
            &quit,
        ],
    )
}

/// Whether the last transcript has text to copy.
#[cfg(target_os = "macos")]
pub fn copy_last_available() -> bool {
    COPY_LAST_AVAILABLE.load(Ordering::Relaxed)
}

fn tooltip(privacy: bool) -> String {
    if privacy {
        t!("tray.privacyTooltip")
    } else {
        "Shrimp".into()
    }
}

/// The tray icon, with a dot in the corner while Privacy mode is on.
fn icon(privacy: bool) -> Image<'static> {
    let (width, height) = (TRAY_ICON.width(), TRAY_ICON.height());
    let mut rgba = TRAY_ICON.rgba().to_vec();
    if privacy {
        let size = width.min(height) as f32;
        let (cx, cy) = (size * 0.8, size * 0.8);
        for y in 0..height {
            for x in 0..width {
                let distance = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy);
                let alpha = if distance <= size * 0.16 {
                    255
                } else if distance <= size * 0.22 {
                    0
                } else {
                    continue;
                };
                let at = ((y * width + x) * 4) as usize;
                let [r, g, b] = PRIVACY_DOT;
                rgba[at..at + 4].copy_from_slice(&[r, g, b, alpha]);
            }
        }
    }
    // Windows shrinks a larger icon to the taskbar's size by sampling pixels,
    // which frays the mark and the rounded corners.
    #[cfg(target_os = "windows")]
    if let Some(size) = crate::platform::small_icon_size().filter(|size| *size < width)
        && width == height
    {
        return Image::new_owned(shrink(&rgba, width, size), size, size);
    }
    Image::new_owned(rgba, width, height)
}

/// Shrinks a square RGBA image to `size` x `size`, each pixel the average of
/// the source area it covers, its colour weighted by alpha so transparent
/// pixels darken nothing.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn shrink(rgba: &[u8], from: u32, size: u32) -> Vec<u8> {
    let scale = from as f32 / size as f32;
    let span = |at: u32| (at as f32 * scale, (at + 1) as f32 * scale);
    let mut out = vec![0; (size * size * 4) as usize];
    for y in 0..size {
        let (top, bottom) = span(y);
        for x in 0..size {
            let (left, right) = span(x);
            let (mut colour, mut alpha, mut area) = ([0f32; 3], 0f32, 0f32);
            for sy in top.floor() as u32..(bottom.ceil() as u32).min(from) {
                let high = bottom.min(sy as f32 + 1.0) - top.max(sy as f32);
                for sx in left.floor() as u32..(right.ceil() as u32).min(from) {
                    let weight = high * (right.min(sx as f32 + 1.0) - left.max(sx as f32));
                    let at = ((sy * from + sx) * 4) as usize;
                    let opacity = f32::from(rgba[at + 3]) / 255.0 * weight;
                    for (sum, channel) in colour.iter_mut().zip(&rgba[at..at + 3]) {
                        *sum += f32::from(*channel) * opacity;
                    }
                    alpha += opacity;
                    area += weight;
                }
            }
            let at = ((y * size + x) * 4) as usize;
            if alpha > 0.0 {
                for (target, sum) in out[at..at + 3].iter_mut().zip(colour) {
                    *target = (sum / alpha).round() as u8;
                }
            }
            out[at + 3] = (alpha / area * 255.0).round() as u8;
        }
    }
    out
}

fn toggle_privacy_mode<R: Runtime>(app: &AppHandle<R>) {
    let Some(settings) = app.try_state::<Arc<SettingsStore>>() else {
        return;
    };
    let on = !settings.get().privacy.privacy_mode;
    if let Err(error) = settings.update(&serde_json::json!({"privacy": {"privacyMode": on}})) {
        log::warn!("could not switch Privacy mode: {error}");
    }
}

/// "Restart Shrimp" (APP-12): a normal quit, so Whisper unloads and no crash
/// screen follows, then the app opens again with its main window, without
/// asking first (Wispr Flow's Restart Flow, article 7492559320).
fn restart<R: Runtime>(app: &AppHandle<R>) {
    let Err(error) = system::restart(app) else {
        return;
    };
    log::error!("could not restart: {}", error.message);
    if let Some(ui) = app.try_state::<Arc<TauriUi>>() {
        ui.notice(Notice::error(t!(
            "notice.restartFailed",
            error = error.message
        )));
    }
}

fn send<R: Runtime>(app: &AppHandle<R>, action: Action) {
    match app.try_state::<ControllerHandle>() {
        Some(controller) => controller.send(ControllerInput::Menu(action)),
        None => log::warn!("dictation is not running"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * width + x) * 4) as usize;
        rgba[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn shrinking_averages_without_darkening_edges() {
        // 4 x 4: the left half opaque coral, the right half transparent black.
        let coral = [240, 90, 56, 255];
        let rgba: Vec<u8> = (0..16)
            .flat_map(|i| if i % 4 < 2 { coral } else { [0, 0, 0, 0] })
            .collect();
        let half = shrink(&rgba, 4, 2);
        assert_eq!(pixel(&half, 2, 0, 0), coral);
        assert_eq!(pixel(&half, 2, 1, 1)[3], 0);

        // Thirds: the middle pixel covers both halves equally, still coral.
        let third = shrink(&rgba, 4, 3);
        let [r, g, b, a] = pixel(&third, 3, 1, 1);
        assert_eq!([r, g, b], [240, 90, 56]);
        assert!((127..=128).contains(&a), "{a}");
        assert_eq!(pixel(&third, 3, 0, 2), coral);
    }

    #[test]
    fn the_windows_tray_icon_shrinks_to_a_taskbar_size() {
        let image: Image<'_> = include_image!("icons/tray-windows.png");
        for size in [16, 20, 24] {
            let small = shrink(image.rgba(), image.width(), size);
            assert_eq!(small.len(), (size * size * 4) as usize);
            // The coral square fills the middle; its rounded corners stay clear.
            assert_eq!(pixel(&small, size, size / 2, size - 2)[3], 255);
            assert!(pixel(&small, size, 0, 0)[3] < 64);
        }
    }
}
