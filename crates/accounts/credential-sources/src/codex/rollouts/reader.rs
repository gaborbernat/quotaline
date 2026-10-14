//! Keeps bounded Codex rollout discovery and reverse reads between refreshes.

use quotaline_codex::codex_usage;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use walkdir::{IntoIter, WalkDir};

mod search;

pub const MAX_FILES: usize = 8;
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
pub const DEFAULT_CACHE_SECONDS: u64 = 15 * 60;
pub const DEFAULT_BUDGET: Budget = Budget {
    bytes: 2 * 1024 * 1024,
    entries: 256,
    time: Duration::from_millis(50),
    cache: DEFAULT_CACHE_SECONDS as f64,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub data: Value,
    pub observed_at: Option<f64>,
    pub reference_now: f64,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Default)]
pub struct Policy {
    pub maximum_tail_bytes: Option<u64>,
    pub require_window: bool,
    pub refresh_empty_cache_when_throttled: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Workload {
    pub trees_scanned: u64,
    pub tree_entries_examined: u64,
    pub stat_checks: u64,
    pub files_opened: u64,
    pub bytes_read: u64,
    pub largest_slice_bytes_read: u64,
    pub largest_tree_slice_entries: u64,
    pub searches_completed: u64,
}

#[derive(Clone, Copy)]
pub struct Budget {
    pub bytes: usize,
    pub entries: usize,
    pub time: Duration,
    pub cache: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RolloutFile {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

enum CacheState {
    Candidates,
    Searched(Option<Reading>),
}

struct Cache {
    files: Vec<RolloutFile>,
    expires_at: f64,
    state: CacheState,
}

struct TreeScan {
    entries: IntoIter,
    candidates: Vec<RolloutFile>,
    reference_now: f64,
}

struct Search {
    files: Vec<RolloutFile>,
    reference_now: f64,
    expires_at: f64,
    file_index: usize,
    current: Option<ReverseRead>,
}

struct ReverseRead {
    cursor: u64,
    floor: u64,
    reversed_line: Vec<u8>,
    oversized: bool,
}

pub struct RolloutReader {
    root: PathBuf,
    budget: Budget,
    policy: Policy,
    cache: Option<Cache>,
    tree_scan: Option<TreeScan>,
    search: Option<Search>,
    workload: Workload,
}

impl RolloutReader {
    pub fn new(root: PathBuf, budget: Budget) -> Self {
        Self::with_policy(root, budget, Policy::default())
    }

    pub fn with_policy(root: PathBuf, budget: Budget, policy: Policy) -> Self {
        Self {
            root,
            budget: Budget {
                bytes: budget.bytes.max(1),
                entries: budget.entries.max(1),
                ..budget
            },
            policy,
            cache: None,
            tree_scan: None,
            search: None,
            workload: Workload::default(),
        }
    }

    pub fn latest(&mut self, now: f64, throttled: bool) -> Option<Reading> {
        let cache_available = self.cache.as_ref().is_some_and(|cache| {
            cache.expires_at > now
                || throttled
                    && !(self.policy.refresh_empty_cache_when_throttled
                        && matches!(cache.state, CacheState::Searched(None)))
        });
        let cached = self
            .cache
            .as_ref()
            .filter(|cache| cache_available && matches!(cache.state, CacheState::Searched(_)))
            .map(|cache| (cache.files.clone(), cache.expires_at));
        if let Some((old_files, expires_at)) = cached {
            match self.refreshed_files(&old_files) {
                Some(files) if files == old_files => return self.reading(),
                Some(files) => {
                    self.cache = Some(Cache {
                        files: files.clone(),
                        expires_at,
                        state: CacheState::Candidates,
                    });
                    self.search = Some(Search::new(files, now, expires_at));
                }
                None => {
                    self.cache = None;
                    self.tree_scan = None;
                    self.search = None;
                }
            }
        }
        if self.tree_scan.is_none() && self.search.is_none() {
            if let Some(cache) = self.cache.as_ref().filter(|_| cache_available) {
                self.search = Some(Search::new(cache.files.clone(), now, cache.expires_at));
            } else {
                self.start_scan(now);
            }
        }
        self.run_slice();
        self.reading()
    }

    pub fn latest_with_cache_age(&mut self, now: f64, throttled: bool, cache_age: Duration) -> Option<Reading> {
        let age = cache_age.as_secs_f64();
        let change = age - self.budget.cache;
        self.budget.cache = age;
        if let Some(cache) = &mut self.cache {
            cache.expires_at += change;
        }
        if let Some(search) = &mut self.search {
            search.expires_at += change;
        }
        self.latest(now, throttled)
    }

    pub fn start_scan(&mut self, now: f64) {
        self.cache = None;
        self.search = None;
        self.workload.trees_scanned += 1;
        self.tree_scan = Some(TreeScan {
            entries: WalkDir::new(&self.root).min_depth(1).into_iter(),
            candidates: Vec::new(),
            reference_now: now,
        });
    }

    pub fn run_slice(&mut self) {
        let started = Instant::now();
        let bytes = self.workload.bytes_read;
        let entries = self.workload.tree_entries_examined;
        self.advance_tree(started);
        if started.elapsed() < self.budget.time {
            self.advance_search(started);
        }
        self.workload.largest_slice_bytes_read = self
            .workload
            .largest_slice_bytes_read
            .max(self.workload.bytes_read - bytes);
        self.workload.largest_tree_slice_entries = self
            .workload
            .largest_tree_slice_entries
            .max(self.workload.tree_entries_examined - entries);
    }

    pub fn cancel(&mut self) {
        self.tree_scan = None;
        self.search = None;
    }

    pub fn pending(&self) -> bool {
        self.tree_scan.is_some() || self.search.is_some()
    }

    pub fn tree_pending(&self) -> bool {
        self.tree_scan.is_some()
    }

    pub fn candidate_paths(&self) -> Vec<PathBuf> {
        self.cache
            .as_ref()
            .map(|cache| cache.files.iter().map(|file| file.path.clone()).collect())
            .unwrap_or_default()
    }

    pub fn workload(&self) -> Workload {
        self.workload
    }

    pub fn reading(&self) -> Option<Reading> {
        match &self.cache.as_ref()?.state {
            CacheState::Candidates => None,
            CacheState::Searched(reading) => reading.clone(),
        }
    }

    fn refreshed_files(&mut self, cached: &[RolloutFile]) -> Option<Vec<RolloutFile>> {
        let mut files = Vec::new();
        for file in cached {
            self.workload.stat_checks += 1;
            files.push(metadata(&file.path)?);
        }
        files.sort_by(newer_first);
        Some(files)
    }

    fn advance_tree(&mut self, started: Instant) {
        let Some(scan) = &mut self.tree_scan else { return };
        let mut examined = 0;
        while examined < self.budget.entries && started.elapsed() < self.budget.time {
            let Some(entry) = scan.entries.next() else {
                let files = std::mem::take(&mut scan.candidates);
                let reference_now = scan.reference_now;
                let expires_at = reference_now + self.budget.cache;
                self.cache = Some(Cache {
                    files: files.clone(),
                    expires_at,
                    state: CacheState::Candidates,
                });
                self.tree_scan = None;
                self.search = Some(Search::new(files, reference_now, expires_at));
                return;
            };
            let Ok(entry) = entry else { continue };
            if entry.file_name().to_string_lossy().starts_with('.') {
                if entry.file_type().is_dir() {
                    scan.entries.skip_current_dir();
                }
                continue;
            }
            examined += 1;
            self.workload.tree_entries_examined += 1;
            if !entry.file_type().is_file()
                || !entry.file_name().to_string_lossy().starts_with("rollout-")
                || entry.path().extension().is_none_or(|extension| extension != "jsonl")
            {
                continue;
            }
            self.workload.stat_checks += 1;
            if let Some(file) = metadata(entry.path()) {
                insert_candidate(&mut scan.candidates, file);
            }
        }
    }
}

impl Search {
    fn new(files: Vec<RolloutFile>, reference_now: f64, expires_at: f64) -> Self {
        Self {
            files,
            reference_now,
            expires_at,
            file_index: 0,
            current: None,
        }
    }
}

fn metadata(path: &Path) -> Option<RolloutFile> {
    let attributes = fs::symlink_metadata(path).ok()?;
    attributes.is_file().then(|| RolloutFile {
        path: path.to_path_buf(),
        size: attributes.len(),
        modified: attributes.modified().unwrap_or(SystemTime::UNIX_EPOCH),
    })
}

fn newer_first(left: &RolloutFile, right: &RolloutFile) -> std::cmp::Ordering {
    right
        .modified
        .cmp(&left.modified)
        .then_with(|| left.path.cmp(&right.path))
}

fn insert_candidate(files: &mut Vec<RolloutFile>, file: RolloutFile) {
    if files.len() < MAX_FILES {
        files.push(file);
        files.sort_by(newer_first);
    } else if newer_first(&file, &files[MAX_FILES - 1]).is_lt() {
        files[MAX_FILES - 1] = file;
        files.sort_by(newer_first);
    }
}

fn parse_line(reversed: &[u8], reference_now: f64, path: &Path, policy: Policy) -> Option<Reading> {
    if reversed.is_empty() {
        return None;
    }
    let line = reversed.iter().rev().copied().collect::<Vec<_>>();
    memchr::memmem::find(&line, b"\"rate_limits\"")?;
    let document: Value = serde_json::from_slice(&line).ok()?;
    let rollout = codex_usage::rollout(&document)?;
    if policy.require_window
        && rollout.data["rate_limit"]["primary_window"].is_null()
        && rollout.data["rate_limit"]["secondary_window"].is_null()
    {
        return None;
    }
    Some(Reading {
        data: rollout.data,
        observed_at: rollout.observed_at,
        reference_now,
        path: path.to_path_buf(),
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/codex/rollouts/reader/tests.rs"]
mod tests;
