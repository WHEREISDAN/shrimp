//! Mute or pause while dictating (SND-03, SND-04). Muting sets the default
//! output's mute (or its volume to 0) and puts it back. Pausing asks
//! MediaRemote: since macOS 15.4 only Apple-signed processes can read what is
//! playing, so `osascript` reads it, while commands still work in-process
//! through `MRMediaRemoteSendCommand`.

use std::ffi::{CStr, c_void};
use std::process::Command;

use super::core_audio;
use crate::platform::{MediaControl, Restore};

/// Calls and meetings are never paused.
const CALL_APPS: [&str; 10] = [
    "us.zoom.xos",
    "com.microsoft.teams",
    "com.microsoft.teams2",
    "com.apple.FaceTime",
    "com.cisco.webexmeetingsapp",
    "com.webex.meetingmanager",
    "com.tinyspeck.slackmacgap",
    "com.hnc.Discord",
    "com.skype.skype",
    "com.logmein.GoToMeeting",
];

const NOW_PLAYING: &str = r#"ObjC.import('Foundation');
$.NSBundle.bundleWithPath('/System/Library/PrivateFrameworks/MediaRemote.framework/').load;
const request = $.NSClassFromString('MRNowPlayingRequest');
const client = request.localNowPlayingPlayerPath.client;
JSON.stringify({ app: client.isNil() ? null : client.bundleIdentifier.js, playing: request.localIsPlaying });"#;

const PLAY: u32 = 0;
const PAUSE: u32 = 1;

/// Players are paused; calls and meetings never are (SND-04).
fn pausable(app: &str) -> bool {
    !CALL_APPS.contains(&app)
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
struct NowPlaying {
    app: Option<String>,
    playing: bool,
}

fn now_playing() -> Option<NowPlaying> {
    let output = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", NOW_PLAYING])
        .output()
        .ok()?;
    serde_json::from_slice(&output.stdout).ok()
}

type SendCommand = unsafe extern "C" fn(u32, *const c_void) -> bool;

fn send_command(command: u32) -> bool {
    const FRAMEWORK: &CStr = c"/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote";
    // SAFETY: dlopen and dlsym with valid C strings; the symbol has this signature.
    unsafe {
        let library = libc::dlopen(FRAMEWORK.as_ptr(), libc::RTLD_LAZY);
        if library.is_null() {
            return false;
        }
        let symbol = libc::dlsym(library, c"MRMediaRemoteSendCommand".as_ptr());
        if symbol.is_null() {
            return false;
        }
        let send: SendCommand = std::mem::transmute(symbol);
        send(command, std::ptr::null())
    }
}

pub struct MacMedia;

impl MediaControl for MacMedia {
    fn mute(&self) -> Option<Restore> {
        let device = core_audio::default_output()?;
        if !core_audio::is_running(device) || core_audio::muted(device) == Some(true) {
            return None;
        }
        if core_audio::set_muted(device, true) {
            log::info!("muted the output while dictating");
            return Some(Box::new(move || {
                core_audio::set_muted(device, false);
            }));
        }
        let volume = core_audio::volume(device)?;
        core_audio::set_volume(device, 0.0).then(|| {
            log::info!("turned the output down while dictating");
            Box::new(move || {
                core_audio::set_volume(device, volume);
            }) as Restore
        })
    }

    fn pause(&self) -> Option<Restore> {
        let output = core_audio::default_output()?;
        if !core_audio::is_running(output) {
            return None;
        }
        let playing = now_playing()?;
        let app = playing.app.filter(|_| playing.playing)?;
        if !pausable(&app) || !send_command(PAUSE) {
            return None;
        }
        log::info!("paused {app} while dictating");
        Some(Box::new(move || {
            std::thread::spawn(move || {
                let still_paused = now_playing()
                    .is_some_and(|now| now.app.as_deref() == Some(app.as_str()) && !now.playing);
                if still_paused && send_command(PLAY) {
                    log::info!("resumed {app}");
                }
            });
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_are_never_paused() {
        assert!(pausable("com.apple.Music"));
        assert!(pausable("com.spotify.client"));
        assert!(!pausable("us.zoom.xos"));
        assert!(!pausable("com.apple.FaceTime"));
        assert!(!pausable("com.microsoft.teams2"));
    }

    #[test]
    fn reads_the_now_playing_probe() {
        let playing: NowPlaying =
            serde_json::from_str(r#"{"app":"com.apple.QuickTimePlayerX","playing":true}"#).unwrap();
        assert_eq!(playing.app.as_deref(), Some("com.apple.QuickTimePlayerX"));
        let nothing: NowPlaying = serde_json::from_str(r#"{"app":null,"playing":false}"#).unwrap();
        assert_eq!(nothing.app, None);
    }
}
