// No console window on Windows, in debug builds too: the dev app runs on the
// desktop like the release, and its logs go to tauri-mcp (DEVELOPMENT.md).
#![windows_subsystem = "windows"]

fn main() {
    shrimp_lib::run();
}
