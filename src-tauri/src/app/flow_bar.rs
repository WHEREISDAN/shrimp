//! The Flow Bar window: a small transparent pill at the bottom centre of the
//! screen, above the Dock. Its webview reports the size its content needs and
//! the window follows, so transparent margins never block clicks.

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::platform;

pub const FLOW_BAR_LABEL: &str = "flow-bar";

/// Logical size before the webview reports its own.
const INITIAL_SIZE: (f64, f64) = (72.0, 28.0);
const MAX_SIZE: (f64, f64) = (480.0, 240.0);
/// Gap between the bar and the Dock or screen edge, in logical points.
const BOTTOM_MARGIN: f64 = 10.0;

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let window =
        WebviewWindowBuilder::new(app, FLOW_BAR_LABEL, WebviewUrl::App("flow-bar.html".into()))
            .title("Shrimp Flow Bar")
            .inner_size(INITIAL_SIZE.0, INITIAL_SIZE.1)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .focused(false)
            .focusable(false)
            .accept_first_mouse(true)
            .visible(false)
            .build()?;
    if let Err(error) = platform::flow_bar::make_panel(&window) {
        log::error!("{error}");
    }
    place(&window, INITIAL_SIZE.0, INITIAL_SIZE.1)?;
    // Tauri must know the window is visible, or WebKit keeps painting a stale
    // snapshot of the first frame over later content. The panel cannot
    // become key, so showing it does not take focus.
    window.show()?;
    platform::flow_bar::set_visible(app, FLOW_BAR_LABEL, true);
    Ok(())
}

/// Resizes the bar to `width` x `height` logical points and keeps it
/// bottom-centred. Runs on the main thread.
pub fn resize<R: Runtime>(app: &AppHandle<R>, width: f64, height: f64) {
    let width = width.clamp(1.0, MAX_SIZE.0);
    let height = height.clamp(1.0, MAX_SIZE.1);
    let handle = app.clone();
    let resized = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(FLOW_BAR_LABEL)
            && let Err(error) = place(&window, width, height)
        {
            log::warn!("could not resize the Flow Bar: {error}");
        }
    });
    if let Err(error) = resized {
        log::warn!("could not reach the main thread: {error}");
    }
}

fn place<R: Runtime>(window: &WebviewWindow<R>, width: f64, height: f64) -> tauri::Result<()> {
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let size = PhysicalSize::new(
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    );
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + area.size.height as i32
        - size.height as i32
        - (BOTTOM_MARGIN * scale).round() as i32;
    window.set_size(size)?;
    window.set_position(PhysicalPosition::new(x, y))
}
