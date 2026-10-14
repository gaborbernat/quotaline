use super::*;

#[test]
fn retry_services_match_provider_credentials() {
    for (provider, expected) in [
        ("claude", Some(("Claude Code-credentials", true))),
        ("codex", Some(("Codex Auth", false))),
        ("gemini", Some(("gemini-cli-oauth", false))),
        ("antigravity", Some(("gemini", false))),
        ("copilot", Some(("copilot-cli", false))),
        ("cursor", None),
        ("unknown", None),
    ] {
        assert_eq!(
            retry_service(provider),
            expected.map(|(service, variants)| (service.into(), variants)),
            "{provider}"
        );
    }
}

#[test]
fn denied_statuses_are_distinct_from_other_keychain_errors() {
    for status in [-25293, -128, -25308] {
        assert!(is_denial(status));
    }
    assert!(!is_denial(-25291));
}

#[test]
fn services_filter_sort_and_deduplicate() {
    assert_eq!(
        services(
            "Claude",
            &["Claude-b".into(), "Codex".into(), "Claude-a".into(), "Claude-b".into()]
        ),
        ["Claude-a", "Claude-b"]
    );
}

#[test]
fn cache_scopes_denials_by_service_and_account_until_expiry() {
    let mut cache = DenialCache::default();
    cache.record(-25293, "service", Some("alice"), 1000.0);
    assert_eq!(cache.status("service", Some("alice"), 1000.0), Some(-25293));
    assert_eq!(cache.status("service", Some("bob"), 1000.0), None);
    assert_eq!(cache.status("other", Some("alice"), 1000.0), None);
    assert_eq!(
        cache.status("service", Some("alice"), 1000.0 + DENIAL_CACHE_INTERVAL),
        None
    );
}

#[test]
fn cache_invalidation_can_include_service_variants() {
    let mut cache = DenialCache::default();
    for service in ["service", "service-variant", "other"] {
        cache.record(-128, service, None, 0.0);
    }
    cache.invalidate("service", false);
    assert_eq!(cache.status("service", None, 0.0), None);
    assert_eq!(cache.status("service-variant", None, 0.0), Some(-128));
    cache.invalidate("service", true);
    assert_eq!(cache.status("service-variant", None, 0.0), None);
    assert_eq!(cache.status("other", None, 0.0), Some(-128));
}
