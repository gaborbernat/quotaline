//! Checks the public behavior of the adjacent module.

use super::*;
use quotaline_quota::Window;

#[test]
fn export_without_current_readings_is_empty() {
    let providers = BTreeMap::from([("claude".into(), Provider::empty("claude", "sign-in"))]);
    let export = UsageExport::from(&providers);
    assert!(export.is_empty());
    assert_eq!(
        serde_json::to_value(export).unwrap(),
        serde_json::json!({"providers": []})
    );
}

#[test]
fn export_contains_only_public_quota_fields() {
    let mut providers = BTreeMap::new();
    let mut connected = Provider::connected(
        "codex",
        "Plus".into(),
        "/private/auth.json".into(),
        "a@example.com".into(),
        vec![Window::new(
            "session",
            "5 hours",
            20.0,
            Some("2026-10-10T12:00:00Z".into()),
            "session",
        )],
    );
    connected.updated = Some(1_780_000_000);
    providers.insert("codex".into(), connected);
    providers.insert("claude".into(), Provider::empty("claude", "sign-in"));
    let export = UsageExport::from(&providers);
    let value = serde_json::to_value(export).unwrap();
    assert_eq!(value["providers"].as_array().unwrap().len(), 1);
    assert_eq!(value["providers"][0]["windows"][0]["usedPercent"], 20.0);
    assert!(value.to_string().contains("fetchedAt"));
    assert!(!value.to_string().contains("/private/auth.json"));
    assert!(!value.to_string().contains("a@example.com"));
}

#[test]
fn export_json_sorts_providers_and_omits_empty_plans() {
    let mut claude = Provider::empty("claude", "connected");
    claude.plan = "Max".into();
    claude.updated = Some(1_780_000_000);
    claude.windows = vec![Window::new(
        "session",
        "Current session",
        36.0,
        Some("2026-05-28T23:26:40.250+02:00".into()),
        "session",
    )];
    let mut codex = Provider::empty("codex", "connected");
    codex.updated = Some(1_780_000_000);
    let providers = BTreeMap::from([("codex".into(), codex), ("claude".into(), claude)]);
    let value: serde_json::Value = serde_json::from_slice(&UsageExport::from(&providers).json()).unwrap();
    assert_eq!(value["providers"][0]["id"], "claude");
    assert_eq!(value["providers"][0]["plan"], "Max");
    assert_eq!(value["providers"][0]["fetchedAt"], "2026-05-28T20:26:40Z");
    assert_eq!(value["providers"][0]["windows"][0]["resetsAt"], "2026-05-28T21:26:40Z");
    assert!(value["providers"][1].get("plan").is_none());
}
