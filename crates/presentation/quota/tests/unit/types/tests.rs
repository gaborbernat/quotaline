//! Checks public wire values shared with platform adapters.

use super::*;

#[test]
fn notice_kinds_use_the_shared_wire_names() {
    assert_eq!(
        [
            NoticeKind::Promotion,
            NoticeKind::LimitReached,
            NoticeKind::SpendControl,
            NoticeKind::Info
        ]
        .map(|kind| kind.as_str()),
        ["promotion", "limitReached", "spendControl", "info"]
    );
}

#[test]
fn provider_fingerprint_prefers_explicit_account_id() {
    let mut provider = Provider::connected("codex", String::new(), String::new(), "user@example.com".into(), vec![]);
    provider.account_fingerprint = Some("account-42".into());
    assert_eq!(provider.account_fingerprint().as_deref(), Some("account-42"));
}

#[test]
fn provider_fingerprint_hashes_email_when_account_id_is_missing() {
    let provider = Provider::connected("codex", String::new(), String::new(), "USER@example.com".into(), vec![]);
    assert_eq!(
        provider.account_fingerprint().as_deref(),
        Some("398dac8df0ca8c21b6c559ac611983faa10d668ae2a0b024e39545928918c01a")
    );
}

#[test]
fn provider_fingerprint_skips_anonymous_identity() {
    let provider = Provider::connected("codex", String::new(), String::new(), "workspace".into(), vec![]);
    assert_eq!(provider.account_fingerprint(), None);
}

fn credits(has_credits: Option<bool>, unlimited: bool, balance: Option<f64>) -> Provider {
    let mut provider = Provider::empty("codex", "connected");
    provider.credit_balance = balance;
    provider.usage_credits = Some(UsageCredits {
        unlimited,
        has_credits,
        overage_limit_reached: false,
        local_messages: None,
        cloud_messages: None,
    });
    provider
}

#[test]
fn provider_credits_follow_the_reported_flag_first() {
    assert_eq!(credits(Some(false), true, Some(5.0)).has_credits(), Some(false));
}

#[test]
fn provider_credits_count_unlimited_plans_as_available() {
    assert_eq!(credits(None, true, Some(0.0)).has_credits(), Some(true));
}

#[test]
fn provider_credits_fall_back_to_the_balance() {
    assert_eq!(credits(None, false, Some(0.0)).has_credits(), Some(false));
    let mut bare = Provider::empty("codex", "connected");
    bare.credit_balance = Some(3.0);
    assert_eq!(bare.has_credits(), Some(true));
    assert_eq!(Provider::empty("codex", "connected").has_credits(), None);
}
