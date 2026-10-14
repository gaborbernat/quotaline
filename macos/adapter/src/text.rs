//! Exposes shared identifier text formatting to Swift.

use quotaline_text::text;

#[uniffi::export]
pub fn humanize_identifier(value: String) -> String {
    text::humanize(&value)
}

#[uniffi::export]
pub fn slug_identifier(value: String) -> String {
    text::slug(&value)
}

#[cfg(test)]
#[path = "../tests/unit/text/tests.rs"]
mod tests;
