//! Checks that a record Swift held comes back as the same shared provider value.

use super::*;
use crate::provider_record::{ProviderMessageRange, ProviderNoticeRecord, ProviderUsageCreditsRecord, ReadingSource};

fn money(amount_minor: i64) -> Option<quota::MoneyAmount> {
    Some(quota::MoneyAmount {
        amount_minor,
        currency: "USD".into(),
        exponent: 2,
    })
}

fn record() -> ProviderRecord {
    ProviderRecord {
        id: "codex".into(),
        plan: "Pro".into(),
        tier: Some("pro".into()),
        identity: "you@example.com".into(),
        organization: "Team".into(),
        account_fingerprint: Some("fingerprint".into()),
        subscription_active_until: Some("2026-09-21T14:13:20.000Z".into()),
        windows: vec![ProviderWindowRecord {
            id: "weekly".into(),
            label: "Weekly".into(),
            used: 40.0,
            active: false,
            reset: Some("2026-05-28T21:26:40.000Z".into()),
            reset_by_day: true,
            group: "weekly".into(),
            scope: Some("all".into()),
            severity: Some("warning".into()),
            detail: "All models".into(),
            duration_seconds: Some(604_800),
        }],
        local_usage: Some(quota::LocalUsage {
            window_tokens: 10,
            window_cost: 0.5,
            cost_per_hour: 0.1,
            today_tokens: 20,
            today_cost: 1.5,
            today_messages: 3,
        }),
        spend: Some(quota::SpendControl {
            enabled: Some(true),
            can_toggle: true,
            disabled_reason: Some("Disabled by the team".into()),
            disabled_reason_code: Some("team_disabled".into()),
            used: Some(12.5),
            limit: Some(50.0),
            used_money: money(1250),
            limit_money: money(5000),
            percent: Some(25.0),
            currency: Some("USD".into()),
            reset: Some("2026-05-29T20:26:40.000Z".into()),
            limit_reached: false,
            balance: Some(3.0),
            balance_money: money(300),
            auto_reload: Some(false),
            can_purchase_credits: true,
            credit_allowance: Some(quota::CreditAllowance {
                used: Some(1.0),
                limit: Some(10.0),
                remaining: Some(9.0),
                source: Some("plan".into()),
            }),
        }),
        details: vec![quota::Detail {
            id: "plan".into(),
            title: "Plan".into(),
            value: "Pro".into(),
            explanation: "Provider-reported detail.".into(),
        }],
        credit_balance: Some(9.5),
        usage_credits: Some(ProviderUsageCreditsRecord {
            unlimited: false,
            has_credits: Some(true),
            overage_limit_reached: true,
            local_messages: Some(ProviderMessageRange { low: 1, high: 2 }),
            cloud_messages: Some(ProviderMessageRange { low: 3, high: 4 }),
        }),
        reset_credits: Some(quota::ResetCredits {
            available: 2,
            applicable: 1,
            total_earned: Some(3),
            purchase_eligible: true,
            details: vec![quota::ResetCreditDetail {
                id: "credit".into(),
                title: "Reset credit".into(),
                status: Some("Active".into()),
                credit_type: Some("bonus".into()),
                description: Some("Earned".into()),
                granted_at: Some("2026-05-01T00:00:00.000Z".into()),
                expires_at: Some("2026-06-01T00:00:00.000Z".into()),
                explanation: "Provider-reported reset credit.".into(),
            }],
        }),
        cloud_credits: Some(quota::CloudCredits {
            limit: 100.0,
            remaining: 83.0,
            expires_at: Some("2026-06-09T10:13:20.000Z".into()),
            locked_reason: Some("trial".into()),
        }),
        reset_credit_expiries: vec![quota::CreditExpiry {
            id: "credit".into(),
            at: 1_780_500_000,
        }],
        notices: vec![ProviderNoticeRecord {
            kind: "limitReached".into(),
            text: "Limit reached.".into(),
            window_id: Some("weekly".into()),
            resets_at: Some("2026-05-28T21:26:40.000Z".into()),
        }],
        warnings: vec!["Showing cached usage.".into()],
    }
}

#[test]
fn record_round_trips_through_the_shared_provider() {
    let record = record();
    assert_eq!(ProviderRecord::from(quota::Provider::from(record.clone())), record);
}

#[test]
fn reading_stamps_the_provider_update_time() {
    let provider = quota::Provider::from(ProviderReadingRecord {
        provider: record(),
        fetched_at: 1_780_000_000.5,
        source: ReadingSource::LocalLog,
    });
    assert_eq!(provider.updated, Some(1_780_000_000));
}

#[test]
fn unknown_notice_kinds_read_as_information() {
    let mut record = record();
    record.notices[0].kind = "newKind".into();
    assert_eq!(quota::Provider::from(record).notices[0].kind, quota::NoticeKind::Info);
}

#[test]
fn window_reset_reads_as_unix_seconds() {
    assert_eq!(record().windows[0].reset_seconds(), Some(1_780_003_600.0));
    assert_eq!(
        ProviderWindowRecord {
            reset: None,
            ..record().windows[0].clone()
        }
        .reset_seconds(),
        None
    );
}
