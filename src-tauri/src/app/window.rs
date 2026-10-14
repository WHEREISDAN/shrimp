use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, Window, WindowEvent};

use super::events::MAIN_VISIBLE_EVENT;
use crate::mic::Microphones;
use crate::platform::{AppIdentity, FocusInspector};
use crate::settings::SettingsStore;

pub const MAIN_LABEL: &str = "main";

/// The main window is open or minimized, not closed to the tray.
static MAIN_OPEN: AtomicBool = AtomicBool::new(false);

/// The app in front when Shrimp's window was last opened from another app
/// (the tray, the Flow Bar, a notice), which the Style page offers for a
/// format of its own (FMT-15).
pub struct PreviousApp {
    focus: Arc<dyn FocusInspector>,
    /// Shrimp's own identities: its bundle ID and executable name.
    own_ids: Vec<String>,
    app: Mutex<Option<AppIdentity>>,
}

impl PreviousApp {
    pub fn new(focus: Arc<dyn FocusInspector>, own_ids: Vec<String>) -> Self {
        Self {
            focus,
            own_ids,
            app: Mutex::new(None),
        }
    }

    fn remember(&self) {
        let Some(app) = self.focus.frontmost_app() else {
            return;
        };
        if self.is_own(&app.bundle_id) {
            return;
        }
        *self.app.lock().unwrap_or_else(|err| err.into_inner()) = Some(app);
    }

    pub fn get(&self) -> Option<AppIdentity> {
        self.app
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }

    pub fn own_ids(&self) -> &[String] {
        &self.own_ids
    }

    fn is_own(&self, id: &str) -> bool {
        self.own_ids.iter().any(|own| own.eq_ignore_ascii_case(id))
    }
}

/// The native material behind the main window's sidebar, which the page
/// leaves clear wherever it should show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowMaterial {
    /// macOS sidebar vibrancy, set up by tauri.macos.conf.json.
    Vibrancy,
    /// Windows 11.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Mica,
    /// Windows 10, which has no Mica: the page paints the sidebar itself.
    None,
}

static MATERIAL: OnceLock<WindowMaterial> = OnceLock::new();

/// Gives the main window its material, once at startup.
pub fn apply_material<R: Runtime>(app: &AppHandle<R>) {
    let material = if cfg!(target_os = "macos") {
        WindowMaterial::Vibrancy
    } else {
        mica(app)
    };
    let _ = MATERIAL.set(material);
}

pub fn material() -> WindowMaterial {
    MATERIAL.get().copied().unwrap_or(WindowMaterial::None)
}

#[cfg(target_os = "windows")]
fn mica<R: Runtime>(app: &AppHandle<R>) -> WindowMaterial {
    use tauri::window::{Effect, EffectsBuilder};

    if !crate::platform::is_windows_11() {
        return WindowMaterial::None;
    }
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return WindowMaterial::None;
    };
    match window.set_effects(EffectsBuilder::new().effect(Effect::Mica).build()) {
        Ok(()) => WindowMaterial::Mica,
        Err(err) => {
            log::warn!("failed to apply Mica: {err}");
            WindowMaterial::None
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn mica<R: Runtime>(_app: &AppHandle<R>) -> WindowMaterial {
    WindowMaterial::None
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(previous) = app.try_state::<PreviousApp>() {
        previous.remember();
    }
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        log::warn!("main window is missing");
        return;
    };
    MAIN_OPEN.store(true, Ordering::SeqCst);
    update_dock(app);
    let shown = window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus());
    if let Err(err) = shown {
        log::warn!("failed to show main window: {err}");
    }
    let _ = app.emit_to(MAIN_LABEL, MAIN_VISIBLE_EVENT, true);
}

/// Shrimp keeps running in the tray, so closing the main window only hides it.
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event
        && window.label() == MAIN_LABEL
    {
        api.prevent_close();
        if let Err(err) = window.hide() {
            log::warn!("failed to hide main window: {err}");
        }
        MAIN_OPEN.store(false, Ordering::SeqCst);
        update_dock(window.app_handle());
        // A hidden page stays mounted (WebView2 even keeps it "visible"), so the
        // microphone meter of Settings would keep the microphone open.
        if let Some(microphones) = window.app_handle().try_state::<Arc<Microphones>>() {
            microphones.stop_preview();
        }
        let _ = window.emit_to(MAIN_LABEL, MAIN_VISIBLE_EVENT, false);
    }
}

/// "Show app in Dock" (APP-07): a Dock app, or only the menu bar icon. With
/// it off, the Dock icon (and the app's menu bar) still shows while the main
/// window is open or minimized, and goes when it closes to the tray, as
/// Wispr Flow's does (article 1790396454).
pub fn update_dock<R: Runtime>(app: &AppHandle<R>) {
    let in_dock = app
        .try_state::<Arc<SettingsStore>>()
        .is_none_or(|settings| settings.get().app.show_in_dock);
    let show = in_dock || MAIN_OPEN.load(Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    {
        let policy = if show {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };
        if let Err(err) = app.set_activation_policy(policy) {
            log::warn!("failed to change the Dock icon: {err}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = show;
}
