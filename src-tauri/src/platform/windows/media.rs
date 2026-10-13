//! Mute or pause while dictating on Windows. Muting mutes each app that is
//! playing on the default output and unmutes the same sessions afterwards
//! (WIN-08). Pausing asks the system media transport controls to pause each
//! playing session and resumes the ones still paused afterwards (WIN-12).
//! Calls and meetings are never paused.

use std::time::Duration;

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

use super::audio;
use crate::platform::{MediaControl, Restore};

/// Calls and meetings, by app id or executable.
const CALL_APPS: [&str; 10] = [
    "teams",
    "zoom",
    "discord",
    "slack",
    "skype",
    "webex",
    "gotomeeting",
    "ringcentral",
    "whatsapp",
    "phone",
];
const SMTC_TIMEOUT: Duration = Duration::from_secs(2);

fn is_call(app: &str) -> bool {
    let app = app.to_lowercase();
    CALL_APPS.iter().any(|call| app.contains(call))
}

#[derive(Default)]
pub struct WinMedia;

impl MediaControl for WinMedia {
    fn mute(&self) -> Option<Restore> {
        let own = std::process::id();
        let muted: Vec<String> = audio::output_sessions()
            .into_iter()
            .filter(|session| session.sounding && session.pid != own)
            .filter(|session| {
                unsafe { session.volume.GetMute() }.is_ok_and(|muted| !muted.as_bool())
            })
            .filter(|session| unsafe { session.volume.SetMute(true, std::ptr::null()) }.is_ok())
            .map(|session| session.instance)
            .collect();
        if muted.is_empty() {
            return None;
        }
        log::info!("muted {} playing app(s) while dictating", muted.len());
        Some(Box::new(move || {
            for session in audio::output_sessions() {
                if muted.contains(&session.instance) {
                    let _ = unsafe { session.volume.SetMute(false, std::ptr::null()) };
                }
            }
        }))
    }

    fn pause(&self) -> Option<Restore> {
        let paused = on_media_thread(pause_playing)?;
        if paused.is_empty() {
            return None;
        }
        log::info!("paused {paused:?} while dictating");
        Some(Box::new(move || {
            std::thread::spawn(move || {
                let resumed = on_media_thread(move || resume(&paused));
                log::info!("resumed {resumed:?}");
            });
        }))
    }
}

/// Runs WinRT work on a thread of its own (multithreaded COM), bounded in time.
fn on_media_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        audio::com();
        let _ = tx.send(work());
    });
    rx.recv_timeout(SMTC_TIMEOUT).ok()
}

/// Pauses every playing session that isn't a call; returns their app ids.
fn pause_playing() -> Vec<String> {
    let Ok(manager) = SessionManager::RequestAsync().and_then(|request| request.join()) else {
        return Vec::new();
    };
    let Ok(sessions) = manager.GetSessions() else {
        return Vec::new();
    };
    sessions
        .into_iter()
        .filter_map(|session| {
            let app = session.SourceAppUserModelId().ok()?.to_string();
            let playing = session
                .GetPlaybackInfo()
                .and_then(|info| info.PlaybackStatus())
                .is_ok_and(|status| status == PlaybackStatus::Playing);
            if !playing || is_call(&app) {
                return None;
            }
            let paused = session
                .TryPauseAsync()
                .and_then(|request| request.join())
                .unwrap_or(false);
            paused.then_some(app)
        })
        .collect()
}

/// Plays the sessions of `apps` again if they are still paused.
fn resume(apps: &[String]) -> Vec<String> {
    let Ok(manager) = SessionManager::RequestAsync().and_then(|request| request.join()) else {
        return Vec::new();
    };
    let Ok(sessions) = manager.GetSessions() else {
        return Vec::new();
    };
    sessions
        .into_iter()
        .filter_map(|session| {
            let app = session.SourceAppUserModelId().ok()?.to_string();
            let paused = session
                .GetPlaybackInfo()
                .and_then(|info| info.PlaybackStatus())
                .is_ok_and(|status| status == PlaybackStatus::Paused);
            if !apps.contains(&app) || !paused {
                return None;
            }
            session
                .TryPlayAsync()
                .and_then(|request| request.join())
                .unwrap_or(false)
                .then_some(app)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_are_never_paused() {
        assert!(is_call("MSTeams_8wekyb3d8bbwe!MSTeams"));
        assert!(is_call("Zoom.exe"));
        assert!(is_call("Discord.exe"));
        assert!(!is_call("Spotify.exe"));
        assert!(!is_call(
            "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"
        ));
        assert!(!is_call("chrome.exe"));
    }
}
