//! Checks that a shared provider reading reaches Swift without losing fields.

use super::*;
use quotaline_quota::{Notice, NoticeKind, Provider, ResetPrecision, Window};

fn demo(id: &str) -> Provider {
    quotaline_demo::demo().remove(id).unwrap()
}

#[test]
fn windows_keep_their_reset_precision_and_grouping() {
    let mut window = Window::new("weekly", "Weekly", 40.0, Some("2026-10-20".into()), "weekly");
    window.reset_precision = ResetPrecision::Day;
    let mut provider = Provider::connected("copilot", "Pro".into(), "fixture".into(), "me".into(), vec![window]);
    provider.tier = Some("Business".into());
    let record = ProviderRecord::from(provider);
    assert_eq!(record.tier.as_deref(), Some("Business"));
    assert_eq!(
        (record.windows[0].group.as_str(), record.windows[0].reset_by_day),
        ("weekly", true)
    );
}

#[test]
fn notices_carry_their_wire_kind() {
    let mut provider = Provider::connected("copilot", "Pro".into(), "fixture".into(), "me".into(), vec![]);
    provider.notices = vec![Notice::new(NoticeKind::LimitReached, "Limit reached")];
    assert_eq!(ProviderRecord::from(provider).notices[0].kind, "limitReached");
}

#[test]
fn claude_spend_keeps_minor_unit_money() {
    let mut provider = demo("claude");
    provider.spend.as_mut().unwrap().used_money = Some(quotaline_quota::MoneyAmount {
        amount_minor: 125,
        currency: "CAD".into(),
        exponent: 2,
    });
    let record = ProviderRecord::from(provider);
    assert_eq!(record.spend.unwrap().used_money.unwrap().amount_minor, 125);
    assert!(record.cloud_credits.is_some());
    assert!(record.local_usage.is_some());
}

#[test]
fn codex_credits_keep_message_ranges_and_reset_credits() {
    let record = ProviderRecord::from(demo("codex"));
    let credits = record.usage_credits.unwrap();
    let messages = credits.local_messages.unwrap();
    assert!(messages.low <= messages.high);
    assert!(!record.reset_credits.unwrap().details.is_empty());
}
