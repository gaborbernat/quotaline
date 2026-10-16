//! Checks the public behavior of the adjacent module.

use super::*;
use serde_json::json;

const MAY_FIRST: f64 = 1_777_636_800.0;
const MAY_SECOND: f64 = MAY_FIRST + 86_400.0;

fn encoded(value: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn successful_endpoints_survive_relaunch_with_one_day_overlap() {
    let mut watermarks = Watermarks::load(None, "account".into(), MAY_FIRST, 7);
    assert!(watermarks.encoded_if_changed().is_none());
    let starts = watermarks.plan("2026-04-25", "2026-05-01");
    assert_eq!(starts.len(), 5);
    assert!(starts.iter().all(|(_, day)| day == "2026-04-25"));
    watermarks.complete(&[Endpoint::TokenUsage, Endpoint::Skills], "2026-05-01", MAY_FIRST, 7);
    let bytes = watermarks.encoded_if_changed().unwrap();
    assert!(watermarks.encoded_if_changed().is_none());
    let mut relaunched = Watermarks::load(Some(&bytes), "account".into(), MAY_SECOND, 7);
    let starts = relaunched.plan("2026-04-26", "2026-05-02");
    assert_eq!(starts[0], (Endpoint::TokenUsage, "2026-04-30".into()));
    assert_eq!(starts[1], (Endpoint::WorkspaceCounts, "2026-04-26".into()));
    assert_eq!(starts[2], (Endpoint::Skills, "2026-04-30".into()));
    relaunched.complete(&[], "2026-05-02", MAY_SECOND, 7);
    assert!(relaunched.encoded_if_changed().is_some());
}

#[test]
fn expanded_retention_backfills_before_previous_coverage() {
    let mut watermarks = Watermarks::load(None, "account".into(), MAY_FIRST, 7);
    watermarks.plan("2026-04-25", "2026-05-01");
    watermarks.complete(&[Endpoint::TokenUsage], "2026-05-01", MAY_FIRST, 7);
    assert_eq!(watermarks.plan("2026-04-02", "2026-05-02")[0].1, "2026-04-02");
    watermarks.complete(&[Endpoint::TokenUsage], "2026-05-02", MAY_SECOND, 30);
    assert_eq!(watermarks.plan("2026-04-02", "2026-05-02")[0].1, "2026-05-01");
}

#[test]
fn legacy_corrupt_and_unknown_versions_recover() {
    let legacy = encoded(json!({"accounts":{"account":{
        "wham/usage/daily-token-usage-breakdown": "2026-05-01"
    }}}));
    let mut migrated = Watermarks::load(Some(&legacy), "account".into(), MAY_FIRST, 7);
    assert_eq!(migrated.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-25");
    let migrated = migrated.encoded_if_changed().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&migrated).unwrap()["version"],
        1
    );
    for raw in [b"bad".to_vec(), encoded(json!({"version":99,"accounts":{}}))] {
        let mut recovered = Watermarks::load(Some(&raw), "account".into(), MAY_FIRST, 7);
        assert_eq!(recovered.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-25");
        assert!(recovered.encoded_if_changed().is_some());
    }
}

#[test]
fn pruning_keeps_only_valid_current_coverage_and_four_recent_accounts() {
    let mut accounts = serde_json::Map::new();
    for (name, last_access) in [("a", 1.0), ("b", 2.0), ("c", 3.0), ("d", 4.0), ("e", 5.0)] {
        accounts.insert(
            name.into(),
            json!({"lastAccess":last_access,"coverage":{
                "wham/usage/daily-token-usage-breakdown": {"start":"2026-04-01","through":"2026-05-01"}
            }}),
        );
    }
    accounts.insert(
        "invalid".into(),
        json!({"lastAccess":6.0,"coverage":{
            "wham/usage/daily-token-usage-breakdown": {"start":"bad","through":"2026-05-01"},
        "wham/analytics/daily-workspace-usage-counts": {"start":"2026-04-25","through":"bad"},
            "wham/analytics/daily-skill-usage-metrics": {"start":"2026-05-02","through":"2026-05-01"},
            "wham/analytics/daily-plugin-usage-metrics": {"start":"2026-04-01","through":"2026-04-01"},
            "wham/analytics/daily-code-review-metrics": {"start":"2026-04-01","through":"2026-05-02"},
            "unknown":{"start":"2026-04-01","through":"2026-05-01"}
        }}),
    );
    let raw = encoded(json!({"version":1,"accounts":accounts}));
    let mut retained = Watermarks::load(Some(&raw), "e".into(), MAY_FIRST, 7);
    assert_eq!(retained.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-30");
    let saved: serde_json::Value = serde_json::from_slice(&retained.encoded_if_changed().unwrap()).unwrap();
    assert!(saved["accounts"]["a"].is_null());
    assert!(saved["accounts"]["invalid"].is_null());
    assert_eq!(
        saved["accounts"]["e"]["coverage"][Endpoint::TokenUsage.path()]["start"],
        "2026-04-25"
    );
}

#[test]
fn equal_recency_evicts_lexically_earlier_account() {
    let mut raw = json!({"version":1,"accounts":{}});
    for name in ["a", "b", "c", "d", "e"] {
        raw["accounts"][name] = json!({"lastAccess":1.0,"coverage":{
            "wham/usage/daily-token-usage-breakdown": {"start":"2026-04-25","through":"2026-05-01"}
        }});
    }
    let mut watermarks = Watermarks::load(Some(&encoded(raw)), "a".into(), MAY_FIRST, 7);
    assert_eq!(watermarks.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-25");
    let stored: serde_json::Value = serde_json::from_slice(&watermarks.encoded_if_changed().unwrap()).unwrap();
    assert!(stored["accounts"]["a"].is_null());
    assert!(!stored["accounts"]["e"].is_null());
}

#[test]
fn plan_rejects_outdated_future_and_reversed_coverage() {
    let mut watermarks = Watermarks::load(None, "account".into(), MAY_FIRST, 7);
    watermarks.plan("2026-04-25", "2026-05-01");
    watermarks.complete(&[Endpoint::TokenUsage], "2026-05-01", MAY_FIRST, 7);
    assert_eq!(watermarks.plan("2026-05-02", "2026-05-03")[0].1, "2026-05-02");
    watermarks.plan("2026-04-25", "2026-05-01");
    watermarks.complete(&[Endpoint::TokenUsage], "2026-05-03", MAY_FIRST, 7);
    assert_eq!(watermarks.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-25");
    watermarks.plan("2026-04-25", "2026-05-01");
    watermarks.complete(&[Endpoint::TokenUsage], "2026-04-24", MAY_FIRST, 7);
    assert_eq!(watermarks.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-25");
}

#[test]
fn completing_without_a_plan_does_not_invent_coverage() {
    let mut watermarks = Watermarks::load(None, "account".into(), MAY_FIRST, 7);
    watermarks.complete(&[Endpoint::TokenUsage], "2026-05-01", MAY_FIRST, 7);
    assert_eq!(watermarks.plan("2026-04-25", "2026-05-01")[0].1, "2026-04-25");
}
