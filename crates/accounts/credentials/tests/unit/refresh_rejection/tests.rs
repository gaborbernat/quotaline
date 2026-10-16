use serde_json::json;

use super::*;

#[test]
fn terminal_codes_require_sign_in_on_client_errors() {
    for code in [
        "invalid_grant",
        "invalid_client",
        "unauthorized_client",
        "refresh_token_expired",
        "refresh_token_reused",
        "refresh_token_invalidated",
    ] {
        assert_eq!(terminal_refresh_error_code(400, &json!({"error": code})), Some(code));
    }
    assert_eq!(
        terminal_refresh_error_code(499, &json!({"error": "invalid_grant"})),
        Some("invalid_grant")
    );
}

#[test]
fn terminal_codes_ignore_other_responses() {
    for (status, body) in [
        (399, json!({"error":"invalid_grant"})),
        (500, json!({"error":"invalid_grant"})),
        (400, json!({"error":"other"})),
        (400, json!({"error":1})),
        (400, json!({})),
    ] {
        assert_eq!(terminal_refresh_error_code(status, &body), None);
    }
}

#[test]
fn rejection_is_scoped_to_provider_and_credential() {
    let mut ledger = RefreshRejectionLedger::default();
    let old = json!({"token":"old"});
    let renewed = json!({"token":"new"});
    assert_eq!(
        ledger.record_failure("claude", &old, 400, &json!({"error":"invalid_grant"})),
        Some("invalid_grant".into())
    );
    assert_eq!(ledger.rejected_code("claude", &old), Some("invalid_grant"));
    assert_eq!(ledger.rejected_code("codex", &old), None);
    assert_eq!(ledger.rejected_code("claude", &renewed), None);
    assert_eq!(ledger.rejected_code("claude", &old), None);
}

#[test]
fn nonterminal_failure_keeps_the_previous_rejection() {
    let mut ledger = RefreshRejectionLedger::default();
    let credential = json!({"token":"old"});
    ledger.record_failure("claude", &credential, 400, &json!({"error":"invalid_grant"}));
    assert_eq!(
        ledger.record_failure("claude", &credential, 500, &json!({"error":"invalid_grant"})),
        None
    );
    assert_eq!(ledger.rejected_code("claude", &credential), Some("invalid_grant"));
}

#[test]
fn successful_refresh_clears_the_rejection() {
    let mut ledger = RefreshRejectionLedger::default();
    let credential = json!({"token":"old"});
    ledger.record_failure("claude", &credential, 400, &json!({"error":"invalid_grant"}));
    ledger.clear("claude");
    assert_eq!(ledger.rejected_code("claude", &credential), None);
}

#[test]
fn refresh_failures_name_the_provider_and_the_rejection() {
    assert_eq!(
        refresh_failed_message("Codex", &refresh_rejected_message("invalid_grant")),
        "Codex token refresh failed: the provider rejected the stored sign-in (invalid_grant). Sign in with the CLI again."
    );
}
