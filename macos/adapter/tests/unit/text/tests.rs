//! macOS text bridge cases.

use super::*;

#[test]
fn identifier_text_uses_core_policy() {
    assert_eq!(humanize_identifier("seven_day-oauth".into()), "Seven Day Oauth");
    assert_eq!(slug_identifier("GPT-5.3-Codex Spark!".into()), "gpt-5-3-codex-spark");
}
