use std::process::Command;

fn main() {
    link_clang_runtime();
    tauri_build::build()
}

/// whisper.cpp's Metal backend checks the macOS version with `@available`. Below a
/// deployment target of macOS 15 (release builds target 14.2) that calls
/// `__isPlatformVersionAtLeast` from clang's runtime, which rustc leaves out
/// (`-nodefaultlibs`). The archive goes last on the link line, so it only fills gaps.
fn link_clang_runtime() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let output = Command::new("xcrun")
        .args(["clang", "--print-runtime-dir"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let dir = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            println!("cargo:rustc-link-arg={dir}/libclang_rt.osx.a");
        }
        _ => println!("cargo:warning=clang's runtime wasn't found; release builds may not link"),
    }
}
