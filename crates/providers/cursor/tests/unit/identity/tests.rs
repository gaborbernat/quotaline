//! Exercises Cursor identity cache expiry and account isolation.

use super::*;

#[test]
fn token_digest_is_stable_and_account_specific() {
    assert_eq!(token_digest("first"), token_digest("first"));
    assert_ne!(token_digest("first"), token_digest("second"));
}

#[test]
fn successful_identity_is_cached_until_its_deadline() {
    let mut cache = Cache::default();
    assert_eq!(cache.get("token", 100.0), None);
    cache.remember("token", Some("account@example.com".into()), 100.0);
    assert_eq!(cache.get("token", 100.0), Some(Some("account@example.com".into())));
    assert_eq!(
        cache.get("token", 100.0 + SUCCESS_TTL_SECONDS - 1.0),
        Some(Some("account@example.com".into()))
    );
    assert_eq!(cache.get("token", 100.0 + SUCCESS_TTL_SECONDS), None);
}

#[test]
fn failed_identity_is_cached_for_a_shorter_period() {
    let mut cache = Cache::default();
    cache.remember("token", None, 100.0);
    assert_eq!(cache.get("token", 100.0 + FAILURE_TTL_SECONDS - 1.0), Some(None));
    assert_eq!(cache.get("token", 100.0 + FAILURE_TTL_SECONDS), None);
}

#[test]
fn another_token_cannot_read_or_reuse_the_cached_identity() {
    let mut cache = Cache::default();
    cache.remember("first", Some("first@example.com".into()), 100.0);
    assert_eq!(cache.get("second", 101.0), None);
    cache.remember("second", Some("second@example.com".into()), 101.0);
    assert_eq!(cache.get("first", 102.0), None);
    assert_eq!(cache.get("second", 102.0), Some(Some("second@example.com".into())));
}
