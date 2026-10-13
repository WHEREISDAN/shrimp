//! tauri-plugin-mcp: lets AI agents drive the real app (DOM, screenshots,
//! input, IPC, logs) through the tauri-mcp tools. Debug builds only.

use tauri::Runtime;
use tauri::plugin::TauriPlugin;

use super::window::MAIN_LABEL;

/// The `tauri-plugin-mcp-server` MCP server connects here by default. The
/// plugin's own default is `$TMPDIR/tauri-mcp.sock`, which differs on macOS.
/// `TAURI_MCP_IPC_PATH` overrides both sides, e.g. to run two dev apps at once.
#[cfg(unix)]
const SOCKET_PATH: &str = "/tmp/tauri-mcp.sock";

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let config = tauri_plugin_mcp::PluginConfig::new("shrimp".to_string())
        .start_socket_server(true)
        .capture_rust_logs(true)
        .default_webview_label(MAIN_LABEL.to_string())
        .expose_commands(crate::commands::NAMES.iter().copied());
    #[cfg(unix)]
    let config = config.socket_path(SOCKET_PATH.into());
    tauri_plugin_mcp::init_with_config(config)
}
