use super::*;
use serde_json::json;

#[test]
fn http_errors_format_status_and_sanitize_details() {
    for (name, error, expected) in [
        (
            "redirect",
            HttpError::Response {
                status: 302,
                body: String::new(),
                retry_after: None,
            },
            "Unexpected redirect (HTTP 302)",
        ),
        (
            "server",
            HttpError::Response {
                status: 500,
                body: "secret".into(),
                retry_after: None,
            },
            "HTTP 500",
        ),
        (
            "vendor check",
            HttpError::VendorSecurityCheck(403),
            "Blocked by provider security check (HTTP 403)",
        ),
        (
            "network",
            HttpError::Network("at /home/test".into()),
            "Network error: at ~",
        ),
        (
            "decoding",
            HttpError::Decoding("token@example.org".into()),
            "Unexpected response: <redacted-email>",
        ),
    ] {
        assert_eq!(error.message(Some("/home/test")), expected, "{name}");
    }
}

#[test]
fn http_errors_classify_provider_failures() {
    for (name, error, kind, message, retry_after) in [
        (
            "network",
            HttpError::Network("offline".into()),
            ProviderFailureKind::NetworkUnavailable,
            "offline",
            None,
        ),
        (
            "sign in",
            HttpError::Response {
                status: 401,
                body: String::new(),
                retry_after: None,
            },
            ProviderFailureKind::NotAuthenticated,
            "HTTP 401. Sign in",
            None,
        ),
        (
            "rate limit",
            HttpError::Response {
                status: 429,
                body: String::new(),
                retry_after: Some(42.0),
            },
            ProviderFailureKind::RateLimited,
            "HTTP 429",
            Some(42.0),
        ),
        (
            "server",
            HttpError::Response {
                status: 500,
                body: String::new(),
                retry_after: None,
            },
            ProviderFailureKind::Failed,
            "HTTP 500",
            None,
        ),
        (
            "decode",
            HttpError::Decoding("bad".into()),
            ProviderFailureKind::Failed,
            "Unexpected response: bad",
            None,
        ),
    ] {
        assert_eq!(
            error.provider_failure("Sign in", None),
            ProviderFailure {
                kind,
                message: message.into(),
                retry_after,
            },
            "{name}"
        );
    }
}

#[test]
fn retry_after_prefers_numeric_header_then_date_then_body() {
    let now = 1_777_702_000.0;
    for (header, body, expected) in [
        (Some("3"), json!({"retry_after":40}), Some(3.0)),
        (Some("invalid"), json!({"retry_after":2.5}), Some(2.5)),
        (Some("-1"), json!({"retry_after":1.2}), Some(1.2)),
        (Some("NaN"), json!({"retry_after":4}), Some(4.0)),
        (Some("invalid"), json!({"retry_after":"4"}), Some(4.0)),
        (Some("invalid"), json!({"retry_after":-4}), Some(0.0)),
        (None, json!({"retry_after":null}), None),
        (None, json!({"retry_after":"invalid"}), None),
    ] {
        assert_eq!(retry_after(header, Some(&body), now), expected, "{header:?} {body}");
    }
    assert_eq!(
        retry_after(Some("Sat, 02 May 2026 06:08:40 GMT"), None, now),
        Some(120.0)
    );
    assert_eq!(retry_after(Some("Sat, 02 May 2026 06:00:00 GMT"), None, now), Some(0.0));
    assert_eq!(retry_after(None, None, now), None);
}

#[test]
fn authentication_failures_require_a_real_sign_in_error() {
    for (status, body, expected) in [
        (401, "", true),
        (403, r#"{"error":{"status":"UNAUTHENTICATED"}}"#, true),
        (403, r#"{"error":{"status":"PERMISSION_DENIED"}}"#, false),
        (403, "<html>challenge</html>", false),
        (403, "", false),
        (500, r#"{"error":"UNAUTHENTICATED"}"#, false),
    ] {
        assert_eq!(
            authentication_failure(status, body.as_bytes()),
            expected,
            "{status} {body}"
        );
    }
}

#[test]
fn response_failures_use_one_classification_policy() {
    for (status, body, security_check, expected) in [
        (401, "", false, ProviderFailureKind::NotAuthenticated),
        (
            403,
            r#"{"error":"UNAUTHENTICATED"}"#,
            false,
            ProviderFailureKind::NotAuthenticated,
        ),
        (403, r#"{"error":"UNAUTHENTICATED"}"#, true, ProviderFailureKind::Failed),
        (
            403,
            r#"{"error":"PERMISSION_DENIED"}"#,
            false,
            ProviderFailureKind::Failed,
        ),
        (429, "", false, ProviderFailureKind::RateLimited),
        (500, "", false, ProviderFailureKind::Failed),
    ] {
        assert_eq!(response_failure_kind(status, body.as_bytes(), security_check), expected);
    }
}

#[test]
fn response_failures_bound_the_body_and_resolve_retry_policy() {
    let large = vec![b'x'; MAX_ERROR_BODY_BYTES + 1];
    for (status, content_type, retry_header, body, expected) in [
        (200, None, None, b"ok".as_slice(), None),
        (299, None, None, b"ok".as_slice(), None),
        (
            429,
            None,
            Some("12"),
            br#"{"retry_after":30}"#.as_slice(),
            Some(ResponseFailure {
                status: 429,
                body: r#"{"retry_after":30}"#.into(),
                retry_after: Some(12.0),
                security_check: false,
                kind: ProviderFailureKind::RateLimited,
            }),
        ),
        (
            429,
            None,
            Some("invalid"),
            br#"{"retry_after":30}"#.as_slice(),
            Some(ResponseFailure {
                status: 429,
                body: r#"{"retry_after":30}"#.into(),
                retry_after: Some(30.0),
                security_check: false,
                kind: ProviderFailureKind::RateLimited,
            }),
        ),
        (
            403,
            Some("text/html"),
            None,
            b"<html>".as_slice(),
            Some(ResponseFailure {
                status: 403,
                body: "<html>".into(),
                retry_after: None,
                security_check: true,
                kind: ProviderFailureKind::Failed,
            }),
        ),
        (
            500,
            None,
            None,
            large.as_slice(),
            Some(ResponseFailure {
                status: 500,
                body: "x".repeat(MAX_ERROR_BODY_BYTES),
                retry_after: None,
                security_check: false,
                kind: ProviderFailureKind::Failed,
            }),
        ),
    ] {
        assert_eq!(
            response_failure(status, content_type, retry_header, body, 0.0),
            expected
        );
    }
}

#[test]
fn vendor_security_checks_identify_html_and_challenge_markers() {
    for (status, content_type, body, expected) in [
        (403, Some("text/html; charset=utf-8"), "", true),
        (403, None, "<!doctype html><title>Just a moment</title>", true),
        (403, None, "<html>", true),
        (403, None, "cf-ray", true),
        (403, None, "cloudflare", true),
        (403, None, "challenge-platform", true),
        (403, None, r#"{"error":"permission denied"}"#, false),
        (401, Some("text/html"), "<html>", false),
    ] {
        assert_eq!(
            vendor_security_check(status, content_type, body.as_bytes()),
            expected,
            "{status} {body}"
        );
    }
}
