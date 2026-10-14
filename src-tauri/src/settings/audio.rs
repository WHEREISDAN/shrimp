//! Microphone, sound and media settings (MIC-02..08, SND-02..04).

use serde::{Deserialize, Serialize};

use crate::audio::DeviceKind;
use crate::t;

/// Settings › Microphone.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MicrophoneSettings {
    /// A device id, or None for Auto-detect.
    pub selected: Option<String>,
    /// Physical microphones in the user's order; Auto-detect uses the first
    /// one connected (MIC-03). Forgetting one removes it.
    pub ranking: Vec<KnownMicrophone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownMicrophone {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
}

/// Settings › Sound (SND-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SoundSettings {
    /// Start, stop, lock, cancel and paste cues.
    pub dictation: bool,
    /// The sounds that come with a warning or error notice and with milestones.
    pub notifications: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            dictation: true,
            notifications: true,
        }
    }
}

/// What happens to playing audio while dictating (SND-03, SND-04). At most
/// one is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaSettings {
    /// Mute the output while dictating; off by default on macOS, on by
    /// default on Windows (WIN-08).
    pub mute: bool,
    /// Pause music and video players while dictating.
    pub pause: bool,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            mute: crate::platform::MUTE_BY_DEFAULT,
            pause: false,
        }
    }
}

impl MediaSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.mute && self.pause {
            return Err(t!("error.settings.muteOrPause"));
        }
        Ok(())
    }
}

impl MicrophoneSettings {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids: Vec<&str> = self.ranking.iter().map(|known| known.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() != self.ranking.len() {
            return Err(t!("error.settings.microphoneRankedTwice"));
        }
        if self.selected.as_deref().is_some_and(str::is_empty) {
            return Err(t!("error.settings.chooseMicrophone"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_auto_detect_with_sounds_and_mute_per_platform() {
        let microphone = MicrophoneSettings::default();
        assert_eq!(microphone.selected, None);
        assert!(SoundSettings::default().dictation && SoundSettings::default().notifications);
        let media = MediaSettings::default();
        assert_eq!(media.mute, crate::platform::MUTE_BY_DEFAULT);
        assert!(!media.pause);
    }

    #[test]
    fn muting_and_pausing_exclude_each_other() {
        let both = MediaSettings {
            mute: true,
            pause: true,
        };
        assert!(both.validate().is_err());
    }
}
