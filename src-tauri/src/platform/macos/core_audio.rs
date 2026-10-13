//! Core Audio device properties: the input devices with their UID, name and
//! transport (MIC-02, MIC-06, MIC-08), and the default output device's
//! state, mute and volume (SND-03).

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectIsPropertySettable, AudioObjectPropertyAddress, AudioObjectPropertyScope,
    AudioObjectPropertySelector, AudioObjectSetPropertyData,
    kAudioDevicePropertyDeviceIsRunningSomewhere, kAudioDevicePropertyDeviceUID,
    kAudioDevicePropertyMute, kAudioDevicePropertyStreams, kAudioDevicePropertyTransportType,
    kAudioDevicePropertyVolumeScalar, kAudioDeviceTransportTypeAggregate,
    kAudioDeviceTransportTypeBluetooth, kAudioDeviceTransportTypeBluetoothLE,
    kAudioDeviceTransportTypeBuiltIn, kAudioDeviceTransportTypeVirtual,
    kAudioHardwarePropertyDefaultInputDevice, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioHardwarePropertyDevices, kAudioObjectPropertyElementMain, kAudioObjectPropertyName,
    kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeInput,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
};
use objc2_core_foundation::{CFRetained, CFString};

use super::iokit;
use crate::audio::devices::classify;
use crate::audio::{AudioSource, DeviceKind, DeviceProvider, InputDevice, MicrophoneSource};

const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

fn address(
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: kAudioObjectPropertyElementMain,
    }
}

fn size(object: AudioObjectID, address: &AudioObjectPropertyAddress) -> Option<u32> {
    let mut size = 0u32;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            object,
            NonNull::from(address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
        )
    };
    (status == 0).then_some(size)
}

/// Reads a fixed-size property.
fn get<T: Copy + Default>(
    object: AudioObjectID,
    address: &AudioObjectPropertyAddress,
) -> Option<T> {
    let mut value = T::default();
    let mut size = std::mem::size_of::<T>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    (status == 0).then_some(value)
}

fn set<T: Copy>(object: AudioObjectID, address: &AudioObjectPropertyAddress, value: T) -> bool {
    let mut settable: u8 = 0;
    let status = unsafe {
        AudioObjectIsPropertySettable(object, NonNull::from(address), NonNull::from(&mut settable))
    };
    if status != 0 || settable == 0 {
        return false;
    }
    let mut value = value;
    let status = unsafe {
        AudioObjectSetPropertyData(
            object,
            NonNull::from(address),
            0,
            std::ptr::null(),
            std::mem::size_of::<T>() as u32,
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    status == 0
}

fn string(object: AudioObjectID, selector: AudioObjectPropertySelector) -> Option<String> {
    let address = address(selector, kAudioObjectPropertyScopeGlobal);
    let raw: *const CFString = get(object, &address).map(|ptr: usize| ptr as *const CFString)?;
    let raw = NonNull::new(raw.cast_mut())?;
    // SAFETY: these properties return a CFString under the create rule.
    Some(unsafe { CFRetained::from_raw(raw) }.to_string())
}

fn devices() -> Vec<AudioObjectID> {
    let address = address(
        kAudioHardwarePropertyDevices,
        kAudioObjectPropertyScopeGlobal,
    );
    let Some(bytes) = size(SYSTEM, &address) else {
        return Vec::new();
    };
    let mut ids = vec![0 as AudioObjectID; bytes as usize / std::mem::size_of::<AudioObjectID>()];
    let mut size = bytes;
    let status = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::new(ids.as_mut_ptr().cast::<c_void>()).expect("a Vec's pointer is never null"),
        )
    };
    if status != 0 {
        return Vec::new();
    }
    ids
}

fn has_input(device: AudioObjectID) -> bool {
    let address = address(kAudioDevicePropertyStreams, kAudioObjectPropertyScopeInput);
    size(device, &address).is_some_and(|bytes| bytes > 0)
}

fn kind(device: AudioObjectID, name: &str) -> DeviceKind {
    let address = address(
        kAudioDevicePropertyTransportType,
        kAudioObjectPropertyScopeGlobal,
    );
    let transport = get::<u32>(device, &address).unwrap_or_default();
    let transport = if transport == kAudioDeviceTransportTypeBuiltIn {
        DeviceKind::BuiltIn
    } else if [
        kAudioDeviceTransportTypeBluetooth,
        kAudioDeviceTransportTypeBluetoothLE,
    ]
    .contains(&transport)
    {
        DeviceKind::Bluetooth
    } else if [
        kAudioDeviceTransportTypeVirtual,
        kAudioDeviceTransportTypeAggregate,
    ]
    .contains(&transport)
    {
        DeviceKind::Virtual
    } else {
        DeviceKind::Wired
    };
    classify(name, transport)
}

fn uid(device: AudioObjectID) -> Option<String> {
    string(device, kAudioDevicePropertyDeviceUID)
}

/// Input devices through Core Audio, opened through cpal by UID.
pub struct CoreAudioDevices;

impl DeviceProvider for CoreAudioDevices {
    fn input_devices(&self) -> Vec<InputDevice> {
        devices()
            .into_iter()
            .filter(|device| has_input(*device))
            .filter_map(|device| {
                let name = string(device, kAudioObjectPropertyName)?;
                Some(InputDevice {
                    id: uid(device)?,
                    kind: kind(device, &name),
                    name,
                })
            })
            .collect()
    }

    fn default_input(&self) -> Option<String> {
        let address = address(
            kAudioHardwarePropertyDefaultInputDevice,
            kAudioObjectPropertyScopeGlobal,
        );
        get::<AudioObjectID>(SYSTEM, &address)
            .filter(|device| *device != 0 && has_input(*device))
            .and_then(uid)
    }

    fn lid_closed(&self) -> bool {
        iokit::lid_closed()
    }

    fn open(&self, device: &InputDevice) -> Box<dyn AudioSource> {
        Box::new(MicrophoneSource::with_id(device.id.clone()))
    }
}

/// The default output device.
pub fn default_output() -> Option<AudioObjectID> {
    let address = address(
        kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal,
    );
    get::<AudioObjectID>(SYSTEM, &address).filter(|device| *device != 0)
}

/// Some process is playing through `device`.
pub fn is_running(device: AudioObjectID) -> bool {
    let address = address(
        kAudioDevicePropertyDeviceIsRunningSomewhere,
        kAudioObjectPropertyScopeGlobal,
    );
    get::<u32>(device, &address).is_some_and(|running| running != 0)
}

pub fn muted(device: AudioObjectID) -> Option<bool> {
    let address = address(kAudioDevicePropertyMute, kAudioObjectPropertyScopeOutput);
    get::<u32>(device, &address).map(|muted| muted != 0)
}

pub fn set_muted(device: AudioObjectID, muted: bool) -> bool {
    let address = address(kAudioDevicePropertyMute, kAudioObjectPropertyScopeOutput);
    set(device, &address, u32::from(muted))
}

pub fn volume(device: AudioObjectID) -> Option<f32> {
    let address = address(
        kAudioDevicePropertyVolumeScalar,
        kAudioObjectPropertyScopeOutput,
    );
    get::<f32>(device, &address)
}

pub fn set_volume(device: AudioObjectID, volume: f32) -> bool {
    let address = address(
        kAudioDevicePropertyVolumeScalar,
        kAudioObjectPropertyScopeOutput,
    );
    set(device, &address, volume)
}
