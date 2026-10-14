//! Exposes the shared Claude transcript reader to macOS.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use quotaline_history::claude::{Config, Reader, Workload};

use super::state::ClaudeTranscriptView;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ClaudeScannerConfig {
    pub file_scan_interval: f64,
    pub checkpoint_interval: f64,
    pub maximum_checkpoint_bytes: u64,
    pub byte_budget: u64,
    pub entry_budget: u64,
    pub time_budget_seconds: f64,
    pub maximum_line_bytes: u64,
    pub maximum_retained_partial_bytes: u64,
    pub maximum_indexed_files: u64,
}

impl Default for ClaudeScannerConfig {
    fn default() -> Self {
        let config = Config::default();
        Self {
            file_scan_interval: config.file_scan_interval,
            checkpoint_interval: config.checkpoint_interval,
            maximum_checkpoint_bytes: config.maximum_checkpoint_bytes as u64,
            byte_budget: config.byte_budget as u64,
            entry_budget: config.entry_budget as u64,
            time_budget_seconds: config.time_budget.as_secs_f64(),
            maximum_line_bytes: config.maximum_line_bytes as u64,
            maximum_retained_partial_bytes: config.maximum_retained_partial_bytes as u64,
            maximum_indexed_files: quotaline_history::claude::MAX_INDEXED_FILES as u64,
        }
    }
}

#[uniffi::export]
pub fn claude_scanner_defaults() -> ClaudeScannerConfig {
    ClaudeScannerConfig::default()
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ClaudeScannerWorkload {
    pub scans_started: u64,
    pub scans_completed: u64,
    pub tree_entries_examined: u64,
    pub stat_checks: u64,
    pub files_opened: u64,
    pub bytes_read: u64,
    pub largest_slice_bytes_read: u64,
    pub largest_slice_entries_examined: u64,
    pub checkpoint_attempts: u64,
    pub checkpoints: u64,
    pub checkpoint_bytes_written: u64,
    pub last_checkpoint_bytes: u64,
    pub retained_partial_files: u64,
    pub retained_partial_bytes: u64,
    pub largest_retained_partial_bytes: u64,
}

impl From<Workload> for ClaudeScannerWorkload {
    fn from(value: Workload) -> Self {
        Self {
            scans_started: value.scans_started,
            scans_completed: value.scans_completed,
            tree_entries_examined: value.tree_entries_examined,
            stat_checks: value.stat_checks,
            files_opened: value.files_opened,
            bytes_read: value.bytes_read,
            largest_slice_bytes_read: value.largest_slice_bytes_read,
            largest_slice_entries_examined: value.largest_slice_entries_examined,
            checkpoint_attempts: value.checkpoint_attempts,
            checkpoints: value.checkpoints,
            checkpoint_bytes_written: value.checkpoint_bytes_written,
            last_checkpoint_bytes: value.last_checkpoint_bytes,
            retained_partial_files: value.retained_partial_files,
            retained_partial_bytes: value.retained_partial_bytes,
            largest_retained_partial_bytes: value.largest_retained_partial_bytes,
        }
    }
}

#[derive(uniffi::Record)]
pub struct ClaudeScannerSnapshot {
    pub view: Arc<ClaudeTranscriptView>,
    pub pricing_warnings: Vec<String>,
}

#[derive(uniffi::Object)]
pub struct ClaudeTranscriptScanner {
    reader: Mutex<Reader>,
}

#[uniffi::export]
impl ClaudeTranscriptScanner {
    #[uniffi::constructor]
    pub fn new(root: String, state_path: Option<String>, config: ClaudeScannerConfig) -> Self {
        let time_budget =
            Duration::try_from_secs_f64(config.time_budget_seconds).unwrap_or_else(|_| Duration::from_millis(1));
        Self {
            reader: Mutex::new(Reader::new(
                PathBuf::from(root),
                state_path.map(PathBuf::from),
                Config {
                    file_scan_interval: config.file_scan_interval,
                    checkpoint_interval: config.checkpoint_interval,
                    maximum_checkpoint_bytes: config.maximum_checkpoint_bytes as usize,
                    byte_budget: config.byte_budget as usize,
                    entry_budget: config.entry_budget as usize,
                    time_budget,
                    maximum_line_bytes: config.maximum_line_bytes as usize,
                    maximum_retained_partial_bytes: config.maximum_retained_partial_bytes as usize,
                },
            )),
        }
    }

    pub fn refresh(&self, now: f64, retention_days: u32, throttled: bool) -> ClaudeScannerSnapshot {
        let snapshot = self.reader.lock().unwrap().refresh(now, retention_days, throttled);
        ClaudeScannerSnapshot {
            view: ClaudeTranscriptView::from_state(snapshot.state, snapshot.projects_complete),
            pricing_warnings: quotaline_claude::claude_transcripts::pricing_warnings(&snapshot.unpriced_models),
        }
    }

    pub fn run_background_slice(&self) {
        self.reader.lock().unwrap().run_background_slice();
    }

    pub fn pending(&self) -> bool {
        self.reader.lock().unwrap().pending()
    }

    pub fn cancel(&self) {
        self.reader.lock().unwrap().cancel();
    }

    pub fn workload(&self) -> ClaudeScannerWorkload {
        self.reader.lock().unwrap().workload().into()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/claude/scanner/tests.rs"]
mod tests;
