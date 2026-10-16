//! Checks Cursor summary and period response normalization.

use super::*;
use serde_json::json;

#[test]
fn summary_preserves_the_original_payload_without_a_warning() {
    let payload = json!({"membershipType":"pro"});
    let response = UsageResponse::summary(payload.clone());
    assert_eq!(response.summary, payload);
    assert_eq!(response.period_message, None);
    assert_eq!(response.warning, None);
}

#[test]
fn period_normalizes_usage_and_reports_why_it_was_used() {
    let response = UsageResponse::period(
        &json!({"planUsage":{"used":60,"limit":100},"displayMessage":"60% used"}),
        "Provider returned HTTP 503",
    );
    assert_eq!(response.summary["individualUsage"]["plan"]["used"], 60);
    assert_eq!(response.period_message.as_deref(), Some("60% used"));
    assert_eq!(
        response.warning.as_deref(),
        Some("Dashboard summary unavailable: Provider returned HTTP 503")
    );
}

#[test]
fn period_omits_an_empty_message() {
    for payload in [json!({}), json!({"displayMessage":""})] {
        let response = UsageResponse::period(&payload, "offline");
        assert_eq!(response.period_message, None);
    }
}
