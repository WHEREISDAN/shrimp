//! Watches a recording's microphone for trouble (MIC-07): a microphone
//! taking over 8 s to open, no sound at all at the start (muted), and, after
//! real audio, 5 s and then 15 s without any. Warnings clear when the cause
//! goes away. Pure and clock-free, so it is unit-tested.

use std::time::{Duration, Instant};

const SLOW_OPEN: Duration = Duration::from_secs(8);
/// Long enough to tell a muted microphone from a quiet start.
const MUTED_AFTER: Duration = Duration::from_millis(2500);
const NO_AUDIO: Duration = Duration::from_secs(5);
const STILL_NO_AUDIO: Duration = Duration::from_secs(15);
/// Below this RMS nothing at all is coming in (about -66 dBFS).
const NOTHING_RMS: f32 = 0.0005;
/// Above this RMS someone is speaking (about -40 dBFS).
const SPEECH_RMS: f32 = 0.01;
/// Below this RMS it is quiet again (about -50 dBFS).
const QUIET_RMS: f32 = 0.003;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MicWarning {
    SlowToOpen,
    /// Nothing but silence since it opened: probably muted.
    Muted,
    NoAudio,
    StillNoAudio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicChange {
    Show(MicWarning),
    /// The cause went away; the notice can go.
    Clear(MicWarning),
}

#[derive(Debug)]
pub struct MicMonitor {
    started: Instant,
    opened: Option<Instant>,
    /// The loudest level since it opened.
    peak: f32,
    heard_speech: bool,
    /// When it last sounded like someone speaking.
    last_loud: Option<Instant>,
    shown: Vec<MicWarning>,
}

impl MicMonitor {
    pub fn new(now: Instant) -> Self {
        Self {
            started: now,
            opened: None,
            peak: 0.0,
            heard_speech: false,
            last_loud: None,
            shown: Vec::new(),
        }
    }

    /// Called about 30 times a second with whether the source has opened and
    /// the RMS of the samples since the last call (None when none came).
    pub fn update(&mut self, now: Instant, open: bool, rms: Option<f32>) -> Vec<MicChange> {
        let mut changes = Vec::new();
        if !open {
            if now.duration_since(self.started) >= SLOW_OPEN {
                self.show(MicWarning::SlowToOpen, &mut changes);
            }
            return changes;
        }
        let opened = *self.opened.get_or_insert(now);
        self.clear(MicWarning::SlowToOpen, &mut changes);
        let level = rms.unwrap_or(0.0);
        self.peak = self.peak.max(level);
        if level >= SPEECH_RMS {
            self.heard_speech = true;
            self.last_loud = Some(now);
            self.clear(MicWarning::Muted, &mut changes);
            self.clear(MicWarning::NoAudio, &mut changes);
            self.clear(MicWarning::StillNoAudio, &mut changes);
        } else if level >= QUIET_RMS && self.heard_speech {
            self.last_loud = Some(now);
        }
        if !self.heard_speech
            && self.peak < NOTHING_RMS
            && now.duration_since(opened) >= MUTED_AFTER
        {
            self.show(MicWarning::Muted, &mut changes);
        }
        if let Some(loud) = self.last_loud {
            let quiet = now.duration_since(loud);
            if quiet >= STILL_NO_AUDIO {
                self.show(MicWarning::StillNoAudio, &mut changes);
            } else if quiet >= NO_AUDIO {
                self.show(MicWarning::NoAudio, &mut changes);
            }
        }
        changes
    }

    fn show(&mut self, warning: MicWarning, changes: &mut Vec<MicChange>) {
        if !self.shown.contains(&warning) {
            self.shown.push(warning);
            changes.push(MicChange::Show(warning));
        }
    }

    fn clear(&mut self, warning: MicWarning, changes: &mut Vec<MicChange>) {
        if let Some(index) = self.shown.iter().position(|shown| *shown == warning) {
            self.shown.remove(index);
            changes.push(MicChange::Clear(warning));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn a_slow_microphone_is_reported_after_8_seconds_and_cleared_when_it_opens() {
        let start = Instant::now();
        let mut monitor = MicMonitor::new(start);
        assert!(monitor.update(at(start, 7_900), false, None).is_empty());
        assert_eq!(
            monitor.update(at(start, 8_000), false, None),
            [MicChange::Show(MicWarning::SlowToOpen)]
        );
        assert!(
            monitor.update(at(start, 8_100), false, None).is_empty(),
            "once"
        );
        assert_eq!(
            monitor.update(at(start, 9_000), true, Some(0.05)),
            [MicChange::Clear(MicWarning::SlowToOpen)]
        );
    }

    #[test]
    fn digital_silence_at_the_start_looks_muted() {
        let start = Instant::now();
        let mut monitor = MicMonitor::new(start);
        assert!(monitor.update(start, true, Some(0.0)).is_empty());
        assert!(monitor.update(at(start, 2_000), true, Some(0.0)).is_empty());
        assert_eq!(
            monitor.update(at(start, 2_500), true, Some(0.0)),
            [MicChange::Show(MicWarning::Muted)]
        );
        assert_eq!(
            monitor.update(at(start, 3_000), true, Some(0.2)),
            [MicChange::Clear(MicWarning::Muted)]
        );
    }

    #[test]
    fn room_noise_is_not_muted_but_a_quiet_start_does_not_count_as_no_audio() {
        let start = Instant::now();
        let mut monitor = MicMonitor::new(start);
        for millis in (0..20_000).step_by(100) {
            assert!(
                monitor
                    .update(at(start, millis), true, Some(0.001))
                    .is_empty(),
                "at {millis} ms"
            );
        }
    }

    #[test]
    fn silence_after_speech_warns_at_5_and_15_seconds_and_clears_when_audio_returns() {
        let start = Instant::now();
        let mut monitor = MicMonitor::new(start);
        monitor.update(start, true, Some(0.1));
        assert!(monitor.update(at(start, 4_900), true, None).is_empty());
        assert_eq!(
            monitor.update(at(start, 5_000), true, None),
            [MicChange::Show(MicWarning::NoAudio)]
        );
        assert_eq!(
            monitor.update(at(start, 15_000), true, Some(0.0)),
            [MicChange::Show(MicWarning::StillNoAudio)]
        );
        assert_eq!(
            monitor.update(at(start, 16_000), true, Some(0.2)),
            [
                MicChange::Clear(MicWarning::NoAudio),
                MicChange::Clear(MicWarning::StillNoAudio)
            ]
        );
    }
}
