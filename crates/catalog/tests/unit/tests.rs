//! Checks the provider registry and its shared refresh policy.

use super::*;

#[test]
fn registry_defines_each_provider_and_its_shared_policy() {
    assert_eq!(IDS, ["claude", "codex", "gemini", "antigravity", "cursor", "copilot"]);
    assert_eq!(CODES, ["CC", "CX", "GM", "AG", "CU", "CP"]);
    assert_eq!(ALL.map(|provider| provider.minimum_minutes), [2, 1, 1, 1, 1, 1]);
    assert_eq!(ALL.map(|provider| provider.default_minutes), [5, 2, 2, 2, 5, 5]);
    assert_eq!(MAXIMUM_REFRESH_MINUTES, 30);
    assert_eq!(
        ALL.map(|provider| provider.supports_analytics),
        [true, true, false, false, false, false]
    );
    assert_eq!(REFRESH_POLICY[0], ("claude", 2, 5));
}

#[test]
fn lookup_finds_known_providers_only() {
    assert_eq!(definition("gemini"), Some(&GEMINI));
    assert_eq!(definition("unknown"), None);
}

#[test]
fn name_and_login_fall_back_for_unknown_providers() {
    assert_eq!((name("copilot"), login("copilot")), ("GitHub Copilot", "copilot login"));
    assert_eq!((name("unknown"), login("unknown")), ("Provider", ""));
}

#[test]
fn unknown_provider_has_generic_name_and_no_login_command() {
    assert_eq!(name("unknown"), "Provider");
    assert_eq!(login("unknown"), "");
}

#[test]
fn every_provider_has_a_login_hint() {
    for (id, expected) in [
        ("claude", "claude auth login"),
        ("codex", "codex login"),
        ("gemini", "gemini"),
        ("antigravity", "Sign in through Antigravity"),
        ("cursor", "cursor-agent login"),
        ("copilot", "copilot login"),
    ] {
        assert_eq!(login(id), expected);
    }
}
