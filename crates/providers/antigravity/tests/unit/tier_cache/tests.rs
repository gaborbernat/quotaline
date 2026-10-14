use super::*;

#[test]
fn empty_cache_requires_a_request() {
    assert_eq!(TierCache::<u8>::default().read("token", 100.0), None);
}

#[test]
fn tier_expires_after_one_hour() {
    let mut cache = TierCache::default();
    cache.record("token", "tier", 100.0);
    assert_eq!(cache.read("token", 100.0 + TIER_CACHE_TTL - 1.0), Some("tier"));
    assert_eq!(cache.read("token", 100.0 + TIER_CACHE_TTL), None);
}

#[test]
fn different_token_cannot_reuse_a_tier() {
    let mut cache = TierCache::default();
    cache.record("first", "tier", 100.0);
    assert_eq!(cache.read("second", 101.0), None);
}

#[test]
fn missing_tier_can_be_cached() {
    let mut cache = TierCache::default();
    cache.record("token", None::<u8>, 100.0);
    assert_eq!(cache.read("token", 101.0), Some(None));
}

#[test]
fn new_tier_replaces_the_previous_one() {
    let mut cache = TierCache::default();
    cache.record("token", "old", 100.0);
    cache.record("token", "new", 101.0);
    assert_eq!(cache.read("token", 102.0), Some("new"));
}
