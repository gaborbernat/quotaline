//! Reads bounded JSONL chunks without dropping incomplete records.

use super::{Record, parse};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartialLine {
    pub committed_offset: u64,
    pub pending: Vec<u8>,
    pub oversized: bool,
}

pub fn consume_chunk(
    mut partial: PartialLine,
    source: &str,
    chunk: &[u8],
    chunk_start: u64,
    maximum_line_bytes: usize,
    cutoff: f64,
) -> (PartialLine, Vec<Record>) {
    let mut records = Vec::new();
    let mut start = 0;
    while start < chunk.len() {
        let Some(relative_newline) = memchr::memchr(b'\n', &chunk[start..]) else {
            if !partial.oversized {
                let suffix = &chunk[start..];
                if partial.pending.len() + suffix.len() <= maximum_line_bytes {
                    partial.pending.extend_from_slice(suffix);
                } else {
                    partial.pending.clear();
                    partial.oversized = true;
                }
            }
            break;
        };
        let newline = start + relative_newline;
        if partial.oversized {
            partial.oversized = false;
        } else if partial.pending.len() + newline - start <= maximum_line_bytes {
            partial.pending.extend_from_slice(&chunk[start..newline]);
            if memchr::memmem::find(&partial.pending, b"\"usage\"").is_some()
                && let Some(mut record) = parse(&partial.pending)
                && record.timestamp as f64 >= cutoff
            {
                if record.id == ":" {
                    record.id = format!("{source}:{}", partial.committed_offset);
                }
                records.push(record);
            }
        }
        partial.pending.clear();
        partial.committed_offset = chunk_start + newline as u64 + 1;
        start = newline + 1;
    }
    (partial, records)
}

#[cfg(test)]
#[path = "../../tests/unit/claude_transcripts/lines/tests.rs"]
mod tests;
