//! The Flow Bar as a non-activating, topmost tool window: it floats above
//! other windows, stays out of the taskbar and Alt+Tab, and clicking it
//! never takes focus from the app being dictated into. Tao rewrites the
//! extended style when it changes window flags, so the style is applied
//! again whenever the bar is shown, and the bar is shown without activation.

use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, HWND_TOPMOST, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SetWindowLongPtrW, SetWindowPos, ShowWindow, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::platform::PlatformError;

fn hwnd<R: Runtime>(window: &WebviewWindow<R>) -> Option<HWND> {
    window.hwnd().ok().map(|handle| HWND(handle.0))
}

fn apply_style(window: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(window, GWL_EXSTYLE);
        let wanted = (style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as isize)
            & !(WS_EX_APPWINDOW.0 as isize);
        if wanted != style {
            SetWindowLongPtrW(window, GWL_EXSTYLE, wanted);
        }
        let _ = SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Gives the Flow Bar its window style. Call on the main thread.
pub fn make_panel<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), PlatformError> {
    let handle =
        hwnd(window).ok_or_else(|| PlatformError::Failed("Flow Bar has no window".into()))?;
    apply_style(handle);
    Ok(())
}

/// Shows the bar for the first time. Tauri's `show` would activate it and
/// take focus from the app in front, so it is shown without activation.
pub fn show_first<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) -> tauri::Result<()> {
    set_visible(app, window.label(), true);
    Ok(())
}

/// Shows the bar without activating it, or hides it. Call on the main thread.
pub fn set_visible<R: Runtime>(app: &AppHandle<R>, label: &str, visible: bool) {
    let Some(handle) = app.get_webview_window(label).as_ref().and_then(hwnd) else {
        log::warn!("Flow Bar window missing");
        return;
    };
    unsafe {
        if visible {
            apply_style(handle);
            let _ = ShowWindow(handle, SW_SHOWNOACTIVATE);
        } else {
            let _ = ShowWindow(handle, SW_HIDE);
        }
    }
}
