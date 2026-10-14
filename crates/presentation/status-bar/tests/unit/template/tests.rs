//! Panel template parsing cases.

use super::*;

#[test]
fn template_parser_keeps_literals_placeholders_and_line_breaks() {
    assert_eq!(
        parse_template("a{{b}}c\\nd\ne{pct}"),
        [
            TemplateToken::Text("a{b}c".into()),
            TemplateToken::Newline,
            TemplateToken::Text("d".into()),
            TemplateToken::Newline,
            TemplateToken::Text("e".into()),
            TemplateToken::Placeholder("pct".into()),
        ]
    );
}

#[test]
fn template_parser_preserves_unclosed_and_escaped_text() {
    for (template, expected) in [
        ("{unclosed", "{unclosed"),
        ("x{", "x{"),
        ("{}", "{}"),
        ("}x", "}x"),
        ("a}", "a}"),
        ("\\t\\", "t"),
        ("{a\\", "{a\\"),
    ] {
        assert_eq!(
            parse_template(template),
            [TemplateToken::Text(expected.into())],
            "{template}"
        );
    }
    assert!(parse_template("").is_empty());
    assert!(parse_template("\\").is_empty());
}

#[test]
fn template_parser_keeps_empty_lines() {
    assert_eq!(parse_template("\n\n"), [TemplateToken::Newline, TemplateToken::Newline]);
}
