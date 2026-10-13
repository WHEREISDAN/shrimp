//! Microphones through the controller with fake devices (MIC-03..08):
//! ranking and fallback, hot-plug prompts, a closed lid, virtual devices,
//! warnings and Bluetooth advice.

mod support;

use shrimp_lib::notify::NoticeAction;
use support::*;

fn mic(id: &str, name: &str, kind: DeviceKind, behaviour: FakeBehaviour) -> FakeDevice {
    FakeDevice {
        device: InputDevice {
            id: id.into(),
            name: name.into(),
            kind,
        },
        behaviour,
    }
}

fn tone() -> FakeBehaviour {
    FakeBehaviour::Tone { amplitude: 0.3 }
}

fn harness(devices: Vec<FakeDevice>) -> (Harness, Arc<FakeDevices>) {
    harness_with_default(devices, None)
}

fn harness_with_default(
    devices: Vec<FakeDevice>,
    default: Option<&str>,
) -> (Harness, Arc<FakeDevices>) {
    let fakes = Arc::new(FakeDevices::default());
    fakes.set(devices);
    fakes.set_default(default);
    let h = Harness::with(
        1.0,
        0.3,
        fast_timing(),
        Setup {
            microphones: Some(Arc::clone(&fakes)),
            ..Setup::default()
        },
    );
    (h, fakes)
}

fn ranking(h: &Harness) -> Vec<String> {
    h.settings
        .get()
        .microphone
        .ranking
        .iter()
        .map(|known| known.id.clone())
        .collect()
}

fn notice_with(h: &Harness, text: &str) -> Notice {
    h.fakes
        .notices
        .lock()
        .unwrap()
        .iter()
        .find(|notice| notice.message.contains(text))
        .cloned()
        .unwrap_or_else(|| panic!("no notice with {text:?}: {:?}", h.fakes.messages()))
}

#[test]
fn auto_detect_ranks_the_first_microphones_and_falls_back_without_a_break() {
    let (h, fakes) = harness_with_default(
        vec![
            mic("builtin", "MacBook Microphone", DeviceKind::BuiltIn, tone()),
            mic("usb", "USB Microphone", DeviceKind::Wired, tone()),
        ],
        Some("usb"),
    );
    assert_eq!(ranking(&h), ["usb", "builtin"], "the system default first");

    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(600));
    fakes.remove("usb");
    std::thread::sleep(Duration::from_millis(700));
    h.shortcut(Action::PushToTalk, false);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert!(
        h.fakes
            .has_notice("USB Microphone disconnected. Switched to MacBook Microphone.")
    );
    let samples = h.fakes.transcribed.lock().unwrap()[0];
    assert!(
        samples > 16_000,
        "audio from both microphones reached STT: {samples} samples"
    );
}

#[test]
fn a_new_microphone_asks_switch_or_dont_switch_but_not_for_airpods_or_virtual_ones() {
    let (h, fakes) = harness(vec![mic(
        "builtin",
        "MacBook Microphone",
        DeviceKind::BuiltIn,
        tone(),
    )]);
    let microphones = h.microphones.as_ref().unwrap();
    assert_eq!(ranking(&h), ["builtin"]);

    fakes.add(mic("pods", "Dan's AirPods", DeviceKind::Bluetooth, tone()));
    fakes.add(mic("hole", "BlackHole 2ch", DeviceKind::Virtual, tone()));
    microphones.poll();
    assert_eq!(
        ranking(&h),
        ["builtin", "pods"],
        "AirPods join last, silently"
    );
    assert!(!h.fakes.has_notice("Switch to it?"));

    fakes.add(mic("usb", "Yeti", DeviceKind::Wired, tone()));
    microphones.poll();
    let prompt = notice_with(&h, "Yeti is connected. Switch to it?");
    assert_eq!(
        prompt.actions,
        [
            NoticeAction::SwitchToNewMicrophone,
            NoticeAction::KeepMicrophone
        ]
    );
    microphones.answer_new_microphone(true).unwrap();
    assert_eq!(ranking(&h), ["usb", "builtin", "pods"]);
    assert_eq!(microphones.list().active.as_deref(), Some("usb"));
}

#[test]
fn the_new_microphone_prompt_waits_until_the_recording_ends() {
    let (h, fakes) = harness(vec![mic(
        "builtin",
        "MacBook Microphone",
        DeviceKind::BuiltIn,
        tone(),
    )]);
    h.shortcut(Action::PushToTalk, true);
    wait_until("recording", || {
        matches!(h.fakes.last_state(), Some(DictationState::Recording { .. }))
    });
    fakes.add(mic("usb", "Yeti", DeviceKind::Wired, tone()));
    h.microphones.as_ref().unwrap().poll();
    assert!(!h.fakes.has_notice("Yeti is connected"));
    std::thread::sleep(Duration::from_millis(600));
    h.shortcut(Action::PushToTalk, false);
    wait_until("prompt", || h.fakes.has_notice("Yeti is connected"));
}

#[test]
fn with_the_lid_closed_the_built_in_microphone_is_unavailable() {
    let (h, fakes) = harness(vec![
        mic("builtin", "MacBook Microphone", DeviceKind::BuiltIn, tone()),
        mic("usb", "Yeti", DeviceKind::Wired, tone()),
    ]);
    let microphones = h.microphones.as_ref().unwrap();
    microphones.select(Some("builtin".into())).unwrap();
    fakes.set_lid_closed(true);
    let list = microphones.list();
    let builtin = list
        .devices
        .iter()
        .find(|entry| entry.id == "builtin")
        .unwrap();
    assert_eq!(builtin.unavailable.as_deref(), Some("Laptop lid is closed"));
    assert_eq!(list.active.as_deref(), Some("usb"));
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    let notice = notice_with(&h, "Selected microphone is unavailable");
    assert_eq!(
        notice.message,
        "Selected microphone is unavailable (Laptop lid is closed). Using Yeti."
    );
    assert_eq!(notice.action(), Some(NoticeAction::ChooseMicrophone));
}

#[test]
fn virtual_microphones_are_never_chosen_automatically() {
    let (h, _fakes) = harness(vec![mic(
        "hole",
        "BlackHole 2ch",
        DeviceKind::Virtual,
        tone(),
    )]);
    assert!(ranking(&h).is_empty());
    dictate(&h);
    wait_until("notice", || h.fakes.has_notice("No microphone detected"));
    let notice = notice_with(&h, "No microphone detected");
    assert_eq!(notice.action(), Some(NoticeAction::OpenSoundSettings));
    assert!(h.fakes.inserted().is_empty());

    let microphones = h.microphones.as_ref().unwrap();
    microphones.select(Some("hole".into())).unwrap();
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
}

#[test]
fn a_silent_microphone_looks_muted() {
    let (h, _fakes) = harness(vec![mic(
        "usb",
        "Yeti",
        DeviceKind::Wired,
        FakeBehaviour::Silent,
    )]);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(3_000));
    h.shortcut(Action::PushToTalk, false);
    wait_until("notice", || h.fakes.has_notice("Is your microphone muted?"));
    let notice = notice_with(&h, "Is your microphone muted?");
    assert_eq!(
        notice.actions,
        [
            NoticeAction::ChooseMicrophone,
            NoticeAction::TroubleshootMicrophone
        ]
    );
}

#[test]
fn five_seconds_without_audio_after_speech_warns_and_clears_when_audio_returns() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("speech-then-silence.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    for i in 0..16_000 * 7 {
        let value = if i < 16_000 {
            0.3 * (i as f32 * 0.07).sin()
        } else {
            0.0
        };
        writer
            .write_sample((value * f32::from(i16::MAX)) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
    let (h, _fakes) = harness(vec![mic(
        "usb",
        "Yeti",
        DeviceKind::Wired,
        FakeBehaviour::Wav { path },
    )]);
    h.controller
        .send(ControllerInput::Shortcut(ShortcutEvent::new(
            Action::HandsFree,
            true,
        )));
    let deadline = Instant::now() + Duration::from_secs(9);
    while !h.fakes.has_notice("No audio from Yeti for 5 seconds.") {
        assert!(
            Instant::now() < deadline,
            "no warning: {:?}",
            h.fakes.messages()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let notice = notice_with(&h, "No audio from Yeti");
    // The file loops back to speech after 7 s.
    std::thread::sleep(Duration::from_millis(1_800));
    assert!(h.fakes.dismissed.lock().unwrap().contains(&notice.id));
    h.shortcut(Action::HandsFree, true);
}

#[test]
fn a_microphone_slow_to_open_is_reported_after_8_seconds_until_it_opens() {
    let (h, _fakes) = harness(vec![mic(
        "usb",
        "Yeti",
        DeviceKind::Wired,
        FakeBehaviour::SlowOpen { millis: 8_600 },
    )]);
    h.controller
        .send(ControllerInput::Shortcut(ShortcutEvent::new(
            Action::HandsFree,
            true,
        )));
    let deadline = Instant::now() + Duration::from_secs(10);
    let slow = loop {
        let found = h
            .fakes
            .notices
            .lock()
            .unwrap()
            .iter()
            .find(|notice| notice.message == "Yeti is taking a long time to open.")
            .cloned();
        if let Some(notice) = found {
            break notice;
        }
        assert!(Instant::now() < deadline, "no warning");
        std::thread::sleep(Duration::from_millis(50));
    };
    wait_until("cleared once open", || {
        h.fakes.dismissed.lock().unwrap().contains(&slow.id)
    });
    h.shortcut(Action::HandsFree, true);
}

#[test]
fn bluetooth_microphones_get_advice_at_most_once_a_week() {
    let (h, _fakes) = harness(vec![mic(
        "pods",
        "Dan's AirPods",
        DeviceKind::Bluetooth,
        tone(),
    )]);
    dictate(&h);
    wait_until("advice", || {
        h.fakes.has_notice("Bluetooth microphones like")
    });
    let advice = notice_with(&h, "Bluetooth microphones like");
    assert_eq!(
        advice.actions,
        [
            NoticeAction::SwitchMicrophone,
            NoticeAction::DismissBluetoothAdvice
        ]
    );
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    dictate(&h);
    wait_until("second insert", || h.fakes.inserted().len() == 2);
    let count = h
        .fakes
        .messages()
        .iter()
        .filter(|message| message.contains("Bluetooth microphones like"))
        .count();
    assert_eq!(count, 1);
    assert_eq!(h.settings.get().notices.bluetooth_advice_count, 1);
    h.microphones
        .as_ref()
        .unwrap()
        .dismiss_bluetooth_advice()
        .unwrap();
    assert!(h.settings.get().notices.bluetooth_advice_off);
}

#[test]
fn when_every_microphone_is_unplugged_what_was_said_is_still_inserted() {
    let (h, fakes) = harness(vec![mic("usb", "Yeti", DeviceKind::Wired, tone())]);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(800));
    fakes.remove("usb");
    wait_until("disconnected", || {
        h.fakes.has_notice("Microphone disconnected")
    });
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    h.shortcut(Action::PushToTalk, false);
}
