//! Keeps directory walks and hot file tails within refresh budgets.

use super::{IngestResult, MAX_INDEXED_FILES, Reader, TranscriptFile};
use quotaline_claude::claude_transcripts::TranscriptState;
use std::{
    fs::{self, Metadata},
    sync::Arc,
    time::Instant,
};
use walkdir::{DirEntry, FilterEntry, WalkDir};

type Entries = FilterEntry<walkdir::IntoIter, fn(&DirEntry) -> bool>;

pub(super) struct TreeScan {
    entries: Entries,
    cutoff: f64,
    reference_now: f64,
    pub(super) generation: u64,
    hot_files: Vec<TranscriptFile>,
    current_file: Option<TranscriptFile>,
}

fn visible(entry: &DirEntry) -> bool {
    !entry.file_name().to_string_lossy().starts_with('.')
}

fn modified_at(metadata: &Metadata) -> Option<f64> {
    metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs_f64())
}

fn is_newer(left: &TranscriptFile, right: &TranscriptFile) -> bool {
    left.modified
        .total_cmp(&right.modified)
        .then_with(|| right.path.cmp(&left.path))
        .is_gt()
}

impl Reader {
    pub(super) fn begin_scan(&mut self, now: f64) {
        self.partial_reads.clear();
        self.record_partial_workload();
        self.scan_generation = self.scan_generation.wrapping_add(1);
        self.scan = Some(TreeScan {
            entries: WalkDir::new(&self.root)
                .min_depth(1)
                .into_iter()
                .filter_entry(visible as fn(&DirEntry) -> bool),
            cutoff: self.work_cutoff,
            reference_now: now,
            generation: self.scan_generation,
            hot_files: Vec::new(),
            current_file: None,
        });
        self.workload.scans_started += 1;
    }

    pub(super) fn prepare_indexed_tails(&mut self) {
        if self.pending_tail_index < self.pending_tails.len() {
            return;
        }
        self.pending_tails.clear();
        self.pending_tail_index = 0;
        for (key, indexed) in self.indexed_files.clone() {
            let Some(current) = self.transcript_file(indexed.path.clone()) else {
                continue;
            };
            let partial_cursor = self.partial_reads.get(&key).map(|partial| partial.cursor);
            if current == indexed {
                continue;
            }
            let stored = self.state.offset(&key);
            let cursor = partial_cursor
                .or_else(|| stored.map(|offset| offset.bytes))
                .unwrap_or(0);
            if current.size < cursor || current.size == indexed.size {
                let generation = stored.map_or(self.scan_generation, |offset| offset.scan_generation);
                Arc::make_mut(&mut self.state).set_offset(key.clone(), 0, generation);
                self.partial_reads.remove(&key);
                self.mark_dirty(0);
            }
            self.indexed_files.insert(key, current.clone());
            self.pending_tails.push(current);
        }
    }

    pub(super) fn run_slice(&mut self) {
        let previous_bytes = self.workload.bytes_read;
        let previous_entries = self.workload.tree_entries_examined;
        self.run_slice_inner();
        self.workload.largest_slice_bytes_read = self
            .workload
            .largest_slice_bytes_read
            .max(self.workload.bytes_read - previous_bytes);
        self.workload.largest_slice_entries_examined = self
            .workload
            .largest_slice_entries_examined
            .max(self.workload.tree_entries_examined - previous_entries);
    }

    fn run_slice_inner(&mut self) {
        let deadline = Instant::now() + self.config.time_budget;
        let mut remaining = self.config.byte_budget;
        while self.pending_tail_index < self.pending_tails.len() && remaining > 0 && Instant::now() < deadline {
            let file = self.pending_tails[self.pending_tail_index].clone();
            let result = self.ingest(&file, self.work_cutoff, remaining, deadline, true);
            self.apply_ingest(&result);
            remaining -= result.bytes_read;
            if !result.complete {
                return;
            }
            self.pending_tail_index += 1;
        }
        if self.pending_tail_index < self.pending_tails.len() {
            return;
        }
        self.pending_tails.clear();
        self.pending_tail_index = 0;
        let Some(mut scan) = self.scan.take() else {
            return;
        };
        let mut entries_remaining = self.config.entry_budget;
        while remaining > 0 && entries_remaining > 0 && Instant::now() < deadline {
            if let Some(file) = scan.current_file.clone() {
                let hot = scan.hot_files.iter().any(|candidate| candidate.path == file.path);
                let result = self.ingest(&file, scan.cutoff, remaining, deadline, hot);
                self.apply_ingest(&result);
                remaining -= result.bytes_read;
                if !result.complete {
                    break;
                }
                scan.current_file = None;
                continue;
            }
            let Some(entry) = scan.entries.by_ref().flatten().next() else {
                self.finish_scan(scan);
                return;
            };
            self.workload.tree_entries_examined += 1;
            entries_remaining -= 1;
            let path = entry.into_path();
            if !path.extension().is_some_and(|extension| extension == "jsonl") {
                continue;
            }
            let Some(metadata) = fs::symlink_metadata(&path).ok().filter(|metadata| metadata.is_file()) else {
                continue;
            };
            let Some(modified) = modified_at(&metadata) else {
                continue;
            };
            if modified < scan.cutoff {
                continue;
            }
            self.workload.stat_checks += 1;
            let file = TranscriptFile {
                path,
                size: metadata.len(),
                modified,
            };
            let key = file.path.to_string_lossy().into_owned();
            Arc::make_mut(&mut self.state).touch_offset(&key, scan.generation);
            let hot = self.insert_hot_file(&file, &mut scan.hot_files);
            if self.state.offset(&key).map(|offset| offset.bytes) != Some(file.size) {
                scan.current_file = Some(file.clone());
                let result = self.ingest(&file, scan.cutoff, remaining, deadline, hot);
                self.apply_ingest(&result);
                remaining -= result.bytes_read;
                if !result.complete {
                    break;
                }
                scan.current_file = None;
            }
        }
        self.scan = Some(scan);
    }

    fn finish_scan(&mut self, scan: TreeScan) {
        if Arc::make_mut(&mut self.state).retain_offsets(scan.generation) {
            self.state = Arc::new(TranscriptState::default());
            Arc::make_mut(&mut self.state).configure_retention(self.retention_days);
            self.indexed_files.clear();
            self.partial_reads.clear();
            self.pending_tails.clear();
            self.pending_tail_index = 0;
            self.has_scanned = false;
            self.mark_dirty(0);
            self.workload.scans_completed += 1;
            self.begin_scan(scan.reference_now);
            return;
        }
        self.indexed_files = scan
            .hot_files
            .into_iter()
            .map(|file| (file.path.to_string_lossy().into_owned(), file))
            .collect();
        self.has_scanned = true;
        self.next_file_scan_at = scan.reference_now + self.config.file_scan_interval;
        self.workload.scans_completed += 1;
        self.record_partial_workload();
    }

    fn insert_hot_file(&mut self, file: &TranscriptFile, files: &mut Vec<TranscriptFile>) -> bool {
        if files.len() < MAX_INDEXED_FILES {
            files.push(file.clone());
        } else if files.last().is_some_and(|oldest| is_newer(file, oldest)) {
            let oldest = files.last_mut().expect("full hot-file index");
            self.partial_reads.remove(&oldest.path.to_string_lossy().into_owned());
            *oldest = file.clone();
        } else {
            self.partial_reads.remove(&file.path.to_string_lossy().into_owned());
            return false;
        }
        files.sort_by(|left, right| {
            right
                .modified
                .total_cmp(&left.modified)
                .then_with(|| left.path.cmp(&right.path))
        });
        true
    }

    fn apply_ingest(&mut self, result: &IngestResult) {
        if result.changed {
            self.mark_dirty(result.bytes_read);
        }
    }
}
