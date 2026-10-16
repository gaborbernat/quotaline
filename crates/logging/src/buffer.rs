//! Keeps diagnostic entries and pending writes within fixed memory and age limits.

use std::collections::VecDeque;

pub const CAPACITY: usize = 500;
pub const PENDING_CAPACITY: usize = CAPACITY * 2;
pub const RETENTION_SECONDS: f64 = 7.0 * 86_400.0;
pub const FILE_BYTE_LIMIT: usize = 1_048_576;
pub const FILE_COUNT: usize = 3;
pub const RETAINED_ENTRY_LIMIT: usize = 100_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub timestamp: f64,
    pub level: String,
    pub category: String,
    pub message: String,
    pub line: String,
    pub sequence_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlushWork {
    pub rewrite: bool,
    pub lines: Vec<String>,
}

enum PendingWork {
    Append(VecDeque<Record>),
    Rewrite,
}

pub struct Buffer {
    entries: VecDeque<Record>,
    unwritten: VecDeque<Record>,
    in_flight: Option<PendingWork>,
    last_prune: Option<f64>,
    rewrite_on_flush: bool,
    stores_file: bool,
}

impl Buffer {
    pub fn new(initial: Vec<Record>, now: f64, stores_file: bool) -> Self {
        Self {
            entries: initial
                .into_iter()
                .rev()
                .take(CAPACITY)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
            unwritten: VecDeque::new(),
            in_flight: None,
            last_prune: stores_file.then_some(now),
            rewrite_on_flush: false,
            stores_file,
        }
    }

    pub fn append(&mut self, entry: Record, now: f64) {
        self.entries.push_back(entry.clone());
        if self.stores_file {
            self.unwritten.push_back(entry);
            if self.unwritten.len() > PENDING_CAPACITY {
                self.unwritten.pop_front();
                self.rewrite_on_flush = true;
            }
        }
        if self.entries.len() > CAPACITY {
            self.entries.pop_front();
        }
        if self.last_prune.is_none_or(|last| now - last >= 3_600.0) {
            let before = self.entries.len();
            self.entries.retain(|entry| entry.timestamp >= now - RETENTION_SECONDS);
            self.last_prune = Some(now);
            self.rewrite_on_flush |= self.stores_file && before != self.entries.len();
        }
    }

    pub fn snapshot(&self) -> Vec<Record> {
        self.entries.iter().cloned().collect()
    }

    pub fn tail(&self, count: usize) -> Vec<Record> {
        self.entries
            .iter()
            .skip(self.entries.len().saturating_sub(count))
            .cloned()
            .collect()
    }

    pub fn take_flush(&mut self) -> Option<FlushWork> {
        if self.in_flight.is_some() {
            return None;
        }
        if self.rewrite_on_flush {
            self.rewrite_on_flush = false;
            self.unwritten.clear();
            self.in_flight = Some(PendingWork::Rewrite);
            return Some(FlushWork {
                rewrite: true,
                lines: self.entries.iter().map(|entry| entry.line.clone()).collect(),
            });
        }
        if self.unwritten.is_empty() {
            return None;
        }
        let pending = std::mem::take(&mut self.unwritten);
        let lines = pending.iter().map(|entry| entry.line.clone()).collect();
        self.in_flight = Some(PendingWork::Append(pending));
        Some(FlushWork { rewrite: false, lines })
    }

    pub fn finish_flush(&mut self) {
        self.in_flight = None;
    }

    pub fn retry_flush(&mut self) {
        match self.in_flight.take() {
            Some(PendingWork::Append(pending)) => {
                self.unwritten = pending.into_iter().chain(std::mem::take(&mut self.unwritten)).collect();
                if self.unwritten.len() > PENDING_CAPACITY {
                    self.unwritten.drain(..self.unwritten.len() - PENDING_CAPACITY);
                    self.rewrite_on_flush = true;
                }
            }
            Some(PendingWork::Rewrite) => self.rewrite_on_flush = true,
            None => {}
        }
    }

    pub fn request_rewrite(&mut self) {
        self.rewrite_on_flush = true;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.unwritten.clear();
        self.in_flight = None;
        self.rewrite_on_flush = false;
    }
}

#[cfg(test)]
#[path = "../tests/unit/buffer/tests.rs"]
mod tests;
