use super::*;

#[test]
fn formats_present_fields_and_omits_absent_fields() {
    assert_eq!(
        message("request.finished", &[("id", Some("42")), ("status", None)], None),
        "request.finished id=42"
    );
}

#[test]
fn quotes_and_escapes_field_values() {
    assert_eq!(
        message("panel.open", &[("detail", Some("one=two \\\"three\\\""))], None),
        "panel.open detail=\"one=two \\\\\\\"three\\\\\\\"\""
    );
}

#[test]
fn quotes_values_containing_only_a_quote() {
    assert_eq!(
        message("panel.open", &[("detail", Some("\"quoted\""))], None),
        "panel.open detail=\"\\\"quoted\\\"\""
    );
}

#[test]
fn redacts_sensitive_values_and_home_directory() {
    assert_eq!(
        message(
            "request.finished",
            &[("detail", Some("Bearer secret /home/test/private"))],
            Some("/home/test")
        ),
        "request.finished detail=\"Bearer <redacted> ~/private\""
    );
}
