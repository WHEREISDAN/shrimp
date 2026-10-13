//! Tauri IPC commands. Keep these thin: parse input, call into a domain
//! module, map errors. Register new commands in `lib.rs` and in `NAMES`.

pub mod app;

/// Every registered command, advertised to the dev MCP bridge.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub const NAMES: &[&str] = &["app_info"];
