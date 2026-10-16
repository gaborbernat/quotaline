//! Binds diagnostic records and file storage to Swift.

mod buffer;
mod issue;
pub use buffer::{LogQueueDefaults, LogQueueRecord, LogQueueWork, RustLogBuffer, log_queue_defaults};
pub use issue::{diagnostic_issue_url, diagnostic_issue_url_limit, repository_url};
use quotaline_logging::buffer::Record;
use quotaline_logging::entry;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct DiagnosticIdentity {
    pub email: Option<String>,
    pub organization: Option<String>,
}

#[uniffi::export]
pub fn redact_diagnostic_report(
    value: String,
    home: String,
    identities: Vec<DiagnosticIdentity>,
    hide_personal_information: bool,
) -> String {
    let mut report = value;
    if hide_personal_information {
        for identity in identities {
            for replacement in quotaline_quota::usage::identity_presentation(
                identity.email.as_deref(),
                identity.organization.as_deref(),
                true,
            )
            .replacements
            {
                report = report.replace(&replacement.value, &replacement.replacement);
            }
        }
    }
    quotaline_logging::redact(&report, Some(&home))
}

static NEXT_SEQUENCE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ParsedLogBatch {
    pub entries: Vec<LogQueueRecord>,
    pub dropped_entries: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct DiagnosticField {
    pub key: String,
    pub value: Option<String>,
}

#[uniffi::export]
pub fn diagnostic_event_message(name: String, fields: Vec<DiagnosticField>, home: String) -> String {
    let fields = fields
        .iter()
        .map(|field| (field.key.as_str(), field.value.as_deref()))
        .collect::<Vec<_>>();
    quotaline_logging::diagnostic::message(&name, &fields, Some(&home))
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LogStorageError {
    #[error("Log storage failed: {message}")]
    Io { message: String },
}

impl From<std::io::Error> for LogStorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}

#[uniffi::export]
pub fn log_load(
    path: String,
    count: u32,
    maximum_file_bytes: u64,
    maximum_entries: u64,
    cutoff: f64,
    home: String,
    live: Vec<LogQueueRecord>,
) -> ParsedLogBatch {
    let loaded = quotaline_storage::logs::load(
        std::path::Path::new(&path),
        count as usize,
        maximum_file_bytes as usize,
        maximum_entries as usize,
        cutoff,
        Some(&home),
    );
    let mut entries: Vec<Record> = loaded
        .entries
        .into_iter()
        .map(|line| entry::retained(line, next_sequence_id()))
        .collect();
    entry::align(&mut entries, &live.into_iter().map(Into::into).collect::<Vec<_>>());
    ParsedLogBatch {
        entries: entries.into_iter().map(Into::into).collect(),
        dropped_entries: loaded.dropped_entries,
    }
}

#[uniffi::export]
pub fn log_make_entry(
    timestamp: f64,
    level: String,
    category: String,
    message: String,
    home: String,
) -> LogQueueRecord {
    entry::create(timestamp, &level, &category, &message, Some(&home), next_sequence_id()).into()
}

fn next_sequence_id() -> u64 {
    NEXT_SEQUENCE_ID.fetch_add(1, Ordering::Relaxed) + 1
}

#[uniffi::export]
pub fn log_append(
    path: String,
    lines: Vec<String>,
    maximum_file_bytes: u64,
    count: u32,
) -> Result<(), LogStorageError> {
    quotaline_storage::logs::append(
        std::path::Path::new(&path),
        &lines,
        maximum_file_bytes as usize,
        count as usize,
    )?;
    Ok(())
}

#[uniffi::export]
pub fn log_rewrite(
    path: String,
    lines: Vec<String>,
    maximum_file_bytes: u64,
    count: u32,
) -> Result<(), LogStorageError> {
    quotaline_storage::logs::rewrite(
        std::path::Path::new(&path),
        &lines,
        maximum_file_bytes as usize,
        count as usize,
    )?;
    Ok(())
}

#[uniffi::export]
pub fn log_clear(path: String, count: u32) -> Result<(), LogStorageError> {
    quotaline_storage::logs::clear(std::path::Path::new(&path), count as usize)?;
    Ok(())
}

#[uniffi::export]
pub fn log_redact(value: String, home: String) -> String {
    quotaline_logging::redact(&value, Some(&home))
}

#[uniffi::export]
pub fn log_message(value: String, home: String) -> String {
    quotaline_logging::message(&value, Some(&home))
}

#[cfg(test)]
#[path = "../tests/unit/logging/tests.rs"]
mod tests;
