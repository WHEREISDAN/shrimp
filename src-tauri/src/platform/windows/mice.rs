//! Connected mice and their buttons (CORE-18): the Raw Input device list,
//! and each mouse's `dwNumberOfButtons`.

use windows::Win32::UI::Input::{
    GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RID_DEVICE_INFO,
    RIDI_DEVICEINFO, RIM_TYPEMOUSE,
};

use crate::platform::MouseDevices;

pub struct RawInputMice;

impl MouseDevices for RawInputMice {
    fn most_buttons(&self) -> u32 {
        devices()
            .into_iter()
            .filter(|device| device.dwType == RIM_TYPEMOUSE)
            .filter_map(|device| buttons(&device))
            .max()
            .unwrap_or(0)
    }
}

fn devices() -> Vec<RAWINPUTDEVICELIST> {
    let size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
    let mut count = 0u32;
    if unsafe { GetRawInputDeviceList(None, &mut count, size) } == u32::MAX || count == 0 {
        return Vec::new();
    }
    let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
    let found = unsafe { GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, size) };
    // A device can come or go between the calls; the second call says how many it wrote.
    if found == u32::MAX {
        return Vec::new();
    }
    list.truncate(found as usize);
    list
}

fn buttons(device: &RAWINPUTDEVICELIST) -> Option<u32> {
    let mut info = RID_DEVICE_INFO {
        cbSize: std::mem::size_of::<RID_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    let mut size = info.cbSize;
    let written = unsafe {
        GetRawInputDeviceInfoW(
            Some(device.hDevice),
            RIDI_DEVICEINFO,
            Some(std::ptr::from_mut(&mut info).cast()),
            &mut size,
        )
    };
    if written == u32::MAX || written == 0 || info.dwType != RIM_TYPEMOUSE {
        return None;
    }
    Some(unsafe { info.Anonymous.mouse }.dwNumberOfButtons)
}
