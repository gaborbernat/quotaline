//! macOS template token bridge cases.

use super::*;

#[test]
fn parsed_template_exposes_text_placeholders_and_newlines() {
    assert_eq!(
        parse_status_template("A{{B}}{pct}\n{unknown}".into()),
        [
            StatusTemplateToken {
                kind: StatusTemplateTokenKind::Text,
                value: "A{B}".into(),
            },
            StatusTemplateToken {
                kind: StatusTemplateTokenKind::Placeholder,
                value: "pct".into(),
            },
            StatusTemplateToken {
                kind: StatusTemplateTokenKind::Newline,
                value: String::new(),
            },
            StatusTemplateToken {
                kind: StatusTemplateTokenKind::Placeholder,
                value: "unknown".into(),
            },
        ]
    );
}

#[test]
fn empty_template_has_no_tokens() {
    assert!(parse_status_template(String::new()).is_empty());
}
