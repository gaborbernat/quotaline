//! Checks controls behavior through the public API.

use super::*;

#[test]
fn invalid_display_settings_are_rejected() {
    for (key, value) in [
        ("order", serde_json::json!("random")),
        ("show_as", serde_json::json!("both")),
        ("short_labels", serde_json::json!({"claude:session":"toolong"})),
    ] {
        let mut settings = Settings::default();
        assert!(settings.set(key, value).is_err(), "{key}");
    }
}

#[test]
fn explicit_provider_activation_is_recorded() {
    let mut settings = Settings::default();
    settings.set("provider:copilot", serde_json::json!(true)).unwrap();
    assert!(settings.providers["copilot"]);
    assert!(settings.configured_providers.contains("copilot"));
}

#[test]
fn explicit_provider_deactivation_is_recorded() {
    let mut settings = Settings::default();
    settings.set("provider:copilot", serde_json::json!(false)).unwrap();
    assert!(!settings.providers["copilot"]);
    assert!(settings.configured_providers.contains("copilot"));
}

#[test]
fn provider_control_rejects_unknown_ids_and_non_boolean_values() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.set("provider:unknown", serde_json::json!(true)),
        Err("Unknown provider".into())
    );
    assert_eq!(
        settings.set("provider:copilot", serde_json::json!(1)),
        Err("Invalid provider state".into())
    );
    assert!(settings.configured_providers.is_empty());
}

#[test]
fn bulk_provider_change_marks_only_changed_providers_as_configured() {
    let mut settings = Settings::default();
    let mut providers = settings.providers.clone();
    providers.insert("gemini".into(), false);
    settings.set("providers", serde_json::json!(providers)).unwrap();
    assert_eq!(settings.configured_providers, BTreeSet::from(["gemini".into()]));
    settings.set("providers", serde_json::json!(providers)).unwrap();
    assert_eq!(settings.configured_providers, BTreeSet::from(["gemini".into()]));
}

#[test]
fn unknown_configured_provider_is_removed_when_settings_change() {
    let mut settings = Settings::default();
    settings.configured_providers.insert("unknown".into());
    settings.set("show_all_providers", serde_json::json!(true)).unwrap();
    assert!(settings.configured_providers.is_empty());
}
