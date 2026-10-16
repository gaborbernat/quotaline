//! Shares rollout scanning across GNOME and macOS while retaining GNOME's cache API.

use parking_lot::Mutex;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

pub mod reader;

const MAX_TAIL: u64 = 1024 * 1024;
const CACHE_AGE: Duration = Duration::from_secs(reader::DEFAULT_CACHE_SECONDS);

#[derive(Clone)]
pub struct Reading {
    pub data: Value,
    pub path: PathBuf,
    pub observed: Option<i64>,
}

#[derive(Default)]
pub struct RolloutCache {
    readers: Mutex<BTreeMap<PathBuf, Arc<Mutex<reader::RolloutReader>>>>,
}

impl RolloutCache {
    pub fn latest(&self, root: &Path, throttled: bool) -> io::Result<Option<Reading>> {
        self.latest_with_cache_age(root, throttled, CACHE_AGE)
    }

    pub fn latest_with_cache_age(
        &self,
        root: &Path,
        throttled: bool,
        cache_age: Duration,
    ) -> io::Result<Option<Reading>> {
        let sessions = root.join("sessions");
        match fs::metadata(&sessions) {
            Ok(metadata) if !metadata.is_dir() => return Err(io::ErrorKind::NotADirectory.into()),
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        let scanner = {
            let mut readers = self.readers.lock();
            if !readers.contains_key(root) && readers.len() >= 4 {
                readers.clear();
            }
            Arc::clone(readers.entry(root.to_path_buf()).or_insert_with(|| {
                Arc::new(Mutex::new(reader::RolloutReader::with_policy(
                    sessions,
                    reader::Budget {
                        bytes: 512 * 1024,
                        entries: 4096,
                        time: Duration::from_millis(50),
                        cache: cache_age.as_secs_f64(),
                    },
                    reader::Policy {
                        maximum_tail_bytes: Some(MAX_TAIL),
                        require_window: true,
                        refresh_empty_cache_when_throttled: true,
                    },
                )))
            }))
        };
        let mut scanner = scanner.lock();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        scanner.latest_with_cache_age(now, throttled, cache_age);
        while scanner.pending() {
            scanner.run_slice();
        }
        Ok(scanner.reading().map(|reading| Reading {
            data: reading.data,
            path: reading.path,
            observed: reading.observed_at.map(|timestamp| timestamp as i64),
        }))
    }
}

#[cfg(test)]
#[path = "../../tests/unit/codex/rollouts/tests.rs"]
mod tests;
