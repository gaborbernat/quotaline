use super::*;

#[test]
fn create_formats_utc_and_sanitizes_message() {
    let entry = create(1_700_000_000.123, "warning", "refresh", "Bearer abc\nnext", None, 7);
    assert_eq!(entry.message, "Bearer <redacted>\\nnext");
    assert_eq!(
        entry.line,
        "[2023-11-14 22:13:20.123] [warning] [refresh] Bearer <redacted>\\nnext"
    );
    assert_eq!(entry.sequence_id, 7);
}

#[test]
fn record_preserves_original_line() {
    let entry = record(0.0, "info", "app", "safe".into(), Some("original".into()), 4);
    assert_eq!(entry.line, "original");
}

#[test]
fn record_formats_app_without_category() {
    let entry = record(0.0, "info", "app", "ready".into(), None, 4);
    assert_eq!(entry.line, "[1970-01-01 00:00:00.000] [info] ready");
}

#[test]
fn retained_keeps_the_stored_line_and_takes_the_sequence_id() {
    let line = "[2026-08-29 19:00:00.000] [warning] [network] stored";
    let parsed = crate::parse::parse_lines(&[line.into()], 0.0, 0.0, None).remove(0);
    let entry = retained(parsed, 5);
    assert_eq!(
        (
            entry.level.as_str(),
            entry.category.as_str(),
            entry.message.as_str(),
            entry.line.as_str(),
            entry.sequence_id
        ),
        ("warning", "network", "stored", line, 5)
    );
}

#[test]
fn align_keeps_matching_suffix_ids() {
    let mut retained = vec![
        record(0.0, "info", "app", "old".into(), None, 1),
        record(1.0, "info", "app", "same".into(), None, 2),
    ];
    let live = vec![
        record(0.0, "info", "app", "different".into(), None, 8),
        record(1.0, "info", "app", "same".into(), None, 9),
    ];
    align(&mut retained, &live);
    assert_eq!(
        retained.iter().map(|entry| entry.sequence_id).collect::<Vec<_>>(),
        [1, 9]
    );
}

#[test]
fn align_stops_at_first_mismatch() {
    let mut retained = vec![record(0.0, "info", "app", "old".into(), None, 1)];
    let live = vec![record(0.0, "info", "app", "different".into(), None, 9)];
    align(&mut retained, &live);
    assert_eq!(retained[0].sequence_id, 1);
}
