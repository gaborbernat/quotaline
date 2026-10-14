use serde_json::{Value, json};

use super::*;

#[test]
fn number_reads_finite_numbers_and_numeric_strings() {
    for (value, expected) in [
        (json!(1.5), Some(1.5)),
        (json!("2.5"), Some(2.5)),
        (json!("inf"), None),
        (json!("NaN"), None),
        (json!("many"), None),
        (json!(true), None),
        (Value::Null, None),
    ] {
        assert_eq!(number(&value), expected, "{value}");
    }
}

#[test]
fn text_skips_empty_and_non_strings() {
    for (value, expected) in [(json!("value"), Some("value")), (json!(""), None), (json!(1), None)] {
        assert_eq!(text(&value), expected, "{value}");
    }
}

#[test]
fn timestamp_reads_epochs_and_iso_dates() {
    for (value, expected) in [
        (json!(12), Some(12.0)),
        (json!("1970-01-01T00:00:01.500Z"), Some(1.5)),
        (json!("yesterday"), None),
        (json!(""), None),
    ] {
        assert_eq!(timestamp(&value), expected, "{value}");
    }
}
