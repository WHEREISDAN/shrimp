//! The picker's live level meter (MIC-02): records from a microphone and
//! reports its level about 20 times a second until dropped.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::audio::{AudioSource, Recorder, level};

const INTERVAL: Duration = Duration::from_millis(50);

pub struct Preview {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Preview {
    pub fn start(source: Box<dyn AudioSource>, on_level: Box<dyn Fn(f32) + Send + Sync>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let running = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("shrimp-mic-preview".into())
            .spawn(move || {
                let mut recorder = match Recorder::start(source) {
                    Ok(recorder) => recorder,
                    Err(error) => {
                        log::warn!("level meter unavailable: {error}");
                        return;
                    }
                };
                while !running.load(Ordering::SeqCst) {
                    std::thread::sleep(INTERVAL);
                    match recorder.poll() {
                        Ok(samples) if !samples.is_empty() => {
                            on_level(level::display_level(level::rms(samples)));
                        }
                        Ok(_) => {}
                        Err(error) => {
                            log::warn!("level meter stopped: {error}");
                            on_level(0.0);
                            return;
                        }
                    }
                }
            })
            .ok();
        Self { stop, handle }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
