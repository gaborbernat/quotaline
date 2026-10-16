//! Exercises bounded diagnostic retention and flush retries.

use super::*;

fn record(id: u64, timestamp: f64) -> Record {
    Record {
        timestamp,
        level: "info".into(),
        category: "app".into(),
        message: format!("message {id}"),
        line: format!("line {id}"),
        sequence_id: id,
    }
}

#[test]
fn loaded_entries_keep_only_the_latest_capacity() {
    let buffer = Buffer::new((0..=CAPACITY as u64).map(|id| record(id, 100.0)).collect(), 100.0, true);
    assert_eq!(buffer.snapshot().len(), CAPACITY);
    assert_eq!(buffer.snapshot()[0].sequence_id, 1);
    assert_eq!(
        buffer.tail(2).iter().map(|entry| entry.sequence_id).collect::<Vec<_>>(),
        [499, 500]
    );
    assert_eq!(buffer.tail(usize::MAX).len(), CAPACITY);
}

#[test]
fn memory_only_entries_prune_after_an_hour_without_pending_writes() {
    let mut buffer = Buffer::new(vec![record(1, 0.0)], 0.0, false);
    buffer.append(record(2, RETENTION_SECONDS + 1.0), RETENTION_SECONDS + 1.0);
    assert_eq!(
        buffer
            .snapshot()
            .iter()
            .map(|entry| entry.sequence_id)
            .collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(buffer.take_flush(), None);
    buffer.append(record(3, RETENTION_SECONDS + 2.0), RETENTION_SECONDS + 2.0);
    assert_eq!(buffer.snapshot().len(), 2);
}

#[test]
fn file_entries_append_once_and_acknowledge_the_flush() {
    let mut buffer = Buffer::new(vec![record(1, 0.0)], 0.0, true);
    buffer.append(record(2, 1.0), 1.0);
    assert_eq!(
        buffer.take_flush(),
        Some(FlushWork {
            rewrite: false,
            lines: vec!["line 2".into()]
        })
    );
    assert_eq!(buffer.take_flush(), None);
    buffer.finish_flush();
    assert_eq!(buffer.take_flush(), None);
}

#[test]
fn failed_append_requeues_before_new_entries() {
    let mut buffer = Buffer::new(vec![], 0.0, true);
    buffer.append(record(1, 1.0), 1.0);
    buffer.take_flush();
    buffer.append(record(2, 2.0), 2.0);
    buffer.retry_flush();
    assert_eq!(
        buffer.take_flush(),
        Some(FlushWork {
            rewrite: false,
            lines: vec!["line 1".into(), "line 2".into()]
        })
    );
}

#[test]
fn pending_overflow_and_failed_rewrite_retain_only_live_entries() {
    let mut buffer = Buffer::new(vec![], 0.0, true);
    for id in 0..=PENDING_CAPACITY as u64 {
        buffer.append(record(id, 1.0), 1.0);
    }
    let work = buffer.take_flush().unwrap();
    assert!(work.rewrite);
    assert_eq!(work.lines.len(), CAPACITY);
    buffer.retry_flush();
    assert_eq!(buffer.take_flush(), Some(work));
}

#[test]
fn failed_append_overflow_changes_the_next_flush_to_a_rewrite() {
    let mut buffer = Buffer::new(vec![], 0.0, true);
    for id in 0..PENDING_CAPACITY as u64 {
        buffer.append(record(id, 1.0), 1.0);
    }
    buffer.take_flush();
    buffer.append(record(PENDING_CAPACITY as u64, 1.0), 1.0);
    buffer.retry_flush();
    let work = buffer.take_flush().unwrap();
    assert!(work.rewrite);
    assert_eq!(work.lines.len(), CAPACITY);
}

#[test]
fn an_expired_entry_requests_a_file_rewrite() {
    let mut buffer = Buffer::new(vec![record(1, 0.0)], 0.0, true);
    buffer.append(record(2, RETENTION_SECONDS + 3_600.0), RETENTION_SECONDS + 3_600.0);
    assert_eq!(
        buffer
            .snapshot()
            .iter()
            .map(|entry| entry.sequence_id)
            .collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(
        buffer.take_flush(),
        Some(FlushWork {
            rewrite: true,
            lines: vec!["line 2".into()]
        })
    );
}

#[test]
fn explicit_rewrite_and_clear_reset_pending_work() {
    let mut buffer = Buffer::new(vec![], 0.0, true);
    buffer.append(record(1, 1.0), 1.0);
    buffer.request_rewrite();
    assert_eq!(
        buffer.take_flush(),
        Some(FlushWork {
            rewrite: true,
            lines: vec!["line 1".into()]
        })
    );
    buffer.clear();
    buffer.retry_flush();
    assert_eq!(buffer.take_flush(), None);
    assert!(buffer.snapshot().is_empty());
    assert!(buffer.tail(0).is_empty());
}

#[test]
fn retry_without_an_in_flight_flush_does_nothing() {
    let mut buffer = Buffer::new(vec![], 0.0, true);
    buffer.retry_flush();
    assert_eq!(buffer.take_flush(), None);
}
