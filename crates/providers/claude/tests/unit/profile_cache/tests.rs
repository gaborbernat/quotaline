use super::*;

#[test]
fn empty_cache_requests_profile() {
    assert_eq!(
        ProfileCache::<String>::default().read("account", 100.0),
        ProfileCacheRead {
            value: None,
            warning: None,
            should_fetch: true
        }
    );
}

#[test]
fn successful_profile_expires_after_six_hours() {
    let mut cache = ProfileCache::default();
    cache.record("account", Some("profile"), None, 100.0);
    assert!(!cache.read("account", 100.0 + PROFILE_SUCCESS_TTL - 1.0).should_fetch);
    assert_eq!(
        cache.read("account", 100.0 + PROFILE_SUCCESS_TTL),
        ProfileCacheRead {
            value: Some("profile"),
            warning: None,
            should_fetch: true
        }
    );
}

#[test]
fn failed_profile_expires_after_five_minutes() {
    let mut cache = ProfileCache::<String>::default();
    cache.record("account", None, Some("unavailable".into()), 100.0);
    assert_eq!(cache.read("account", 101.0).warning.as_deref(), Some("unavailable"));
    assert!(!cache.read("account", 100.0 + PROFILE_FAILURE_TTL - 1.0).should_fetch);
    assert!(cache.read("account", 100.0 + PROFILE_FAILURE_TTL).should_fetch);
}

#[test]
fn profile_with_warning_uses_the_short_lifetime() {
    let mut cache = ProfileCache::default();
    cache.record("account", Some("stale"), Some("unavailable".into()), 100.0);
    assert_eq!(cache.read("account", 101.0).value, Some("stale"));
    assert!(cache.read("account", 100.0 + PROFILE_FAILURE_TTL).should_fetch);
}

#[test]
fn profile_is_scoped_to_its_credential() {
    let mut cache = ProfileCache::default();
    cache.record("first", Some("profile"), None, 100.0);
    assert_eq!(cache.read("second", 101.0).value, None);
    assert!(cache.read("second", 101.0).should_fetch);
}

#[test]
fn successful_profile_replaces_failed_reading() {
    let mut cache = ProfileCache::default();
    cache.record("account", None, Some("unavailable".into()), 100.0);
    cache.record("account", Some("profile"), None, 101.0);
    assert_eq!(cache.read("account", 102.0).value, Some("profile"));
    assert_eq!(cache.read("account", 102.0).warning, None);
}

#[test]
fn failed_attempt_keeps_stale_profile_and_explains_failure() {
    let mut cache = ProfileCache::default();
    cache.record("account", Some("profile"), None, 0.0);
    let reading = cache.record_attempt("account", Err("HTTP 503".into()), PROFILE_SUCCESS_TTL);
    assert_eq!(reading.value, Some("profile"));
    assert_eq!(reading.warning.as_deref(), Some("Profile unavailable: HTTP 503"));
    assert!(!reading.should_fetch);
    assert!(
        cache
            .read("account", PROFILE_SUCCESS_TTL + PROFILE_FAILURE_TTL)
            .should_fetch
    );
}

#[test]
fn failed_attempt_without_profile_waits_for_short_retry() {
    let mut cache = ProfileCache::<String>::default();
    let reading = cache.record_attempt("account", Err("HTTP 500".into()), 10.0);
    assert_eq!(reading.value, None);
    assert_eq!(reading.warning.as_deref(), Some("Profile unavailable: HTTP 500"));
    assert!(!cache.read("account", 11.0).should_fetch);
}

#[test]
fn successful_attempt_replaces_warning() {
    let mut cache = ProfileCache::default();
    cache.record_attempt("account", Err("HTTP 500".into()), 0.0);
    let reading = cache.record_attempt("account", Ok("profile"), PROFILE_FAILURE_TTL);
    assert_eq!(reading.value, Some("profile"));
    assert_eq!(reading.warning, None);
}
