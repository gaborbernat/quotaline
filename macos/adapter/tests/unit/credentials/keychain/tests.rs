use super::*;

#[test]
fn ffi_retry_services_include_variant_policy() {
    assert_eq!(
        keychain_retry_service("claude".into()),
        Some(KeychainRetryService {
            service: "Claude Code-credentials".into(),
            including_variants: true,
        })
    );
    assert_eq!(keychain_retry_service("cursor".into()), None);
}

#[test]
fn ffi_cache_tracks_denials_and_retries() {
    let cache = KeychainDenialCache::default();
    assert_eq!(keychain_denial_cache_interval(), 1800.0);
    assert!(keychain_is_denial(-25293));
    assert!(!keychain_is_denial(-25291));
    cache.record(-25293, "service".into(), Some("alice".into()), 1000.0);
    assert_eq!(
        cache.status("service".into(), Some("alice".into()), 1000.0),
        Some(-25293)
    );
    cache.invalidate("service".into(), false);
    assert_eq!(cache.status("service".into(), Some("alice".into()), 1000.0), None);
}
