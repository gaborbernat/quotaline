//! Checks split records, bounded tails, malformed lines, and committed offsets.

use super::*;

fn record() -> Vec<u8> {
    br#"{"type":"assistant","timestamp":"2026-10-09T12:00:00Z","message":{"id":"one","model":"claude-sonnet-4-6","usage":{"input_tokens":2}}}"#.to_vec()
}

#[test]
fn complete_record_advances_offset_and_returns_parsed_usage() {
    let mut bytes = record();
    bytes.push(b'\n');
    let (partial, records) = consume_chunk(PartialLine::default(), "session.jsonl", &bytes, 100, 1024, 0.0);
    assert_eq!(partial.committed_offset, 100 + bytes.len() as u64);
    assert!(partial.pending.is_empty());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, "one:");
}

#[test]
fn split_record_waits_for_newline_and_keeps_the_committed_offset() {
    let bytes = record();
    let middle = bytes.len() / 2;
    let (partial, records) = consume_chunk(PartialLine::default(), "session.jsonl", &bytes[..middle], 0, 1024, 0.0);
    assert_eq!(partial.pending, bytes[..middle]);
    assert_eq!(partial.committed_offset, 0);
    assert!(records.is_empty());
    let mut tail = bytes[middle..].to_vec();
    tail.push(b'\n');
    let (partial, records) = consume_chunk(partial, "session.jsonl", &tail, middle as u64, 1024, 0.0);
    assert_eq!(partial.committed_offset, (bytes.len() + 1) as u64);
    assert_eq!(records.len(), 1);
}

#[test]
fn oversized_line_skips_to_the_next_newline() {
    let (partial, records) = consume_chunk(PartialLine::default(), "session.jsonl", b"too long", 0, 3, 0.0);
    assert!(partial.oversized);
    assert!(partial.pending.is_empty());
    assert!(records.is_empty());
    let mut next = b" tail\n".to_vec();
    next.extend_from_slice(&record());
    next.push(b'\n');
    let (partial, records) = consume_chunk(partial, "session.jsonl", &next, 8, 1024, 0.0);
    assert_eq!(records.len(), 1);
    assert_eq!(partial.committed_offset, 8 + next.len() as u64);
    assert!(!partial.oversized);
}

#[test]
fn oversized_tail_stays_discarded_until_a_newline_arrives() {
    let (partial, _) = consume_chunk(PartialLine::default(), "session.jsonl", b"too long", 0, 3, 0.0);
    let (partial, records) = consume_chunk(partial, "session.jsonl", b"more bytes", 8, 3, 0.0);
    assert!(partial.oversized);
    assert!(partial.pending.is_empty());
    assert_eq!(partial.committed_offset, 0);
    assert!(records.is_empty());
}

#[test]
fn complete_oversized_line_does_not_block_the_next_record() {
    let mut bytes = b"ignored-too-long\n".to_vec();
    bytes.extend_from_slice(&record());
    bytes.push(b'\n');
    let (partial, records) = consume_chunk(PartialLine::default(), "session.jsonl", &bytes, 0, 150, 0.0);
    assert_eq!(records.len(), 1);
    assert_eq!(partial.committed_offset, bytes.len() as u64);
    let (partial, records) = consume_chunk(
        partial,
        "session.jsonl",
        b"another-oversized-line\n",
        bytes.len() as u64,
        3,
        0.0,
    );
    assert!(records.is_empty());
    assert_eq!(
        partial.committed_offset,
        (bytes.len() + b"another-oversized-line\n".len()) as u64
    );
}

#[test]
fn irrelevant_invalid_and_old_lines_do_not_produce_records() {
    let mut bytes = b"not json\n{\"usage\":true}\n".to_vec();
    bytes.extend_from_slice(&record());
    bytes.push(b'\n');
    let (partial, records) = consume_chunk(
        PartialLine::default(),
        "session.jsonl",
        &bytes,
        0,
        1024,
        1_799_000_000.0,
    );
    assert!(records.is_empty());
    assert_eq!(partial.committed_offset, bytes.len() as u64);
}

#[test]
fn marker_free_lines_advance_without_json_parsing() {
    let (partial, records) = consume_chunk(PartialLine::default(), "session.jsonl", b"plain text\n", 0, 1024, 0.0);
    assert!(records.is_empty());
    assert_eq!(partial.committed_offset, 11);
}

#[test]
fn message_without_provider_id_uses_file_and_line_offset() {
    let bytes = br#"{"type":"assistant","timestamp":"2026-10-09T12:00:00Z","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":2}}}
{"type":"assistant","timestamp":"2026-10-09T12:01:00Z","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":2}}}
"#;
    let (_, records) = consume_chunk(PartialLine::default(), "session.jsonl", bytes, 0, 1024, 0.0);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id, "session.jsonl:0");
    assert_eq!(
        records[1].id,
        format!(
            "session.jsonl:{}",
            bytes.iter().position(|byte| *byte == b'\n').unwrap() + 1
        )
    );
}
