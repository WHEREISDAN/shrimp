//! Where Shrimp runs from (APP-11). Run from the mounted disk image,
//! Downloads or an App Translocation path, it can't update itself, so the
//! main window asks to move it into an Applications folder first, as Wispr
//! Flow does (article 7682075140).

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::relaunch::{open_after_exit, running_bundle};
use crate::platform::PlatformError;
use crate::t;

/// Debug builds run from the build folder, so they only pretend with `1`.
const PRETEND_ENV: &str = "SHRIMP_PRETEND_OUTSIDE_APPLICATIONS";
const SYSTEM_APPLICATIONS: &str = "/Applications";

/// Whether this run is outside /Applications and ~/Applications.
pub fn outside_applications() -> bool {
    if cfg!(debug_assertions) {
        return std::env::var(PRETEND_ENV).is_ok_and(|value| value == "1");
    }
    running_bundle().is_ok_and(|bundle| !in_applications(&bundle, &home()))
}

/// Copies the running app into /Applications, or ~/Applications when that
/// isn't writable, and opens the copy once this process exits. Returns
/// whether the caller should quit now: debug builds only log what they
/// would copy, since the dev bundle must stay where the build put it.
pub fn move_to_applications() -> Result<bool, PlatformError> {
    let source = running_bundle().or_else(|error| {
        // `tauri dev` runs no bundle; a debug build only names it.
        if cfg!(debug_assertions) {
            std::env::current_exe().map_err(|_| error)
        } else {
            Err(error)
        }
    })?;
    let name = source
        .file_name()
        .ok_or_else(|| PlatformError::Failed(t!("error.relaunch.notBundled")))?;
    let target = applications_folder().join(name);
    if cfg!(debug_assertions) {
        log::info!(
            "a release would copy {} to {} and open the copy",
            source.display(),
            target.display()
        );
        return Ok(false);
    }
    copy_bundle(&source, &target)?;
    log::info!("copied Shrimp to {}", target.display());
    open_after_exit(&target)?;
    Ok(true)
}

/// Inside either Applications folder, at any depth.
fn in_applications(bundle: &Path, home: &Path) -> bool {
    bundle.starts_with(SYSTEM_APPLICATIONS) || bundle.starts_with(home.join("Applications"))
}

fn home() -> PathBuf {
    PathBuf::from(objc2_foundation::NSHomeDirectory().to_string())
}

/// /Applications for an administrator, else the user's own folder.
fn applications_folder() -> PathBuf {
    let system = Path::new(SYSTEM_APPLICATIONS);
    if writable(system) {
        system.to_path_buf()
    } else {
        home().join("Applications")
    }
}

fn writable(folder: &Path) -> bool {
    let Ok(path) = CString::new(folder.as_os_str().as_encoded_bytes()) else {
        return false;
    };
    unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
}

/// `ditto` keeps the bundle's symlinks, permissions and signature. Without
/// the quarantine flag the copy isn't translocated again when it opens. An
/// older copy at `target` is replaced only once the new one is complete.
fn copy_bundle(source: &Path, target: &Path) -> Result<(), PlatformError> {
    let folder = target.parent().unwrap_or(Path::new(SYSTEM_APPLICATIONS));
    let failed = |details: String| {
        PlatformError::Failed(t!(
            "error.install.copyFailed",
            folder = folder.display(),
            details = details
        ))
    };
    std::fs::create_dir_all(folder).map_err(|err| failed(err.to_string()))?;
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let staging = folder.join(format!(".{name}.moving"));
    let _ = std::fs::remove_dir_all(&staging);
    let copied = Command::new("/usr/bin/ditto")
        .arg("--noqtn")
        .arg(source)
        .arg(&staging)
        .output()
        .map_err(|err| failed(err.to_string()))?;
    if !copied.status.success() {
        let _ = std::fs::remove_dir_all(&staging);
        let details = String::from_utf8_lossy(&copied.stderr).trim().to_owned();
        return Err(failed(details));
    }
    if target.exists() {
        std::fs::remove_dir_all(target).map_err(|err| failed(err.to_string()))?;
    }
    std::fs::rename(&staging, target).map_err(|err| failed(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_applications_folders_count() {
        let home = Path::new("/Users/dan");
        let inside = |path: &str| in_applications(Path::new(path), home);
        assert!(inside("/Applications/Shrimp.app"));
        assert!(inside("/Applications/Utilities/Shrimp.app"));
        assert!(inside("/Users/dan/Applications/Shrimp.app"));
        assert!(!inside("/Volumes/Shrimp/Shrimp.app"), "the disk image");
        assert!(!inside("/Users/dan/Downloads/Shrimp.app"));
        assert!(!inside(
            "/private/var/folders/x/T/AppTranslocation/3F2A/d/Shrimp.app"
        ));
        assert!(!inside("/ApplicationsOld/Shrimp.app"));
        assert!(!inside("/Users/other/Applications/Shrimp.app"));
    }

    #[test]
    fn copies_a_bundle_and_replaces_an_older_one() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("from/Shrimp.app");
        std::fs::create_dir_all(source.join("Contents/MacOS")).unwrap();
        std::fs::write(source.join("Contents/MacOS/shrimp"), "new").unwrap();
        let target = dir.path().join("Applications/Shrimp.app");
        std::fs::create_dir_all(target.join("Contents")).unwrap();
        std::fs::write(target.join("Contents/old"), "old").unwrap();

        copy_bundle(&source, &target).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Contents/MacOS/shrimp")).unwrap(),
            "new"
        );
        assert!(!target.join("Contents/old").exists());
        assert!(!dir.path().join("Applications/.Shrimp.app.moving").exists());
        assert!(source.exists(), "a copy, not a move");
    }

    #[test]
    fn a_failed_copy_keeps_the_older_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Applications/Shrimp.app");
        std::fs::create_dir_all(&target).unwrap();
        let error = copy_bundle(&dir.path().join("missing.app"), &target).unwrap_err();
        assert!(error.to_string().contains("Applications"), "{error}");
        assert!(target.exists());
    }
}
