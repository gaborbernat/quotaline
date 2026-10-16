use super::*;

#[test]
fn origin_policy_accepts_only_matching_parts() {
    let source = Origin {
        scheme: Some("HTTPS"),
        host: Some("Example.COM"),
        port: Some(443),
    };
    for (case, destination, expected) in [
        (
            "same origin",
            Origin {
                scheme: Some("https"),
                host: Some("example.com"),
                port: Some(443),
            },
            true,
        ),
        (
            "scheme",
            Origin {
                scheme: Some("http"),
                ..source
            },
            false,
        ),
        (
            "host",
            Origin {
                host: Some("other.test"),
                ..source
            },
            false,
        ),
        ("port", Origin { port: None, ..source }, false),
        ("missing scheme", Origin { scheme: None, ..source }, false),
        ("missing host", Origin { host: None, ..source }, false),
    ] {
        assert_eq!(same_origin(source, destination), expected, "{case}");
    }
    assert!(same_origin(
        Origin {
            scheme: None,
            host: None,
            port: None
        },
        Origin {
            scheme: None,
            host: None,
            port: None
        }
    ));
}

#[test]
fn endpoint_policy_masks_only_hex_identifier_segments() {
    for (case, input, expected) in [
        (
            "identifier",
            "https://h/p/ABCDEF12-3456-7890-abcd-ef1234567890/x",
            "https://h/p/{id}/x",
        ),
        ("short", "https://h/not-an-identifier", "https://h/not-an-identifier"),
        (
            "non-hex",
            "https://h/zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz",
            "https://h/zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz",
        ),
        ("plain", "https://h/path", "https://h/path"),
    ] {
        assert_eq!(redacted_endpoint(input), expected, "{case}");
    }
}
