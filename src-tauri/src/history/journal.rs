//! The recording in progress, written to disk as it arrives so a quit or a
//! crash loses at most a second of it (HIS-06). Each run of one format is
//! its own WAV file, `active-<started>-<part>.wav`; the next launch turns
//! leftovers into an "Interrupted" history entry.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::audio::{AudioClip, AudioFormat, read_wav};

const PREFIX: &str = "active-";
const FLUSH_EVERY: Duration = Duration::from_secs(1);

pub struct Journal {
    dir: PathBuf,
    started: i64,
    parts: Vec<PathBuf>,
    writer: Option<(AudioFormat, WavWriter<BufWriter<File>>)>,
    flushed: Instant,
}

impl Journal {
    pub fn new(dir: &Path, started: i64) -> Self {
        Self {
            dir: dir.to_path_buf(),
            started,
            parts: Vec::new(),
            writer: None,
            flushed: Instant::now(),
        }
    }

    /// Appends samples; a new format starts a new part.
    pub fn write(&mut self, format: AudioFormat, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        if self.writer.as_ref().map(|(current, _)| *current) != Some(format) {
            self.close();
            match self.open(format) {
                Ok(writer) => self.writer = Some((format, writer)),
                Err(error) => {
                    log::warn!("could not save the recording as it happens: {error}");
                    return;
                }
            }
        }
        let Some((_, writer)) = self.writer.as_mut() else {
            return;
        };
        if samples
            .iter()
            .try_for_each(|sample| writer.write_sample(*sample))
            .is_err()
        {
            log::warn!("could not save part of the recording");
            self.writer = None;
            return;
        }
        if self.flushed.elapsed() >= FLUSH_EVERY {
            let _ = writer.flush();
            self.flushed = Instant::now();
        }
    }

    fn open(&mut self, format: AudioFormat) -> Result<WavWriter<BufWriter<File>>, hound::Error> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self
            .dir
            .join(format!("{PREFIX}{}-{}.wav", self.started, self.parts.len()));
        let spec = WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let writer = WavWriter::create(&path, spec)?;
        self.parts.push(path);
        Ok(writer)
    }

    fn close(&mut self) {
        if let Some((_, writer)) = self.writer.take()
            && let Err(error) = writer.finalize()
        {
            log::warn!("could not finish saving the recording: {error}");
        }
    }

    /// Closes the files, which stay until the dictation is stored.
    pub fn finish(mut self) -> JournalFiles {
        self.close();
        JournalFiles(std::mem::take(&mut self.parts))
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        self.close();
    }
}

/// A finished journal's files; delete them once the dictation is stored.
#[derive(Debug, Default)]
pub struct JournalFiles(Vec<PathBuf>);

impl JournalFiles {
    pub fn delete(self) {
        for path in self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Recordings left by a run that quit or crashed: when each started, and its
/// audio, oldest first. The files are deleted once read.
pub fn leftovers(dir: &Path) -> Vec<(i64, AudioClip)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut parts: Vec<(i64, usize, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_stem()?.to_str()?.strip_prefix(PREFIX)?.to_owned();
            let (started, part) = name.split_once('-')?;
            Some((started.parse().ok()?, part.parse().ok()?, path))
        })
        .collect();
    parts.sort();
    let mut recordings: Vec<(i64, Vec<AudioClip>)> = Vec::new();
    for (started, _, path) in parts {
        let clip = read_wav(&path)
            .ok()
            .and_then(|(format, samples)| AudioClip::from_interleaved(format, &samples).ok());
        let _ = std::fs::remove_file(&path);
        let Some(clip) = clip else {
            log::warn!(
                "could not read the interrupted recording {}",
                path.display()
            );
            continue;
        };
        match recordings.last_mut() {
            Some((last, clips)) if *last == started => clips.push(clip),
            _ => recordings.push((started, vec![clip])),
        }
    }
    recordings
        .into_iter()
        .map(|(started, clips)| (started, AudioClip::concat(&clips)))
        .filter(|(_, clip)| !clip.is_empty())
        .collect()
}

/// Deletes every leftover without reading it ("Never store").
pub fn discard_all(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_journal = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(PREFIX));
        if is_journal {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEREO_48K: AudioFormat = AudioFormat {
        sample_rate: 48_000,
        channels: 2,
    };
    const MONO_16K: AudioFormat = AudioFormat {
        sample_rate: 16_000,
        channels: 1,
    };

    #[test]
    fn a_crashed_recording_is_read_back_across_formats() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::new(dir.path(), 42);
        journal.write(STEREO_48K, &vec![0.25; 48_000 * 2]);
        journal.write(MONO_16K, &vec![0.25; 16_000]);
        // A crash: the journal is never finished.
        std::mem::forget(journal);
        let mut found = leftovers(dir.path());
        assert_eq!(found.len(), 1);
        let (started, clip) = found.remove(0);
        assert_eq!(started, 42);
        assert!(
            clip.duration() >= Duration::from_millis(990),
            "{:?}",
            clip.duration()
        );
        assert!(
            leftovers(dir.path()).is_empty(),
            "files are deleted once read"
        );
    }

    #[test]
    fn a_finished_journal_is_deleted_after_storing() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::new(dir.path(), 7);
        journal.write(MONO_16K, &[0.1; 1_600]);
        let files = journal.finish();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        files.delete();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
