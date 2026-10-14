//! Checks UniFFI values around the shared diagnostic queue.

use super::*;

fn record(id: u64) -> LogQueueRecord {
    LogQueueRecord {
        timestamp: id as f64,
        level: "warning".into(),
        category: "refresh".into(),
        message: format!("message {id}"),
        line: format!("line {id}"),
        sequence_id: id,
    }
}

#[test]
fn defaults_expose_the_shared_memory_limits() {
    assert_eq!(log_queue_defaults().capacity, buffer::CAPACITY as u32);
    assert_eq!(log_queue_defaults().pending_capacity, buffer::PENDING_CAPACITY as u32);
    assert_eq!(log_queue_defaults().retention_seconds, buffer::RETENTION_SECONDS);
    assert_eq!(log_queue_defaults().file_byte_limit, buffer::FILE_BYTE_LIMIT as u64);
    assert_eq!(log_queue_defaults().file_count, buffer::FILE_COUNT as u32);
    assert_eq!(
        log_queue_defaults().retained_entry_limit,
        buffer::RETAINED_ENTRY_LIMIT as u32
    );
}

#[test]
fn queue_records_round_trip_through_the_bridge() {
    let entry = record(1);
    let queue = RustLogBuffer::new(vec![entry.clone()], 1.0, false);
    assert_eq!(queue.snapshot(), vec![entry.clone()]);
    assert_eq!(queue.tail(1), [entry]);
    queue.append(record(2), 2.0);
    assert_eq!(queue.tail(1), [record(2)]);
    queue.clear();
    assert!(queue.snapshot().is_empty());
}

#[test]
fn queue_flush_and_retry_keep_the_original_lines() {
    let queue = RustLogBuffer::new(vec![], 0.0, true);
    queue.append(record(1), 1.0);
    assert_eq!(
        queue.take_flush(),
        Some(LogQueueWork {
            rewrite: false,
            lines: vec!["line 1".into()]
        })
    );
    queue.retry_flush();
    assert_eq!(
        queue.take_flush(),
        Some(LogQueueWork {
            rewrite: false,
            lines: vec!["line 1".into()]
        })
    );
    queue.finish_flush();
    assert_eq!(queue.take_flush(), None);
}

#[test]
fn queue_can_request_a_rewrite() {
    let queue = RustLogBuffer::new(vec![record(1)], 1.0, true);
    queue.request_rewrite();
    assert_eq!(
        queue.take_flush(),
        Some(LogQueueWork {
            rewrite: true,
            lines: vec!["line 1".into()]
        })
    );
}
