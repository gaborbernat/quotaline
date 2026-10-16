//! Reads the loosely typed numbers, strings, and timestamps that provider responses report.

use serde_json::Value;

use crate::time::parse_iso_epoch;

/// A finite number, whether the provider sent a JSON number or a numeric string.
pub fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        .filter(|value| value.is_finite())
}

pub fn text(value: &Value) -> Option<&str> {
    value.as_str().filter(|value| !value.is_empty())
}

/// Epoch seconds from either a number or an RFC 3339 string.
pub fn timestamp(value: &Value) -> Option<f64> {
    number(value).or_else(|| text(value).and_then(parse_iso_epoch))
}

#[cfg(test)]
#[path = "../tests/unit/json/tests.rs"]
mod tests;
