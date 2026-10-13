//! Opens URLs in the default browser (CMD-05).

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

use crate::platform::{Launcher, PlatformError};

pub struct MacLauncher;

impl Launcher for MacLauncher {
    fn open_url(&self, url: &str) -> Result<(), PlatformError> {
        let url = NSURL::URLWithString(&NSString::from_str(url))
            .ok_or_else(|| PlatformError::Failed("not a valid URL".into()))?;
        if NSWorkspace::sharedWorkspace().openURL(&url) {
            Ok(())
        } else {
            Err(PlatformError::Failed("the browser did not open".into()))
        }
    }
}
