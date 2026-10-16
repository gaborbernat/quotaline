use super::*;

#[test]
fn parses_levels_and_categories_without_rewriting_stored_lines() {
    for (level, category) in [
        ("debug", "app"),
        ("info", "geometry"),
        ("warning", "network"),
        ("error", "persistence"),
        ("info", "refresh"),
        ("warning", "status"),
        ("error", "tabs"),
    ] {
        let line = format!("[2026-08-29 19:00:00.000] [{level}] [{category}] retained");
        let parsed = parse_lines(std::slice::from_ref(&line), 0.0, 0.0, None);
        assert_eq!(
            (parsed[0].level, parsed[0].category, parsed[0].message.as_str()),
            (level, category, "retained")
        );
        assert_eq!(parsed[0].original_line.as_deref(), Some(line.as_str()));
    }
}

#[test]
fn parses_utc_milliseconds_and_leap_days() {
    let line = "[2024-02-29 12:34:56.789] [info] retained";
    let parsed = parse_lines(&[line.into()], 0.0, 0.0, None);
    assert_eq!(parsed[0].timestamp, 1_709_210_096.789);
    assert_eq!(parsed[0].message, "retained");
}

#[test]
fn malformed_dates_use_the_previous_timestamp_and_keep_the_whole_line() {
    for stamp in [
        "202x-08-29 12:34:56.789",
        "0000-08-29 12:34:56.789",
        "2025-02-29 12:34:56.789",
        "2026-04-31 12:34:56.789",
        "2026-13-29 12:34:56.789",
    ] {
        let line = format!("[{stamp}] [info] retained");
        let parsed = parse_lines(std::slice::from_ref(&line), 42.0, 0.0, None);
        assert_eq!(parsed[0].timestamp, 42.0, "{stamp}");
        assert_eq!(parsed[0].message, line, "{stamp}");
        assert_eq!(parsed[0].original_line, None, "{stamp}");
    }
}

#[test]
fn unrecognized_levels_retain_the_text_after_a_valid_timestamp() {
    for line in [
        "[2026-08-29 19:00:00.000] no level here",
        "[2026-08-29 19:00:00.000] [fatal] unknown level",
        "[2026-08-29 19:00:00.000] [info without close",
    ] {
        let parsed = parse_lines(&[line.into()], 0.0, 0.0, None);
        assert_eq!(parsed[0].level, "info");
        assert_eq!(parsed[0].original_line, None);
    }
}

#[test]
fn invalid_category_remains_part_of_the_message() {
    let line = "[2026-08-29 19:00:00.000] [warning] [unknown] detail";
    let parsed = parse_lines(&[line.into()], 0.0, 0.0, None);
    assert_eq!(parsed[0].category, "app");
    assert_eq!(parsed[0].message, "[unknown] detail");
    assert_eq!(parsed[0].original_line.as_deref(), Some(line));
}

#[test]
fn malformed_prefixes_are_safe_and_get_a_rendered_line() {
    for line in [
        "",
        "é malformed",
        "2026-08-29 19:00:00.000] [info] bad",
        "[2026-08-29 19:00:00.000x [info] bad",
    ] {
        let parsed = parse_lines(&[line.into()], 42.0, 0.0, None);
        assert_eq!(parsed[0].message, line);
        assert_eq!(parsed[0].original_line, None);
    }
}

#[test]
fn secret_redaction_forces_a_fresh_render() {
    let line = "[2026-08-29 19:00:00.000] [warning] [network] access_token=private";
    let parsed = parse_lines(&[line.into()], 0.0, 0.0, None);
    assert_eq!(parsed[0].message, "access_token=<redacted>");
    assert_eq!(parsed[0].original_line, None);
}

#[test]
fn expired_entries_are_dropped_after_updating_the_fallback_timestamp() {
    let lines = [
        "[2026-08-29 19:00:00.000] [info] old".into(),
        "old continuation".into(),
        "[2026-08-29 19:00:02.000] [info] recent".into(),
        "recent continuation".into(),
    ];
    let parsed = parse_lines(&lines, 0.0, 1_788_030_001.0, None);
    assert_eq!(
        parsed.iter().map(|line| line.message.as_str()).collect::<Vec<_>>(),
        ["recent", "recent continuation"]
    );
}
