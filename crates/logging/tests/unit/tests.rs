use super::*;

#[test]
fn redacts_structured_secrets_urls_emails_and_home() {
    let report = "Bearer abc access_token=xyz user@example.com /home/test/private https://example.com/path?key=value";
    let safe = redact(report, Some("/home/test"));
    assert_eq!(
        safe,
        "Bearer <redacted> access_token=<redacted> <redacted-email> ~/private https://example.com/path?<redacted>"
    );
}

#[test]
fn redacts_multiline_reports_without_removing_other_lines() {
    assert_eq!(
        redact("first body=private\nsecond access_token: secret\nthird visible", None),
        "first body=<redacted>\nsecond access_token=<redacted>\nthird visible"
    );
}

#[test]
fn ignores_a_root_home_path_and_redacts_escaped_json_fields() {
    assert_eq!(
        redact(r#"{\"client_secret\":\"private\"}"#, Some("/")),
        "{client_secret=<redacted>}"
    );
}

#[test]
fn message_escapes_newlines_and_removes_controls() {
    assert_eq!(message("visible\nnext\tline", None), "visible\\nnextline");
}

#[test]
fn truncation_preserves_utf8_boundaries_and_byte_limit() {
    assert_eq!(truncate("ééé", 5), "é…");
    assert_eq!(truncate("text", 4), "text");
    assert_eq!(truncate("text", 2), "");
    assert_eq!(message(&"é".repeat(2_000), None).len(), 1_899);
}
