//! Local Whisper runs on the GPU through whisper.cpp's Vulkan backend. Its one
//! direct import from the Vulkan loader (`vulkan-1.dll`, installed by GPU
//! drivers) is delay-loaded (see build.rs), so Shrimp starts without it. When
//! the loader is missing, this hook hands whisper.cpp a `vkGetInstanceProcAddr`
//! whose functions all fail; Vulkan then reports no device and ggml stays on
//! the CPU.

use std::ffi::{c_char, c_void};

use windows::Win32::System::LibraryLoader::GetModuleHandleW;

/// `dliFailLoadLib` and `dliFailGetProc` from delayimp.h.
const FAIL_LOAD_LIB: u32 = 3;
const FAIL_GET_PROC: u32 = 4;
/// `VK_ERROR_INITIALIZATION_FAILED`.
const VK_INITIALIZATION_FAILED: i32 = -3;

type DelayHook = unsafe extern "system" fn(u32, *const DelayLoadInfo) -> *const c_void;

/// The start of delayimp.h's `DelayLoadInfo`.
#[repr(C)]
pub struct DelayLoadInfo {
    size: u32,
    descriptor: *const c_void,
    import: *mut *const c_void,
    dll: *const c_char,
}

/// Read by the delay-load helper when loading a delay-loaded DLL fails.
#[unsafe(no_mangle)]
#[used]
pub static __pfnDliFailureHook2: Option<DelayHook> = Some(delay_load_failed);

fn is_vulkan_loader(info: *const DelayLoadInfo) -> bool {
    // SAFETY: the helper passes a valid DelayLoadInfo whose dll is a C string.
    unsafe { info.as_ref() }
        .filter(|info| !info.dll.is_null())
        .map(|info| unsafe { std::ffi::CStr::from_ptr(info.dll) })
        .is_some_and(|dll| dll.to_bytes().eq_ignore_ascii_case(b"vulkan-1.dll"))
}

unsafe extern "system" fn delay_load_failed(
    notify: u32,
    info: *const DelayLoadInfo,
) -> *const c_void {
    if !is_vulkan_loader(info) {
        return std::ptr::null();
    }
    match notify {
        // Any loaded module will do: looking the function up in it fails next.
        FAIL_LOAD_LIB => unsafe { GetModuleHandleW(None) }
            .map_or(std::ptr::null(), |module| module.0.cast_const()),
        FAIL_GET_PROC => missing_get_instance_proc_addr as *const c_void,
        _ => std::ptr::null(),
    }
}

/// Every Vulkan function it returns fails, whatever its arguments (x64
/// passes them in registers, so ignoring them is safe).
unsafe extern "system" fn missing_get_instance_proc_addr(
    _instance: *const c_void,
    _name: *const c_char,
) -> *const c_void {
    failing_vulkan_call as *const c_void
}

unsafe extern "system" fn failing_vulkan_call() -> i32 {
    VK_INITIALIZATION_FAILED
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(dll: &std::ffi::CStr) -> DelayLoadInfo {
        DelayLoadInfo {
            size: std::mem::size_of::<DelayLoadInfo>() as u32,
            descriptor: std::ptr::null(),
            import: std::ptr::null_mut(),
            dll: dll.as_ptr(),
        }
    }

    #[test]
    fn a_missing_loader_yields_failing_vulkan_calls() {
        let hook = __pfnDliFailureHook2.unwrap();
        let vulkan = info(c"vulkan-1.dll");
        let module = unsafe { hook(FAIL_LOAD_LIB, &vulkan) };
        assert!(!module.is_null());
        let other = info(c"other.dll");
        assert!(unsafe { hook(FAIL_LOAD_LIB, &other) }.is_null());
        let get = unsafe { hook(FAIL_GET_PROC, &vulkan) };
        let get: unsafe extern "system" fn(*const c_void, *const c_char) -> *const c_void =
            unsafe { std::mem::transmute(get) };
        let call = unsafe { get(std::ptr::null(), c"vkEnumerateInstanceVersion".as_ptr()) };
        let call: unsafe extern "system" fn() -> i32 = unsafe { std::mem::transmute(call) };
        assert_eq!(unsafe { call() }, VK_INITIALIZATION_FAILED);
        assert!(unsafe { hook(1, &vulkan) }.is_null());
    }
}
