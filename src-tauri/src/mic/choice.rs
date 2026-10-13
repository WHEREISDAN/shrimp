//! Which microphone a dictation records from (MIC-03, MIC-05, MIC-06): the
//! selected one if it can be used, else the ranked physical microphones in
//! order, then other physical ones. Virtual devices are used only when
//! selected, and a closed lid rules out the built-in microphone.

use serde::Serialize;

use crate::audio::{DeviceKind, InputDevice};
use crate::settings::{KnownMicrophone, MicrophoneSettings};
use crate::t;

/// One microphone as the picker shows it. Mirrors `MicrophoneEntry` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneEntry {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub connected: bool,
    /// Why it can't be used now.
    pub unavailable: Option<String>,
    /// Its place in the ranking, for physical microphones.
    pub rank: Option<usize>,
}

/// The picker's contents. Mirrors `MicrophoneList` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneList {
    /// A device id, or None for Auto-detect.
    pub selected: Option<String>,
    /// The device a dictation would record from now.
    pub active: Option<String>,
    /// Ranked microphones (connected or not), other connected physical
    /// ones, then connected virtual devices.
    pub devices: Vec<MicrophoneEntry>,
}

/// Why `device` can't be used now, if it can't.
pub fn unavailable(device: &InputDevice, lid_closed: bool) -> Option<String> {
    (device.kind == DeviceKind::BuiltIn && lid_closed).then(|| t!("notice.mic.lidClosed"))
}

fn physical(device: &InputDevice) -> bool {
    device.kind != DeviceKind::Virtual
}

/// Unranked physical microphones in the order Auto-detect tries them: the
/// system default, then the built-in one, then the rest by name.
fn unranked<'a>(
    connected: &'a [InputDevice],
    settings: &MicrophoneSettings,
    default_input: Option<&str>,
) -> Vec<&'a InputDevice> {
    let mut devices: Vec<&InputDevice> = connected
        .iter()
        .filter(|device| physical(device))
        .filter(|device| !settings.ranking.iter().any(|known| known.id == device.id))
        .collect();
    devices.sort_by_key(|device| {
        (
            Some(device.id.as_str()) != default_input,
            device.kind != DeviceKind::BuiltIn,
            device.name.to_lowercase(),
        )
    });
    devices
}

/// The devices a dictation tries, best first.
pub fn order(
    connected: &[InputDevice],
    settings: &MicrophoneSettings,
    lid_closed: bool,
    default_input: Option<&str>,
) -> Vec<InputDevice> {
    let usable = |device: &&InputDevice| unavailable(device, lid_closed).is_none();
    let selected = settings
        .selected
        .as_deref()
        .and_then(|id| connected.iter().find(|device| device.id == id))
        .filter(usable);
    let ranked = settings
        .ranking
        .iter()
        .filter_map(|known| connected.iter().find(|device| device.id == known.id))
        .filter(|device| physical(device));
    let mut order: Vec<InputDevice> = Vec::new();
    for device in selected
        .into_iter()
        .chain(ranked)
        .chain(unranked(connected, settings, default_input))
        .filter(usable)
    {
        if !order.iter().any(|known| known.id == device.id) {
            order.push(device.clone());
        }
    }
    order
}

pub fn list(
    connected: &[InputDevice],
    settings: &MicrophoneSettings,
    lid_closed: bool,
    default_input: Option<&str>,
) -> MicrophoneList {
    let entry = |device: &InputDevice, connected: bool, rank: Option<usize>| MicrophoneEntry {
        id: device.id.clone(),
        name: device.name.clone(),
        kind: device.kind,
        connected,
        unavailable: unavailable(device, lid_closed),
        rank,
    };
    let ranked = settings.ranking.iter().enumerate().map(|(rank, known)| {
        let device = connected
            .iter()
            .find(|device| device.id == known.id)
            .cloned()
            .unwrap_or_else(|| InputDevice {
                id: known.id.clone(),
                name: known.name.clone(),
                kind: known.kind,
            });
        let is_connected = connected.iter().any(|device| device.id == known.id);
        entry(&device, is_connected, Some(rank))
    });
    let others = unranked(connected, settings, default_input)
        .into_iter()
        .map(|device| entry(device, true, None));
    let virtuals = connected
        .iter()
        .filter(|device| !physical(device))
        .map(|device| entry(device, true, None));
    MicrophoneList {
        selected: settings.selected.clone(),
        active: order(connected, settings, lid_closed, default_input)
            .first()
            .map(|device| device.id.clone()),
        devices: ranked.chain(others).chain(virtuals).collect(),
    }
}

/// The ranking with `device` added first or last.
pub fn ranked_with(
    ranking: &[KnownMicrophone],
    device: &InputDevice,
    first: bool,
) -> Vec<KnownMicrophone> {
    let known = KnownMicrophone {
        id: device.id.clone(),
        name: device.name.clone(),
        kind: device.kind,
    };
    let rest = ranking
        .iter()
        .filter(|other| other.id != device.id)
        .cloned();
    if first {
        std::iter::once(known).chain(rest).collect()
    } else {
        rest.chain(std::iter::once(known)).collect()
    }
}

/// The ranking with `id` moved one place up or down.
pub fn moved(ranking: &[KnownMicrophone], id: &str, up: bool) -> Vec<KnownMicrophone> {
    let mut ranking = ranking.to_vec();
    if let Some(index) = ranking.iter().position(|known| known.id == id) {
        let target = if up {
            index.checked_sub(1)
        } else {
            Some(index + 1).filter(|next| *next < ranking.len())
        };
        if let Some(target) = target {
            ranking.swap(index, target);
        }
    }
    ranking
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(id: &str, kind: DeviceKind) -> InputDevice {
        InputDevice {
            id: id.into(),
            name: format!("{id} mic"),
            kind,
        }
    }

    fn known(device: &InputDevice) -> KnownMicrophone {
        KnownMicrophone {
            id: device.id.clone(),
            name: device.name.clone(),
            kind: device.kind,
        }
    }

    fn ids(devices: &[InputDevice]) -> Vec<&str> {
        devices.iter().map(|device| device.id.as_str()).collect()
    }

    #[test]
    fn auto_detect_follows_the_ranking_and_skips_virtual_devices() {
        let builtin = device("builtin", DeviceKind::BuiltIn);
        let usb = device("usb", DeviceKind::Wired);
        let blackhole = device("blackhole", DeviceKind::Virtual);
        let connected = [builtin.clone(), usb.clone(), blackhole.clone()];
        let settings = MicrophoneSettings {
            selected: None,
            ranking: vec![known(&usb), known(&builtin)],
        };
        assert_eq!(
            ids(&order(&connected, &settings, false, Some("blackhole"))),
            ["usb", "builtin"],
            "a virtual default is never chosen automatically"
        );
        let first_run = MicrophoneSettings::default();
        assert_eq!(
            ids(&order(&connected, &first_run, false, Some("usb"))),
            ["usb", "builtin"]
        );
        assert_eq!(
            ids(&order(&connected, &first_run, false, None)),
            ["builtin", "usb"]
        );
    }

    #[test]
    fn a_selected_device_comes_first_with_the_ranking_as_fallback() {
        let builtin = device("builtin", DeviceKind::BuiltIn);
        let usb = device("usb", DeviceKind::Wired);
        let blackhole = device("blackhole", DeviceKind::Virtual);
        let connected = [builtin.clone(), usb.clone(), blackhole];
        let settings = MicrophoneSettings {
            selected: Some("blackhole".into()),
            ranking: vec![known(&builtin), known(&usb)],
        };
        assert_eq!(
            ids(&order(&connected, &settings, false, None)),
            ["blackhole", "builtin", "usb"]
        );
        let missing = MicrophoneSettings {
            selected: Some("gone".into()),
            ..settings
        };
        assert_eq!(
            ids(&order(&connected, &missing, false, None)),
            ["builtin", "usb"]
        );
    }

    #[test]
    fn a_closed_lid_rules_out_the_built_in_microphone() {
        let builtin = device("builtin", DeviceKind::BuiltIn);
        let usb = device("usb", DeviceKind::Wired);
        let connected = [builtin.clone(), usb.clone()];
        let settings = MicrophoneSettings {
            selected: Some("builtin".into()),
            ranking: vec![known(&builtin), known(&usb)],
        };
        assert_eq!(ids(&order(&connected, &settings, true, None)), ["usb"]);
        let list = list(&connected, &settings, true, None);
        assert_eq!(
            list.devices[0].unavailable.as_deref(),
            Some("Laptop lid is closed")
        );
        assert_eq!(list.active.as_deref(), Some("usb"));
    }

    #[test]
    fn the_list_shows_disconnected_ranked_devices_and_virtual_ones_last() {
        let builtin = device("builtin", DeviceKind::BuiltIn);
        let headset = device("headset", DeviceKind::Bluetooth);
        let blackhole = device("blackhole", DeviceKind::Virtual);
        let usb = device("usb", DeviceKind::Wired);
        let settings = MicrophoneSettings {
            selected: None,
            ranking: vec![known(&headset), known(&builtin)],
        };
        let list = list(&[builtin, blackhole, usb], &settings, false, None);
        let summary: Vec<(&str, bool, Option<usize>)> = list
            .devices
            .iter()
            .map(|entry| (entry.id.as_str(), entry.connected, entry.rank))
            .collect();
        assert_eq!(
            summary,
            [
                ("headset", false, Some(0)),
                ("builtin", true, Some(1)),
                ("usb", true, None),
                ("blackhole", true, None),
            ]
        );
        assert_eq!(list.active.as_deref(), Some("builtin"));
    }

    #[test]
    fn rankings_move_and_take_new_devices_first_or_last() {
        let a = device("a", DeviceKind::Wired);
        let b = device("b", DeviceKind::BuiltIn);
        let c = device("c", DeviceKind::Wired);
        let ranking = vec![known(&a), known(&b)];
        let ids = |ranking: &[KnownMicrophone]| -> Vec<String> {
            ranking.iter().map(|known| known.id.clone()).collect()
        };
        assert_eq!(ids(&ranked_with(&ranking, &c, true)), ["c", "a", "b"]);
        assert_eq!(ids(&ranked_with(&ranking, &c, false)), ["a", "b", "c"]);
        assert_eq!(ids(&moved(&ranking, "b", true)), ["b", "a"]);
        assert_eq!(ids(&moved(&ranking, "b", false)), ["a", "b"]);
    }
}
