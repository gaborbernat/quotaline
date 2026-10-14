//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn bridge_lists_each_provider_once_with_its_policy() {
    let providers = provider_definitions();
    assert_eq!(providers.len(), 6);
    assert_eq!(providers[0].id, "claude");
    assert_eq!(providers[0].name, "Claude");
    assert_eq!(providers[0].code, "CC");
    assert_eq!(providers[0].minimum_minutes, 2);
    assert_eq!(providers[0].default_minutes, 5);
    assert!(providers[0].supports_analytics);
    assert_eq!(providers[2].id, "gemini");
    assert!(!providers[2].supports_analytics);
    assert_eq!(providers[5].id, "copilot");
}

#[test]
fn refresh_ceiling_matches_shared_minutes() {
    assert_eq!(
        maximum_refresh_seconds(),
        quotaline_catalog::MAXIMUM_REFRESH_MINUTES * 60
    );
}
