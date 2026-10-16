//! Checks the macOS demo readings through the exported boundary.

use super::*;

const NOW: f64 = 1_789_000_000.0;

fn reading(id: &str, long_text: bool) -> crate::provider_record::ProviderRecord {
    demo_reading(id.into(), NOW, long_text).unwrap().provider
}

#[test]
fn demo_reading_covers_every_provider() {
    for id in ["claude", "codex", "gemini", "antigravity", "cursor", "copilot"] {
        let reading = demo_reading(id.into(), NOW, false).unwrap();
        assert_eq!(reading.provider.id, id);
        assert_eq!((reading.fetched_at, reading.source), (NOW, ReadingSource::Network));
        assert!(!reading.provider.windows.is_empty());
    }
}

#[test]
fn demo_reading_preserves_reference_metadata() {
    let claude = reading("claude", false);
    assert_eq!(claude.tier.as_deref(), Some("default_claude_max_20x"));
    assert!(claude.account_fingerprint.is_some_and(|value| value.len() == 64));
    assert_eq!(claude.local_usage.unwrap().today_tokens, 6_200_000);
    let spend = claude.spend.unwrap();
    assert_eq!(
        (spend.used, spend.limit, spend.balance),
        (Some(12.4), Some(50.0), Some(37.6))
    );
    let cloud = claude.cloud_credits.unwrap();
    assert_eq!((cloud.remaining, cloud.limit), (83.0, 250.0));
    assert!(cloud.expires_at.is_some());
    assert_eq!(claude.notices[0].kind, "promotion");

    let codex = reading("codex", false);
    assert_eq!(codex.credit_balance, Some(42.5));
    let credits = codex.usage_credits.unwrap();
    assert_eq!(
        (
            credits.local_messages.unwrap().low,
            credits.cloud_messages.unwrap().high
        ),
        (120, 130)
    );
    assert_eq!(codex.reset_credits.unwrap().available, 1);
    assert_eq!(codex.reset_credit_expiries.len(), 1);

    assert_eq!(reading("gemini", false).credit_balance, Some(1500.0));
    let antigravity = reading("antigravity", false);
    assert_eq!(antigravity.tier.as_deref(), Some("google_ai_ultra"));
    assert_eq!(antigravity.windows.len(), 4);
    let cursor = reading("cursor", false).spend.unwrap();
    assert_eq!(cursor.limit, Some(50.0));
    assert!(cursor.used.is_some());
    assert!(cursor.reset.is_some());
}

#[test]
fn long_text_fixture_expands_display_data() {
    let standard = reading("claude", false);
    let expanded = reading("claude", true);
    assert!(expanded.identity.len() > 50);
    assert!(expanded.tier.unwrap().len() > 50);
    assert!(expanded.windows[0].id.len() > 50);
    assert!(expanded.windows[2].scope.as_ref().unwrap().len() > 50);
    assert_eq!(expanded.windows[0].used, standard.windows[0].used);
    assert_eq!(expanded.notices.len(), standard.notices.len() + 1);
}

#[test]
fn demo_reading_rejects_invalid_requests() {
    assert_eq!(
        demo_reading("claude".into(), f64::NAN, false),
        Err(DemoError::InvalidTime)
    );
    assert_eq!(
        demo_reading("unknown".into(), NOW, false),
        Err(DemoError::UnknownProvider)
    );
}

#[test]
fn demo_analytics_bridge_keeps_daily_metrics() {
    let claude = demo_analytics_points("claude".into(), NOW, 3, false).unwrap().unwrap();
    assert_eq!(claude.len(), 72);
    assert_eq!(claude[0].metric, "inputTokens");
    assert_eq!(claude[0].series, "fable");
    let codex = demo_analytics_points("codex".into(), NOW, 2, false).unwrap().unwrap();
    assert_eq!(codex.len(), 32);
    assert_eq!(codex[0].metric, "surfaceUsagePercent");
    let expanded = demo_analytics_points("claude".into(), NOW, 1, true).unwrap().unwrap();
    assert!(expanded[0].series.len() > 50);
    assert_eq!(demo_analytics_points("gemini".into(), NOW, 1, false).unwrap(), None);
    assert_eq!(
        demo_analytics_points("codex".into(), NOW, 0, false).unwrap(),
        Some(Vec::new())
    );
    assert_eq!(
        demo_analytics_points("claude".into(), f64::NAN, 1, false),
        Err(DemoError::InvalidTime)
    );
}
