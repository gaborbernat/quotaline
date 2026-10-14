use super::*;

#[test]
fn bridge_checks_request_origin() {
    let source = HttpOrigin {
        scheme: Some("HTTPS".into()),
        host: Some("Example.COM".into()),
        port: None,
    };
    assert!(http_same_origin(
        source.clone(),
        HttpOrigin {
            scheme: Some("https".into()),
            host: Some("example.com".into()),
            port: None,
        }
    ));
    assert!(!http_same_origin(
        source,
        HttpOrigin {
            scheme: Some("http".into()),
            host: Some("example.com".into()),
            port: None,
        }
    ));
}

#[test]
fn bridge_redacts_request_identifiers() {
    assert_eq!(
        http_redacted_endpoint("https://h/p/ABCDEF12-3456-7890-abcd-ef1234567890".into()),
        "https://h/p/{id}"
    );
}

#[test]
fn bridge_formats_each_http_error() {
    for (error, expected) in [
        (
            HttpErrorRecord::Response {
                status: 302,
                body: String::new(),
                retry_after: None,
            },
            "Unexpected redirect (HTTP 302)",
        ),
        (
            HttpErrorRecord::VendorSecurityCheck { status: 403 },
            "Blocked by provider security check (HTTP 403)",
        ),
        (
            HttpErrorRecord::Network {
                detail: "offline".into(),
            },
            "Network error: offline",
        ),
        (
            HttpErrorRecord::Decoding { detail: "bad".into() },
            "Unexpected response: bad",
        ),
    ] {
        assert_eq!(http_error_message(error, None), expected);
    }
}

#[test]
fn bridge_classifies_provider_errors() {
    for (error, expected) in [
        (
            HttpErrorRecord::Response {
                status: 401,
                body: String::new(),
                retry_after: None,
            },
            ProviderFailure {
                kind: ProviderFailureKind::NotAuthenticated,
                message: "HTTP 401. Sign in".into(),
                retry_after: None,
            },
        ),
        (
            HttpErrorRecord::Response {
                status: 429,
                body: String::new(),
                retry_after: Some(42.0),
            },
            ProviderFailure {
                kind: ProviderFailureKind::RateLimited,
                message: "HTTP 429".into(),
                retry_after: Some(42.0),
            },
        ),
        (
            HttpErrorRecord::Network {
                detail: "offline".into(),
            },
            ProviderFailure {
                kind: ProviderFailureKind::NetworkUnavailable,
                message: "offline".into(),
                retry_after: None,
            },
        ),
        (
            HttpErrorRecord::Decoding { detail: "bad".into() },
            ProviderFailure {
                kind: ProviderFailureKind::Failed,
                message: "Unexpected response: bad".into(),
                retry_after: None,
            },
        ),
    ] {
        assert_eq!(http_provider_failure(error, "Sign in".into(), None), expected);
    }
}

#[test]
fn bridge_exposes_http_response_policy() {
    assert_eq!(http_response_failure(200, None, None, b"ok".to_vec(), 0.0), None);
    assert_eq!(
        http_response_failure(
            429,
            None,
            Some("invalid".into()),
            br#"{"retry_after":30}"#.to_vec(),
            0.0
        ),
        Some(quotaline_http::ResponseFailure {
            status: 429,
            body: r#"{"retry_after":30}"#.into(),
            retry_after: Some(30.0),
            security_check: false,
            kind: ProviderFailureKind::RateLimited,
        })
    );
    assert_eq!(
        http_response_failure(403, Some("text/html".into()), None, b"<html>".to_vec(), 0.0),
        Some(quotaline_http::ResponseFailure {
            status: 403,
            body: "<html>".into(),
            retry_after: None,
            security_check: true,
            kind: ProviderFailureKind::Failed,
        })
    );
    assert!(http_authentication_failure(
        403,
        br#"{"error":"UNAUTHENTICATED"}"#.to_vec()
    ));
}
