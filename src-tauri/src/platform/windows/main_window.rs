//! The main window's frame on Windows 10. With tao's shadow, an undecorated
//! window keeps a frame on its left, right and bottom, where Windows 10 draws
//! a translucent 1 px line that turns white over anything light (Tauri's
//! documented "1px white border"). Without that shadow the window has no frame
//! and no line; extending DWM's frame 1 px into the window brings the shadow
//! back, and the page draws its own hairline and resize borders. Windows 11
//! keeps tao's frame, which it rounds and outlines itself.

use tauri::{LogicalSize, Manager, Runtime, WebviewWindow};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows::Win32::UI::Controls::MARGINS;

use crate::platform::PlatformError;

fn failed(err: tauri::Error) -> PlatformError {
    PlatformError::Failed(err.to_string())
}

fn hwnd<R: Runtime>(window: &WebviewWindow<R>) -> Result<HWND, PlatformError> {
    window.hwnd().map(|handle| HWND(handle.0)).map_err(failed)
}

/// Swaps the frame line for a shadow alone. Call before the window is first
/// shown.
pub fn drop_frame_line<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), PlatformError> {
    window.set_shadow(false).map_err(failed)?;
    // The client area takes over the frame, so the window gets its configured
    // size again; its own inner size isn't final before it is first shown.
    let configured = window
        .app_handle()
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == window.label())
        .map(|config| LogicalSize::new(config.width, config.height));
    if let Some(size) = configured {
        window.set_size(size).map_err(failed)?;
    }
    let margins = MARGINS {
        cxLeftWidth: 1,
        cxRightWidth: 1,
        cyTopHeight: 1,
        cyBottomHeight: 1,
    };
    unsafe { DwmExtendFrameIntoClientArea(hwnd(window)?, &margins) }
        .map_err(|err| PlatformError::Failed(format!("could not extend the frame: {err}")))
}
