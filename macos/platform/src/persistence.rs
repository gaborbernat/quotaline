//! Owns coalesced snapshot writes and widget reload decisions for macOS.

use std::{path::PathBuf, sync::Mutex};

use quotaline_status_bar::WidgetContent;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PersistenceWorkload {
    pub cache_loads: u64,
    pub cache_submissions: u64,
    pub cache_writes: u64,
    pub coalesced_cache_submissions: u64,
    pub usage_file_writes: u64,
    pub widget_submissions: u64,
    pub widget_writes: u64,
    pub coalesced_widget_submissions: u64,
    pub widget_reloads: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceFailure {
    CacheWrite { detail: String },
    UsageFileWrite { detail: String },
    WidgetWrite { detail: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetDrainResult {
    pub failures: Vec<PersistenceFailure>,
    pub reloads: u64,
}

struct SnapshotSubmission {
    cache: Vec<u8>,
    usage: Option<Vec<u8>>,
}

struct WidgetSubmission {
    data: Vec<u8>,
    content: WidgetContent,
}

#[derive(Default)]
struct State {
    pending_snapshots: Option<SnapshotSubmission>,
    pending_widget: Option<WidgetSubmission>,
    last_cache: Option<Vec<u8>>,
    last_usage: Option<Vec<u8>>,
    last_widget: Option<WidgetSubmission>,
    workload: PersistenceWorkload,
}

pub struct SnapshotWriteLedger {
    cache_path: Option<PathBuf>,
    usage_path: Option<PathBuf>,
    widget_path: Option<PathBuf>,
    state: Mutex<State>,
}

impl SnapshotWriteLedger {
    pub fn new(cache_path: Option<String>, usage_path: Option<String>, widget_path: Option<String>) -> Self {
        Self {
            cache_path: cache_path.map(PathBuf::from),
            usage_path: usage_path.map(PathBuf::from),
            widget_path: widget_path.map(PathBuf::from),
            state: Mutex::new(State::default()),
        }
    }

    pub fn workload(&self) -> PersistenceWorkload {
        self.state.lock().unwrap().workload
    }

    pub fn record_cache_load(&self) {
        self.state.lock().unwrap().workload.cache_loads += 1;
    }

    pub fn submit_snapshots(&self, cache: Vec<u8>, usage: Option<Vec<u8>>) {
        let mut state = self.state.lock().unwrap();
        state.workload.cache_submissions += 1;
        if state.pending_snapshots.is_some() {
            state.workload.coalesced_cache_submissions += 1;
        }
        state.pending_snapshots = Some(SnapshotSubmission { cache, usage });
    }

    pub fn has_pending_snapshots(&self) -> bool {
        self.state.lock().unwrap().pending_snapshots.is_some()
    }

    pub fn submit_widget(&self, data: Vec<u8>, content: WidgetContent) {
        if self.widget_path.is_none() {
            return;
        }
        let mut state = self.state.lock().unwrap();
        state.workload.widget_submissions += 1;
        if state.pending_widget.is_some() {
            state.workload.coalesced_widget_submissions += 1;
        }
        state.pending_widget = Some(WidgetSubmission { data, content });
    }

    pub fn has_pending_widget(&self) -> bool {
        self.state.lock().unwrap().pending_widget.is_some()
    }

    pub fn drain_snapshots(&self) -> Vec<PersistenceFailure> {
        let mut failures = Vec::new();
        loop {
            let submission = {
                let mut state = self.state.lock().unwrap();
                match state.pending_snapshots.take() {
                    Some(submission) => submission,
                    None => return failures,
                }
            };
            let write_cache = self.state.lock().unwrap().last_cache.as_ref() != Some(&submission.cache);
            if write_cache {
                let result = self.cache_path.as_ref().map_or(Ok(()), |path| {
                    quotaline_storage::write_private_file(path, &submission.cache).map(|_| ())
                });
                match result {
                    Ok(()) => {
                        let mut state = self.state.lock().unwrap();
                        state.last_cache = Some(submission.cache);
                        state.workload.cache_writes += 1;
                    }
                    Err(error) => failures.push(PersistenceFailure::CacheWrite {
                        detail: error.to_string(),
                    }),
                }
            }
            if let (Some(path), Some(data)) = (&self.usage_path, submission.usage)
                && self.state.lock().unwrap().last_usage.as_ref() != Some(&data)
            {
                match quotaline_storage::write_private_file(path, &data) {
                    Ok(_) => {
                        let mut state = self.state.lock().unwrap();
                        state.last_usage = Some(data);
                        state.workload.usage_file_writes += 1;
                    }
                    Err(error) => failures.push(PersistenceFailure::UsageFileWrite {
                        detail: error.to_string(),
                    }),
                }
            }
        }
    }

    pub fn drain_widgets(&self) -> WidgetDrainResult {
        let mut result = WidgetDrainResult {
            failures: Vec::new(),
            reloads: 0,
        };
        loop {
            let submission = {
                let mut state = self.state.lock().unwrap();
                match state.pending_widget.take() {
                    Some(submission) => submission,
                    None => return result,
                }
            };
            let state = self.state.lock().unwrap();
            if state
                .last_widget
                .as_ref()
                .is_some_and(|last| last.data == submission.data)
            {
                continue;
            }
            let reload = state
                .last_widget
                .as_ref()
                .is_none_or(|last| last.content != submission.content);
            drop(state);
            match quotaline_storage::write_private_file(self.widget_path.as_ref().unwrap(), &submission.data) {
                Ok(_) => {
                    let mut state = self.state.lock().unwrap();
                    state.last_widget = Some(submission);
                    state.workload.widget_writes += 1;
                    if reload {
                        state.workload.widget_reloads += 1;
                        result.reloads += 1;
                    }
                }
                Err(error) => result.failures.push(PersistenceFailure::WidgetWrite {
                    detail: error.to_string(),
                }),
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/persistence/tests.rs"]
mod tests;
