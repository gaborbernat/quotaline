//! Exposes the shared incremental Codex rollout reader to Swift.

use super::codex_usage::CodexRollout;
use quotaline_credential_sources::codex_rollouts::reader::{
    Budget, DEFAULT_BUDGET, MAX_FILES, MAX_LINE_BYTES, RolloutReader, Workload,
};
use std::sync::Arc;
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct CodexRolloutDefaults {
    pub max_files: u32,
    pub maximum_line_bytes: u64,
    pub cache_seconds: f64,
    pub byte_budget: u64,
    pub entry_budget: u64,
    pub time_seconds: f64,
}

#[uniffi::export]
pub fn codex_rollout_defaults() -> CodexRolloutDefaults {
    CodexRolloutDefaults {
        max_files: MAX_FILES as u32,
        maximum_line_bytes: MAX_LINE_BYTES as u64,
        cache_seconds: DEFAULT_BUDGET.cache,
        byte_budget: DEFAULT_BUDGET.bytes as u64,
        entry_budget: DEFAULT_BUDGET.entries as u64,
        time_seconds: DEFAULT_BUDGET.time.as_secs_f64(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexRolloutWorkloadRecord {
    pub trees_scanned: u64,
    pub tree_entries_examined: u64,
    pub stat_checks: u64,
    pub files_opened: u64,
    pub bytes_read: u64,
    pub largest_slice_bytes_read: u64,
    pub largest_tree_slice_entries: u64,
    pub searches_completed: u64,
}

impl From<Workload> for CodexRolloutWorkloadRecord {
    fn from(value: Workload) -> Self {
        Self {
            trees_scanned: value.trees_scanned,
            tree_entries_examined: value.tree_entries_examined,
            stat_checks: value.stat_checks,
            files_opened: value.files_opened,
            bytes_read: value.bytes_read,
            largest_slice_bytes_read: value.largest_slice_bytes_read,
            largest_tree_slice_entries: value.largest_tree_slice_entries,
            searches_completed: value.searches_completed,
        }
    }
}

#[derive(uniffi::Object)]
pub struct CodexRolloutScanner {
    state: Mutex<RolloutReader>,
}

#[uniffi::export]
impl CodexRolloutScanner {
    #[uniffi::constructor]
    pub fn new(root: String, cache_seconds: f64, byte_budget: u64, entry_budget: u64, time_seconds: f64) -> Self {
        Self {
            state: Mutex::new(RolloutReader::new(
                PathBuf::from(root),
                Budget {
                    bytes: byte_budget as usize,
                    entries: entry_budget as usize,
                    time: Duration::from_secs_f64(time_seconds.max(0.001)),
                    cache: cache_seconds,
                },
            )),
        }
    }

    pub fn latest(&self, now: f64, throttled: bool) -> Option<Arc<CodexRollout>> {
        self.lock().latest(now, throttled).map(|reading| {
            Arc::new(CodexRollout::new(
                &reading.data,
                reading.reference_now,
                reading.observed_at,
            ))
        })
    }

    pub fn start_scan(&self, now: f64) {
        self.lock().start_scan(now);
    }

    pub fn run_slice(&self) {
        self.lock().run_slice();
    }

    pub fn cancel(&self) {
        self.lock().cancel();
    }

    pub fn pending(&self) -> bool {
        self.lock().pending()
    }

    pub fn tree_pending(&self) -> bool {
        self.lock().tree_pending()
    }

    pub fn candidate_paths(&self) -> Vec<String> {
        self.lock()
            .candidate_paths()
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    }

    pub fn workload(&self) -> CodexRolloutWorkloadRecord {
        self.lock().workload().into()
    }
}

impl CodexRolloutScanner {
    fn lock(&self) -> MutexGuard<'_, RolloutReader> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/codex_rollout_reader/tests.rs"]
mod tests;
