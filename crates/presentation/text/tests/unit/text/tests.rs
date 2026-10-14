//! Identifier normalization cases.

use super::*;

#[test]
fn humanize_separates_provider_keys() {
    for (value, expected) in [
        ("seven_day_oauth-apps", "Seven Day Oauth Apps"),
        ("already Human", "Already Human"),
        ("__", ""),
        ("éclair-mode", "Éclair Mode"),
    ] {
        assert_eq!(humanize(value), expected, "{value}");
    }
}

#[test]
fn decimal_trims_trailing_zeros() {
    for (value, expected) in [(12.0, "12"), (1.5, "1.5"), (0.123_456_7, "0.123457"), (0.0, "0")] {
        assert_eq!(decimal(value), expected, "{value}");
    }
}

#[test]
fn slug_keeps_ascii_words() {
    for (value, expected) in [
        ("GPT-5.3-Codex Spark!", "gpt-5-3-codex-spark"),
        ("--Repeated---separators--", "repeated-separators"),
        ("éclair", "clair"),
        ("***", ""),
    ] {
        assert_eq!(slug(value), expected, "{value}");
    }
}

#[test]
fn normalized_labels_ignore_case_accents_and_spacing() {
    assert_eq!(normalized_label("  Café   Pro "), "cafe pro");
    assert_eq!(normalized_label("CAFÉ PRO"), normalized_label("cafe pro"));
}
