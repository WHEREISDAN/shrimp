fn main() {
    // whisper.cpp's Vulkan backend imports vulkan-1.dll, which GPU drivers
    // install. Loading it on first use lets Shrimp start without it and run
    // Whisper on the CPU (src/platform/windows/gpu.rs).
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if windows && std::env::var_os("CARGO_FEATURE_VULKAN").is_some() {
        println!("cargo:rustc-link-arg=/DELAYLOAD:vulkan-1.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }
    tauri_build::build()
}
