//! Settings › System's Reset & restart (APP-08), the error screens'
//! actions (APP-09) and Move to Applications (APP-11).

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::CommandError;
use crate::app::events::HEALTH_EVENT;
use crate::health::{self, AppHealth, Health, Repair};
use crate::platform;

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, CommandError> {
    app.path()
        .app_data_dir()
        .map_err(|err| CommandError::new("platform", err))
}

/// Free space where Shrimp keeps its data. Debug builds read
/// `SHRIMP_FAKE_FREE_BYTES` instead, to show the storage screen.
pub fn free_bytes(dir: &std::path::Path) -> Option<u64> {
    if cfg!(debug_assertions)
        && let Some(fake) = std::env::var("SHRIMP_FAKE_FREE_BYTES")
            .ok()
            .and_then(|value| value.parse().ok())
    {
        return Some(fake);
    }
    platform::free_bytes(dir)
}

#[tauri::command]
pub fn get_app_health(health: State<'_, Arc<Health>>) -> AppHealth {
    health.get()
}

/// The crash screen's Continue.
#[tauri::command]
pub fn dismiss_crash(app: AppHandle, health: State<'_, Arc<Health>>) -> AppHealth {
    health.dismiss_crash();
    let current = health.get();
    let _ = app.emit(HEALTH_EVENT, &current);
    current
}

/// The storage screen's Check again.
#[tauri::command]
pub fn check_storage(
    app: AppHandle,
    health: State<'_, Arc<Health>>,
) -> Result<AppHealth, CommandError> {
    health.check_storage(free_bytes(&data_dir(&app)?));
    let current = health.get();
    let _ = app.emit(HEALTH_EVENT, &current);
    Ok(current)
}

#[tauri::command]
pub fn open_storage_settings() -> Result<(), CommandError> {
    platform::open_storage_settings().map_err(|err| CommandError::new("platform", err))
}

/// Quits normally (Whisper unloads, the crash marker goes) and opens the
/// app again once this process has gone.
pub(crate) fn restart<R: Runtime>(app: &AppHandle<R>) -> Result<(), CommandError> {
    platform::relaunch_after_exit().map_err(|err| CommandError::new("platform", err))?;
    app.exit(0);
    Ok(())
}

/// The damaged-data screen: restore the newest backup or start fresh, then
/// restart, which repairs the files before opening them.
#[tauri::command]
pub fn repair_data(repair: Repair, app: AppHandle) -> Result<(), CommandError> {
    health::request_repair(&data_dir(&app)?, repair).map_err(|err| CommandError::new("io", err))?;
    restart(&app)
}

/// The Move to Applications screen's button (APP-11): copies Shrimp into
/// the Applications folder, quits and opens the copy. Debug builds only
/// log what they would copy and keep running.
#[tauri::command]
pub async fn move_to_applications(app: AppHandle) -> Result<(), CommandError> {
    let quit = tauri::async_runtime::spawn_blocking(platform::move_to_applications)
        .await
        .map_err(|err| CommandError::new("platform", err))?
        .map_err(|err| CommandError::new("platform", err))?;
    if quit {
        app.exit(0);
    }
    Ok(())
}

/// Quits normally, from the Move to Applications screen.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Reset & restart: local data, settings and API keys are deleted at the
/// next launch; the downloaded speech models stay.
#[tauri::command]
pub fn reset_app(app: AppHandle) -> Result<(), CommandError> {
    health::request_reset(&data_dir(&app)?).map_err(|err| CommandError::new("io", err))?;
    restart(&app)
}
