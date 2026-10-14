//! Normalizes provider identifiers and labels for display, comparison, and stable keys.

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub fn humanize(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn slug(value: &str) -> String {
    let mut result = String::new();
    for character in value.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
        } else if !result.is_empty() && !result.ends_with('-') {
            result.push('-');
        }
    }
    result.trim_end_matches('-').to_string()
}

/// Prints an amount with up to six decimals and no trailing zeros.
pub fn decimal(value: f64) -> String {
    format!("{value:.6}").trim_end_matches('0').trim_end_matches('.').into()
}

/// Folds case, accents, and spacing so two labels a reader would call the same compare equal.
pub fn normalized_label(label: &str) -> String {
    label
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
#[path = "../tests/unit/text/tests.rs"]
mod tests;
