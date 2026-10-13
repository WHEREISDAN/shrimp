//! The Flow Bar as a non-activating `NSPanel`: it floats above every window,
//! including full-screen Spaces, and clicking it never takes focus from the
//! app being dictated into.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime, WebviewWindow};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt, tauri_panel,
};

use crate::platform::PlatformError;

tauri_panel! {
    panel!(FlowBarPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            is_floating_panel: true
        }
    })
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_nspanel::init()
}

/// Converts the Flow Bar window into a panel. Call on the main thread.
pub fn make_panel<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), PlatformError> {
    let failed =
        |err: &dyn std::fmt::Display| PlatformError::Failed(format!("Flow Bar panel: {err}"));
    let panel = window
        .to_panel::<FlowBarPanel<R>>()
        .map_err(|err| failed(&err))?;
    panel.set_level(PanelLevel::Status.value());
    panel
        .add_style_mask(StyleMask::empty().nonactivating_panel().value())
        .map_err(|err| failed(&err))?;
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .full_screen_auxiliary()
            .stationary()
            .ignores_cycle()
            .value(),
    );
    panel.set_hides_on_deactivate(false);
    panel.set_works_when_modal(true);
    panel.set_has_shadow(false);
    Ok(())
}

/// Shows the panel for the first time. Tauri must know the window is
/// visible, or WebKit keeps painting a stale snapshot of the first frame over
/// later content. The panel cannot become key, so showing it does not take focus.
pub fn show_first<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.show()?;
    set_visible(app, window.label(), true);
    Ok(())
}

/// Shows (without activating Shrimp) or hides the panel. Call on the main thread.
pub fn set_visible<R: Runtime>(app: &AppHandle<R>, label: &str, visible: bool) {
    match app.get_webview_panel(label) {
        Ok(panel) if visible => panel.show(),
        Ok(panel) => panel.hide(),
        Err(err) => log::warn!("Flow Bar panel missing: {err:?}"),
    }
}
