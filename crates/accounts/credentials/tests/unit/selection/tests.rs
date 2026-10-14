use super::*;
use base64::Engine;

fn candidate<'a>(
    access_token: &'a str,
    refresh_token: Option<&'a str>,
    expires_at: Option<f64>,
) -> ClaudeCredentialCandidate<'a> {
    ClaudeCredentialCandidate {
        access_token,
        refresh_token,
        expires_at,
    }
}

fn token(subject: &str, issue: u8) -> String {
    let claims = format!(r#"{{"sub":"{subject}","iat":{issue}}}"#);
    format!(
        "header.{}.signature",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims)
    )
}

#[test]
fn candidate_count_selection() {
    for (name, candidates, expected) in [
        ("empty", vec![], None),
        ("single", vec![candidate("first", None, None)], Some(0)),
    ] {
        assert_eq!(select_claude_credential(&candidates), expected, "{name}");
    }
}

#[test]
fn same_subject_prefers_newer_credential() {
    let first = token("account", 1);
    let second = token("account", 2);
    assert_eq!(
        select_claude_credential(&[candidate(&first, None, Some(1.0)), candidate(&second, None, Some(2.0))]),
        Some(1)
    );
}

#[test]
fn shared_refresh_token_prefers_newer_credential() {
    assert_eq!(
        select_claude_credential(&[
            candidate("first", Some("same"), Some(1.0)),
            candidate("second", Some("same"), Some(2.0))
        ]),
        Some(1)
    );
}

#[test]
fn identical_access_token_prefers_newer_credential() {
    assert_eq!(
        select_claude_credential(&[candidate("same", None, Some(1.0)), candidate("same", None, Some(2.0))]),
        Some(1)
    );
}

#[test]
fn different_account_does_not_replace_first_credential() {
    let first = token("first", 1);
    let second = token("second", 2);
    assert_eq!(
        select_claude_credential(&[
            candidate(&first, Some("a"), Some(1.0)),
            candidate(&second, Some("b"), Some(2.0))
        ]),
        Some(0)
    );
    assert_eq!(
        select_claude_credential(&[
            candidate("first", None, Some(1.0)),
            candidate("second", None, Some(2.0))
        ]),
        Some(0)
    );
}

#[test]
fn known_expiry_must_increase() {
    for expiry in [0.0, 1.0] {
        assert_eq!(
            select_claude_credential(&[
                candidate("same", None, Some(1.0)),
                candidate("same", None, Some(expiry))
            ]),
            Some(0)
        );
    }
}

#[test]
fn unknown_expiry_is_later_than_known_expiry() {
    for (name, first, second, expected) in [
        ("new unknown", Some(1.0), None, Some(1)),
        ("current unknown", None, Some(2.0), Some(0)),
        ("both unknown", None, None, Some(0)),
    ] {
        assert_eq!(
            select_claude_credential(&[candidate("same", None, first), candidate("same", None, second)]),
            expected,
            "{name}"
        );
    }
}

#[test]
fn selection_compares_each_candidate_with_the_current_winner() {
    assert_eq!(
        select_claude_credential(&[
            candidate("same", None, Some(1.0)),
            candidate("same", None, Some(3.0)),
            candidate("same", None, Some(2.0))
        ]),
        Some(1)
    );
}
