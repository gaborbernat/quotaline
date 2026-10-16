//! Legacy provider payloads retain required defaults.

use super::*;

#[test]
fn legacy_window_without_active_field_remains_active() {
    let window: Window = serde_json::from_value(serde_json::json!({
        "id":"session", "label":"Session", "used":20, "reset":null, "group":"session"
    }))
    .unwrap();
    assert!(window.active);
}

#[test]
fn legacy_provider_detail_keeps_explanatory_hover_text() {
    let detail: Detail = serde_json::from_value(serde_json::json!({
        "title":"Subscription", "value":"Active"
    }))
    .unwrap();
    assert_eq!(detail.explanation, "Provider-reported detail.");
}

#[test]
fn legacy_reset_credit_detail_keeps_explanatory_text() {
    let detail: ResetCreditDetail = serde_json::from_value(serde_json::json!({
        "title":"Extra reset", "status":null, "credit_type":null, "description":null,
        "granted_at":null, "expires_at":null
    }))
    .unwrap();
    assert!(detail.explanation.contains("never reserves or consumes"));
}
