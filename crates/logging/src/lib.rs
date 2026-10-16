//! Redacts secrets and bounds diagnostic text for every frontend.

use regex_lite::Regex;
use std::sync::LazyLock;

pub mod buffer;
pub mod diagnostic;
pub mod entry;
pub mod issue;
pub mod parse;

pub const MAX_MESSAGE_BYTES: usize = 1_900;

static REDACTIONS: LazyLock<[(Regex, &str); 6]> = LazyLock::new(|| {
    [
        (Regex::new(r"(?i)\b(?:request|response)?[_-]?body\s*=[^\r\n]*").unwrap(), "body=<redacted>"),
        (
            Regex::new(r"(?i)\b(authorization|proxy[_-]?authorization)\s*[:=]\s*(?:Bearer|Basic)\s+[^\s,;}\]\r\n]+")
                .unwrap(),
            "$1=<redacted>",
        ),
        (Regex::new(r"(?i)\b(Bearer|Basic)\s+[A-Za-z0-9+/_=.:-]+").unwrap(), "$1 <redacted>"),
        (
            Regex::new(
                r#"(?i)(?:\\?["'])?\b(authorization|proxy[_-]?authorization|cookie|set[_-]?cookie|password|secret|client[_-]?secret|api[_-]?key|access[_-]?token|refresh[_-]?token|id[_-]?token)(?:\\?["'])?\s*[:=]\s*(?:\\?["'][^"'\r\n]*\\?["']|[^\s\\,;}\]\r\n]+)"#,
            )
            .unwrap(),
            "$1=<redacted>",
        ),
        (
            Regex::new(r"(?i)([a-z][a-z0-9+.-]*://[^ \t\r\n?]+)\?[^ \t\r\n\\]*").unwrap(),
            "$1?<redacted>",
        ),
        (Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").unwrap(), "<redacted-email>"),
    ]
});

pub fn redact(value: &str, home: Option<&str>) -> String {
    let mut safe = home
        .filter(|home| home.len() > 1)
        .map_or_else(|| value.to_owned(), |home| value.replace(home, "~"));
    for (pattern, replacement) in REDACTIONS.iter() {
        safe = pattern.replace_all(&safe, *replacement).into_owned();
    }
    safe
}

pub fn message(value: &str, home: Option<&str>) -> String {
    let mut safe = value.replace('\r', "\\r").replace('\n', "\\n");
    safe.retain(|character| !character.is_control());
    truncate(&redact(&safe, home), MAX_MESSAGE_BYTES)
}

pub fn truncate(value: &str, maximum_bytes: usize) -> String {
    if value.len() <= maximum_bytes {
        return value.into();
    }
    if maximum_bytes < '…'.len_utf8() {
        return String::new();
    }
    format!(
        "{}…",
        &value[..value.floor_char_boundary(maximum_bytes - '…'.len_utf8())]
    )
}

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
