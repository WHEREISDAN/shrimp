//! The Flow Bar window: a small transparent pill on the bottom, left or
//! right edge of the display under the cursor (BAR-05). Its webview reports
//! the size its content needs and the window follows, so transparent
//! margins never block clicks; with nothing to show it hides (BAR-04).
//! The panel never becomes key, so WebKit sends it no hover events: Rust
//! tracks the cursor and tells the webview when it is over the bar.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use super::events::FLOW_BAR_HOVER_EVENT;
use super::flow_bar_place::{self, Area, Spacing};
use crate::platform;
use crate::settings::SettingsStore;

pub const FLOW_BAR_LABEL: &str = "flow-bar";

/// Logical size before the webview reports its own.
const INITIAL_SIZE: (f64, f64) = (72.0, 28.0);
const MAX_SIZE: (f64, f64) = (480.0, 320.0);
/// The pill's height with its padding, in logical points.
const PILL_HEIGHT: f64 = 28.0;
/// Gap between the bar and the Dock or screen edge, in logical points.
const MARGIN: f64 = 10.0;
/// Room between the pill and the ends of its edge for the language chips
/// and the menu, in logical points.
const ROOM: f64 = 200.0;
/// How often the bar checks where the cursor is: over the bar (hover) or on
/// another display.
const TRACK_INTERVAL: Duration = Duration::from_millis(100);

/// What the bar's placement remembers between calls.
pub struct FlowBarState {
    settings: Arc<SettingsStore>,
    /// The content size in logical points; None while hidden.
    size: Mutex<Option<(f64, f64)>>,
    /// The work area it was last placed on.
    area: Mutex<Option<Area>>,
    dragging: Mutex<bool>,
    hovered: Mutex<bool>,
}

pub fn create<R: Runtime>(app: &AppHandle<R>, settings: Arc<SettingsStore>) -> tauri::Result<()> {
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
    let state = Arc::new(FlowBarState {
        settings,
        size: Mutex::new(Some(INITIAL_SIZE)),
        area: Mutex::new(None),
        dragging: Mutex::new(false),
        hovered: Mutex::new(false),
    });
    app.manage(Arc::clone(&state));
    place(&window, &state)?;
    // Tauri must know the window is visible, or WebKit keeps painting a stale
    // snapshot of the first frame over later content. The panel cannot
    // become key, so showing it does not take focus.
    window.show()?;
    platform::flow_bar::set_visible(app, FLOW_BAR_LABEL, true);
    track_cursor(app.clone(), state);
    Ok(())
}

fn on_main<R: Runtime>(app: &AppHandle<R>, work: impl FnOnce(&AppHandle<R>) + Send + 'static) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || work(&handle)) {
        log::warn!("could not reach the main thread: {error}");
    }
}

fn state<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<FlowBarState>> {
    app.try_state::<Arc<FlowBarState>>()
        .map(|state| Arc::clone(&state))
}

/// Resizes the bar to `width` x `height` logical points, or hides it when
/// it has nothing to show (0 x 0).
pub fn resize<R: Runtime>(app: &AppHandle<R>, width: f64, height: f64) {
    on_main(app, move |app| {
        let Some(state) = state(app) else {
            return;
        };
        let Some(window) = app.get_webview_window(FLOW_BAR_LABEL) else {
            return;
        };
        if width < 1.0 || height < 1.0 {
            *lock(&state.size) = None;
            platform::flow_bar::set_visible(app, FLOW_BAR_LABEL, false);
            return;
        }
        let shown = lock(&state.size).is_some();
        *lock(&state.size) = Some((width.min(MAX_SIZE.0), height.min(MAX_SIZE.1)));
        if !*lock(&state.dragging)
            && let Err(error) = place(&window, &state)
        {
            log::warn!("could not place the Flow Bar: {error}");
        }
        if !shown {
            platform::flow_bar::set_visible(app, FLOW_BAR_LABEL, true);
        }
    });
}

fn work_area(monitor: &Monitor) -> Area {
    let area = monitor.work_area();
    Area {
        x: f64::from(area.position.x),
        y: f64::from(area.position.y),
        width: f64::from(area.size.width),
        height: f64::from(area.size.height),
    }
}

/// The display under the cursor, or the primary one.
fn cursor_monitor<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<Option<Monitor>> {
    let cursor = window.cursor_position()?;
    match window.monitor_from_point(cursor.x, cursor.y)? {
        Some(monitor) => Ok(Some(monitor)),
        None => window.primary_monitor(),
    }
}

fn place<R: Runtime>(window: &WebviewWindow<R>, state: &FlowBarState) -> tauri::Result<()> {
    let Some((width, height)) = *lock(&state.size) else {
        return Ok(());
    };
    let Some(monitor) = cursor_monitor(window)? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let area = work_area(&monitor);
    *lock(&state.area) = Some(area);
    let settings = state.settings.get().flow_bar;
    let (x, y) = flow_bar_place::origin(
        area,
        settings.edge,
        settings.position,
        width * scale,
        height * scale,
        Spacing {
            pill: PILL_HEIGHT * scale,
            margin: MARGIN * scale,
            room: ROOM * scale,
        },
    );
    window.set_size(PhysicalSize::new(
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    ))?;
    window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))
}

/// Dragging the resting bar (BAR-05): it follows the cursor, and on release
/// docks to the nearest edge of that display, which is remembered.
pub fn drag<R: Runtime>(app: &AppHandle<R>, ended: bool) {
    on_main(app, move |app| {
        let (Some(state), Some(window)) = (state(app), app.get_webview_window(FLOW_BAR_LABEL))
        else {
            return;
        };
        let Ok(cursor) = window.cursor_position() else {
            return;
        };
        if !ended {
            *lock(&state.dragging) = true;
            if let (Ok(size), Some(_)) = (window.outer_size(), *lock(&state.size)) {
                let x = cursor.x - f64::from(size.width) / 2.0;
                let y = cursor.y - f64::from(size.height) / 2.0;
                let _ =
                    window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
            }
            return;
        }
        *lock(&state.dragging) = false;
        let Ok(Some(monitor)) = cursor_monitor(&window) else {
            return;
        };
        let (edge, position) =
            flow_bar_place::nearest_edge(work_area(&monitor), cursor.x, cursor.y);
        let patch = serde_json::json!({"flowBar": {"edge": edge, "position": position}});
        if let Err(error) = state.settings.update(&patch) {
            log::warn!("could not save where the Flow Bar is: {error}");
        }
        log::info!("Flow Bar docked to the {edge:?} edge at {position:.2}");
        if let Err(error) = place(&window, &state) {
            log::warn!("could not place the Flow Bar: {error}");
        }
    });
}

/// Follows the cursor: reports when it enters or leaves the bar, and moves
/// the bar to the display the cursor is on.
fn track_cursor<R: Runtime>(app: AppHandle<R>, state: Arc<FlowBarState>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(TRACK_INTERVAL);
        loop {
            interval.tick().await;
            if *lock(&state.dragging) {
                continue;
            }
            let state = Arc::clone(&state);
            on_main(&app, move |app| {
                if let Some(window) = app.get_webview_window(FLOW_BAR_LABEL) {
                    track_hover(app, &window, &state);
                    follow_display(&window, &state);
                }
            });
        }
    });
}

fn track_hover<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>, state: &FlowBarState) {
    let inside = lock(&state.size).is_some()
        && match (
            window.cursor_position(),
            window.outer_position(),
            window.outer_size(),
        ) {
            (Ok(cursor), Ok(origin), Ok(size)) => {
                let (left, top) = (f64::from(origin.x), f64::from(origin.y));
                (left..left + f64::from(size.width)).contains(&cursor.x)
                    && (top..top + f64::from(size.height)).contains(&cursor.y)
            }
            _ => false,
        };
    if std::mem::replace(&mut *lock(&state.hovered), inside) != inside
        && let Err(error) = app.emit_to(FLOW_BAR_LABEL, FLOW_BAR_HOVER_EVENT, inside)
    {
        log::warn!("could not report hovering the Flow Bar: {error}");
    }
}

fn follow_display<R: Runtime>(window: &WebviewWindow<R>, state: &FlowBarState) {
    if lock(&state.size).is_none() {
        return;
    }
    let Ok(Some(monitor)) = cursor_monitor(window) else {
        return;
    };
    if *lock(&state.area) != Some(work_area(&monitor)) {
        log::info!("Flow Bar moves to the display under the cursor");
        if let Err(error) = place(window, state) {
            log::warn!("could not move the Flow Bar: {error}");
        }
    }
}

/// Re-places the bar after its edge or position changed in the settings.
pub fn settings_changed<R: Runtime>(app: &AppHandle<R>) {
    on_main(app, |app| {
        if let (Some(state), Some(window)) = (state(app), app.get_webview_window(FLOW_BAR_LABEL))
            && !*lock(&state.dragging)
        {
            let _ = place(&window, &state);
        }
    });
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}
