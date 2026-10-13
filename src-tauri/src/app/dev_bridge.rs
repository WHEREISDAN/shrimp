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
/// On Windows the MCP server always connects to the pipe `\\.\pipe\tmp\tauri-mcp.sock`.
#[cfg(windows)]
const SOCKET_PATH: &str = r"tmp\tauri-mcp.sock";

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let config = tauri_plugin_mcp::PluginConfig::new("shrimp".to_string())
        .start_socket_server(true)
        .capture_rust_logs(true)
        .default_webview_label(MAIN_LABEL.to_string())
        .expose_commands(crate::commands::NAMES.iter().copied())
        .socket_path(SOCKET_PATH.into());
    // The MCP server looks for the token next to the pipe's name, where no
    // file can exist, so the debug pipe goes without one. A named pipe's
    // default security already limits it to this user and administrators.
    #[cfg(windows)]
    let config = config.insecure_no_auth();
    tauri_plugin_mcp::init_with_config(config)
}
