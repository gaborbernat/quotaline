//! Owns bounded transcript discovery, aggregate state, and checkpoints.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use quotaline_claude::claude_transcripts::TranscriptState;

mod checkpoint;
mod ingest;
mod scan;

pub const MAX_INDEXED_FILES: usize = 64;
pub const READ_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub file_scan_interval: f64,
    pub checkpoint_interval: f64,
    pub maximum_checkpoint_bytes: usize,
    pub byte_budget: usize,
    pub entry_budget: usize,
    pub time_budget: Duration,
    pub maximum_line_bytes: usize,
    pub maximum_retained_partial_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_scan_interval: 15.0 * 60.0,
            checkpoint_interval: 5.0 * 60.0,
            maximum_checkpoint_bytes: 1024 * 1024,
            byte_budget: 8 * 1024 * 1024,
            entry_budget: 256,
            time_budget: Duration::from_millis(50),
            maximum_line_bytes: 16 * 1024 * 1024,
            maximum_retained_partial_bytes: 16 * 1024 * 1024,
        }
    }
}

impl Config {
    fn bounded(self) -> Self {
        Self {
            maximum_checkpoint_bytes: self.maximum_checkpoint_bytes.max(1),
            byte_budget: self.byte_budget.max(1),
            entry_budget: self.entry_budget.max(1),
            time_budget: self.time_budget.max(Duration::from_millis(1)),
            maximum_line_bytes: self.maximum_line_bytes.max(1),
            maximum_retained_partial_bytes: self.maximum_retained_partial_bytes.max(self.maximum_line_bytes.max(1)),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Workload {
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

pub struct Snapshot {
    pub state: Arc<TranscriptState>,
    pub unpriced_models: Vec<String>,
    pub projects_complete: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct TranscriptFile {
    path: PathBuf,
    size: u64,
    modified: f64,
}

struct PartialRead {
    committed_offset: u64,
    cursor: u64,
    pending: Vec<u8>,
    oversized: bool,
}

struct IngestResult {
    changed: bool,
    bytes_read: usize,
    complete: bool,
}

impl IngestResult {
    fn failed() -> Self {
        Self {
            changed: false,
            bytes_read: 0,
            complete: true,
        }
    }
}

pub struct Reader {
    root: PathBuf,
    state_path: Option<PathBuf>,
    config: Config,
    state: Arc<TranscriptState>,
    indexed_files: BTreeMap<String, TranscriptFile>,
    partial_reads: BTreeMap<String, PartialRead>,
    pending_tails: Vec<TranscriptFile>,
    pending_tail_index: usize,
    scan: Option<scan::TreeScan>,
    next_file_scan_at: f64,
    next_checkpoint_at: f64,
    next_prune_at: f64,
    work_reference_now: f64,
    work_cutoff: f64,
    uncheckpointed_bytes: usize,
    checkpoint_retry_at: Option<f64>,
    state_dirty: bool,
    state_loaded: bool,
    has_scanned: bool,
    retention_days: u32,
    scan_generation: u64,
    reported_unpriced_models: BTreeSet<String>,
    workload: Workload,
}

impl Reader {
    pub fn new(root: PathBuf, state_path: Option<PathBuf>, config: Config) -> Self {
        Self {
            root: root.canonicalize().unwrap_or(root),
            state_path,
            config: config.bounded(),
            state: Arc::new(TranscriptState::default()),
            indexed_files: BTreeMap::new(),
            partial_reads: BTreeMap::new(),
            pending_tails: Vec::new(),
            pending_tail_index: 0,
            scan: None,
            next_file_scan_at: f64::NEG_INFINITY,
            next_checkpoint_at: f64::INFINITY,
            next_prune_at: f64::NEG_INFINITY,
            work_reference_now: f64::NEG_INFINITY,
            work_cutoff: f64::NEG_INFINITY,
            uncheckpointed_bytes: 0,
            checkpoint_retry_at: None,
            state_dirty: false,
            state_loaded: false,
            has_scanned: false,
            retention_days: quotaline_settings::DEFAULT_HISTORY_RETENTION_DAYS,
            scan_generation: 0,
            reported_unpriced_models: BTreeSet::new(),
            workload: Workload::default(),
        }
    }

    pub fn refresh(&mut self, now: f64, retention_days: u32, throttled: bool) -> Snapshot {
        self.load_state();
        let retention_days = quotaline_settings::normalize_history_retention_days(retention_days);
        self.retention_days = retention_days;
        if Arc::make_mut(&mut self.state).configure_retention(retention_days) {
            self.mark_dirty(0);
            self.scan = None;
            self.has_scanned = false;
            self.pending_tails.clear();
            self.pending_tail_index = 0;
            self.next_file_scan_at = f64::NEG_INFINITY;
            self.next_prune_at = f64::NEG_INFINITY;
        }
        self.work_reference_now = now;
        self.work_cutoff = now - f64::from(retention_days) * 86_400.0;
        if now >= self.next_prune_at {
            if Arc::make_mut(&mut self.state).prune(now, retention_days) {
                self.mark_dirty(0);
            }
            self.next_prune_at = (now / 86_400.0).floor() * 86_400.0 + 86_400.0;
        }
        self.prepare_indexed_tails();
        if self.scan.is_none() && (!self.has_scanned || now >= self.next_file_scan_at && !throttled) {
            self.begin_scan(now);
        }
        self.run_slice();
        self.checkpoint_if_needed(now);
        let unpriced_models = self
            .state
            .unpriced_models()
            .into_iter()
            .filter(|model| self.reported_unpriced_models.insert(model.clone()))
            .collect();
        Snapshot {
            state: self.state.clone(),
            unpriced_models,
            projects_complete: self.has_scanned
                && self.scan.is_none()
                && self.pending_tail_index >= self.pending_tails.len()
                && self.partial_reads.is_empty(),
        }
    }

    pub fn run_background_slice(&mut self) {
        if self.pending() {
            self.run_slice();
            self.checkpoint_if_needed(self.work_reference_now);
        }
    }

    pub fn pending(&self) -> bool {
        self.pending_tail_index < self.pending_tails.len() || self.scan.is_some()
    }

    pub fn request_discovery(&mut self) {
        self.next_file_scan_at = f64::NEG_INFINITY;
    }

    pub fn cancel(&mut self) {
        self.scan = None;
        self.has_scanned = false;
        self.pending_tails.clear();
        self.pending_tail_index = 0;
        self.partial_reads.clear();
        self.record_partial_workload();
    }

    pub fn workload(&self) -> Workload {
        self.workload
    }

    fn mark_dirty(&mut self, bytes: usize) {
        self.state_dirty = true;
        self.uncheckpointed_bytes += bytes;
    }

    fn transcript_file(&mut self, path: PathBuf) -> Option<TranscriptFile> {
        self.workload.stat_checks += 1;
        let metadata = fs::symlink_metadata(&path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        let modified = metadata.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
        Some(TranscriptFile {
            path,
            size: metadata.len(),
            modified: modified.as_secs_f64(),
        })
    }

    fn record_partial_workload(&mut self) {
        self.workload.retained_partial_files = self.partial_reads.len() as u64;
        self.workload.retained_partial_bytes = self
            .partial_reads
            .values()
            .map(|partial| partial.pending.len() as u64)
            .sum();
        self.workload.largest_retained_partial_bytes = self
            .workload
            .largest_retained_partial_bytes
            .max(self.workload.retained_partial_bytes);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/claude/reader/tests.rs"]
mod tests;
