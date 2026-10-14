//! Adapts the shared diagnostic queue to Swift values.

use quotaline_logging::buffer;
use quotaline_logging::buffer::Buffer;
use quotaline_logging::buffer::Record;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct LogQueueRecord {
    pub timestamp: f64,
    pub level: String,
    pub category: String,
    pub message: String,
    pub line: String,
    pub sequence_id: u64,
}

impl From<LogQueueRecord> for Record {
    fn from(record: LogQueueRecord) -> Self {
        Self {
            timestamp: record.timestamp,
            level: record.level,
            category: record.category,
            message: record.message,
            line: record.line,
            sequence_id: record.sequence_id,
        }
    }
}

impl From<Record> for LogQueueRecord {
    fn from(record: Record) -> Self {
        Self {
            timestamp: record.timestamp,
            level: record.level,
            category: record.category,
            message: record.message,
            line: record.line,
            sequence_id: record.sequence_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct LogQueueDefaults {
    pub capacity: u32,
    pub pending_capacity: u32,
    pub retention_seconds: f64,
    pub file_byte_limit: u64,
    pub file_count: u32,
    pub retained_entry_limit: u32,
}

#[uniffi::export]
pub fn log_queue_defaults() -> LogQueueDefaults {
    LogQueueDefaults {
        capacity: buffer::CAPACITY as u32,
        pending_capacity: buffer::PENDING_CAPACITY as u32,
        retention_seconds: buffer::RETENTION_SECONDS,
        file_byte_limit: buffer::FILE_BYTE_LIMIT as u64,
        file_count: buffer::FILE_COUNT as u32,
        retained_entry_limit: buffer::RETAINED_ENTRY_LIMIT as u32,
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct LogQueueWork {
    pub rewrite: bool,
    pub lines: Vec<String>,
}

#[derive(uniffi::Object)]
pub struct RustLogBuffer {
    state: Mutex<Buffer>,
}

#[uniffi::export]
impl RustLogBuffer {
    #[uniffi::constructor]
    pub fn new(initial: Vec<LogQueueRecord>, now: f64, stores_file: bool) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Buffer::new(
                initial.into_iter().map(Into::into).collect(),
                now,
                stores_file,
            )),
        })
    }

    pub fn append(&self, entry: LogQueueRecord, now: f64) {
        self.state().append(entry.into(), now);
    }

    pub fn snapshot(&self) -> Vec<LogQueueRecord> {
        self.state().snapshot().into_iter().map(Into::into).collect()
    }

    pub fn tail(&self, count: u32) -> Vec<LogQueueRecord> {
        self.state().tail(count as usize).into_iter().map(Into::into).collect()
    }

    pub fn take_flush(&self) -> Option<LogQueueWork> {
        self.state().take_flush().map(|work| LogQueueWork {
            rewrite: work.rewrite,
            lines: work.lines,
        })
    }

    pub fn finish_flush(&self) {
        self.state().finish_flush();
    }

    pub fn retry_flush(&self) {
        self.state().retry_flush();
    }

    pub fn request_rewrite(&self) {
        self.state().request_rewrite();
    }

    pub fn clear(&self) {
        self.state().clear();
    }
}

impl RustLogBuffer {
    fn state(&self) -> std::sync::MutexGuard<'_, Buffer> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/logging/buffer/tests.rs"]
mod tests;
