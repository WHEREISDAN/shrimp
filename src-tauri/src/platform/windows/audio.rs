//! Windows audio endpoints through the MMDevice API: the input devices with
//! their endpoint id, name and kind (MIC-02, MIC-06, MIC-08), and the audio
//! sessions of the default output for per-app mute (WIN-08). Recording opens
//! the cpal (WASAPI) device whose id is the endpoint id.

use windows::Win32::Foundation::{PROPERTYKEY, S_OK};
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
    IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, eCapture, eConsole,
    eMultimedia, eRender,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantToUInt32;
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantToStringAlloc};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows::core::{GUID, Interface};

use crate::audio::devices::classify;
use crate::audio::{AudioSource, DeviceKind, DeviceProvider, InputDevice, MicrophoneSource};

const DEVICE_PROPERTIES: GUID = GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0);
const ENDPOINT_PROPERTIES: GUID = GUID::from_u128(0x1da5d803_d492_4edd_8c23_e0c0ffee7f0e);
const FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: DEVICE_PROPERTIES,
    pid: 14,
};
const ENUMERATOR_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: DEVICE_PROPERTIES,
    pid: 24,
};
const FORM_FACTOR: PROPERTYKEY = PROPERTYKEY {
    fmtid: ENDPOINT_PROPERTIES,
    pid: 0,
};
const JACK_SUBTYPE: PROPERTYKEY = PROPERTYKEY {
    fmtid: ENDPOINT_PROPERTIES,
    pid: 8,
};
/// `EndpointFormFactor::Headset`, Bluetooth's usual form.
const FORM_HEADSET: u32 = 5;
/// `KSNODETYPE_MICROPHONE_ARRAY`: a laptop's or webcam's built-in array.
const MICROPHONE_ARRAY: &str = "{DFF21BE5-F70F-11D0-B917-00A0C9223196}";

/// COM on the calling thread; an STA thread (the main thread) keeps its mode.
pub fn com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

pub fn enumerator() -> Option<IMMDeviceEnumerator> {
    com();
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.ok()
}

fn string(
    store: &windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore,
    key: &PROPERTYKEY,
) -> Option<String> {
    let value: PROPVARIANT = unsafe { store.GetValue(key) }.ok()?;
    let text = unsafe { PropVariantToStringAlloc(&value) }.ok()?;
    let result = unsafe { text.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(text.0 as *const _)) };
    result.filter(|text| !text.is_empty())
}

fn number(
    store: &windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore,
    key: &PROPERTYKEY,
) -> Option<u32> {
    let value: PROPVARIANT = unsafe { store.GetValue(key) }.ok()?;
    unsafe { PropVariantToUInt32(&value) }.ok()
}

fn endpoint_id(device: &IMMDevice) -> Option<String> {
    let id = unsafe { device.GetId() }.ok()?;
    let text = unsafe { id.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(id.0 as *const _)) };
    text
}

/// The kind from the bus the device sits on and its jack.
pub fn kind_of(
    enumerator: &str,
    form_factor: Option<u32>,
    jack: Option<&str>,
    name: &str,
) -> DeviceKind {
    let enumerator = enumerator.to_ascii_uppercase();
    let transport = if enumerator.starts_with("BTH") || name.to_lowercase().contains("bluetooth") {
        DeviceKind::Bluetooth
    } else if enumerator == "ROOT" || enumerator == "SW" {
        DeviceKind::Virtual
    } else if jack.is_some_and(|jack| jack.eq_ignore_ascii_case(MICROPHONE_ARRAY))
        && enumerator != "USB"
    {
        DeviceKind::BuiltIn
    } else if form_factor == Some(FORM_HEADSET) && enumerator.is_empty() {
        DeviceKind::Bluetooth
    } else {
        DeviceKind::Wired
    };
    classify(name, transport)
}

fn describe(device: &IMMDevice) -> Option<InputDevice> {
    let id = endpoint_id(device)?;
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }.ok()?;
    let name = string(&store, &FRIENDLY_NAME)?;
    let bus = string(&store, &ENUMERATOR_NAME).unwrap_or_default();
    let jack = string(&store, &JACK_SUBTYPE);
    let kind = kind_of(&bus, number(&store, &FORM_FACTOR), jack.as_deref(), &name);
    Some(InputDevice { id, name, kind })
}

/// Input endpoints through the MMDevice API, opened through cpal by id.
pub struct WasapiDevices;

impl DeviceProvider for WasapiDevices {
    fn input_devices(&self) -> Vec<InputDevice> {
        let Some(enumerator) = enumerator() else {
            return Vec::new();
        };
        let Ok(collection) =
            (unsafe { enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) })
        else {
            return Vec::new();
        };
        let count = unsafe { collection.GetCount() }.unwrap_or(0);
        (0..count)
            .filter_map(|index| unsafe { collection.Item(index) }.ok())
            .filter_map(|device| describe(&device))
            .collect()
    }

    fn default_input(&self) -> Option<String> {
        let enumerator = enumerator()?;
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) }.ok()?;
        endpoint_id(&device)
    }

    fn open(&self, device: &InputDevice) -> Box<dyn AudioSource> {
        Box::new(MicrophoneSource::with_id(device.id.clone()))
    }
}

/// An app's audio session on the default output.
pub struct Session {
    pub instance: String,
    pub pid: u32,
    pub volume: ISimpleAudioVolume,
    /// Something is audible right now.
    pub sounding: bool,
}

/// The sessions of the default output device, except the system sounds.
pub fn output_sessions() -> Vec<Session> {
    let Some(enumerator) = enumerator() else {
        return Vec::new();
    };
    let Ok(device) = (unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }) else {
        return Vec::new();
    };
    let Ok(manager) = (unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) })
    else {
        return Vec::new();
    };
    let Ok(sessions) = (unsafe { manager.GetSessionEnumerator() }) else {
        return Vec::new();
    };
    let count = unsafe { sessions.GetCount() }.unwrap_or(0);
    (0..count)
        .filter_map(|index| unsafe { sessions.GetSession(index) }.ok())
        .filter_map(|control| {
            let control: IAudioSessionControl2 = control.cast().ok()?;
            // S_OK means it is the system sounds session; other sessions
            // answer S_FALSE, which also counts as success.
            if unsafe { control.IsSystemSoundsSession() } == S_OK {
                return None;
            }
            let active =
                unsafe { control.GetState() }.is_ok_and(|state| state == AudioSessionStateActive);
            let peak = control
                .cast::<IAudioMeterInformation>()
                .ok()
                .and_then(|meter| unsafe { meter.GetPeakValue() }.ok())
                .unwrap_or(0.0);
            let instance = unsafe { control.GetSessionInstanceIdentifier() }.ok()?;
            let instance_text = unsafe { instance.to_string() }.ok();
            unsafe { CoTaskMemFree(Some(instance.0 as *const _)) };
            Some(Session {
                instance: instance_text?,
                pid: unsafe { control.GetProcessId() }.unwrap_or(0),
                volume: control.cast().ok()?,
                sounding: active && peak > 0.0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_from_bus_and_jack() {
        assert_eq!(
            kind_of("USB", Some(4), None, "Analogue 1 + 2 (USB Audio Interface)"),
            DeviceKind::Wired
        );
        assert_eq!(
            kind_of("BTHHFENUM", Some(5), None, "Headset (WH-1000XM4)"),
            DeviceKind::Bluetooth
        );
        assert_eq!(
            kind_of(
                "HDAUDIO",
                Some(4),
                Some(MICROPHONE_ARRAY),
                "Microphone Array (Realtek)"
            ),
            DeviceKind::BuiltIn
        );
        assert_eq!(
            kind_of("ROOT", Some(4), None, "Steam Streaming Microphone"),
            DeviceKind::Virtual
        );
        assert_eq!(
            kind_of(
                "SWD",
                Some(4),
                None,
                "SteelSeries Sonar - Microphone (SteelSeries Sonar Virtual Audio Device)"
            ),
            DeviceKind::Virtual,
            "named virtual"
        );
    }

    #[test]
    fn lists_this_machines_inputs() {
        // Every listed device has an endpoint id cpal can open.
        for device in WasapiDevices.input_devices() {
            assert!(device.id.starts_with("{0.0.1."), "{device:?}");
        }
    }
}
