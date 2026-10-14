//! Restarting Shrimp (APP-08, APP-09, APP-12). Tauri's restart opens the new
//! instance while this one still runs, so the single-instance check hands
//! over to this one and quits, and this one then exits: nothing is left.
//! Instead a small shell waits for this process to end, then opens the
//! bundle through LaunchServices with `RELAUNCH_ARG`.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::platform::{PlatformError, RELAUNCH_ARG};
use crate::t;

/// Opens the app again once this process has exited. The caller exits next.
pub fn relaunch_after_exit() -> Result<(), PlatformError> {
    let executable = std::env::current_exe()
        .map_err(|err| PlatformError::Failed(t!("error.relaunch.noExecutable", details = err)))?;
    let bundle = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| PlatformError::Failed(t!("error.relaunch.notBundled")))?;
    spawn_waiter(std::process::id(), bundle)
}

fn spawn_waiter(pid: u32, bundle: &Path) -> Result<(), PlatformError> {
    Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "while kill -0 {pid} 2>/dev/null; do sleep 0.2; done; \
             exec /usr/bin/open -n \"$0\" --args {RELAUNCH_ARG}"
        ))
        .arg(bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| PlatformError::Failed(t!("error.relaunch.failed", details = err)))
}
