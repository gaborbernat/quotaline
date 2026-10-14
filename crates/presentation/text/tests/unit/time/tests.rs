use super::*;

#[test]
fn parses_offset_and_fractional_iso_dates() {
    assert_eq!(parse_iso_epoch("1970-01-01T00:00:00Z"), Some(0.0));
    assert_eq!(parse_iso_epoch("1970-01-01T01:00:00.500000+01:00"), Some(0.5));
    assert_eq!(
        parse_iso_epoch("2026-09-01T14:59:59.522121+00:00"),
        Some(1_788_274_799.522)
    );
    assert_eq!(parse_iso_epoch("invalid"), None);
}

#[test]
fn formats_millisecond_utc_dates() {
    assert_eq!(format_iso_epoch(0.0).as_deref(), Some("1970-01-01T00:00:00.000Z"));
    assert_eq!(format_iso_epoch(0.5).as_deref(), Some("1970-01-01T00:00:00.500Z"));
    assert_eq!(format_iso_epoch(f64::NAN), None);
    assert_eq!(format_iso_epoch(f64::MAX), None);
}
