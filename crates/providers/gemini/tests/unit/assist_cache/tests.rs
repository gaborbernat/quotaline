use super::*;

#[test]
fn credit_balances_expire_after_one_minute() {
    let mut cache = AssistCache::default();
    cache.record("account", "assist", true, 100.0);
    assert_eq!(cache.read("account", 100.0 + ASSIST_BALANCE_TTL - 1.0), Some("assist"));
    assert_eq!(cache.read("account", 100.0 + ASSIST_BALANCE_TTL), None);
}

#[test]
fn assist_without_balances_expires_after_one_hour() {
    let mut cache = AssistCache::default();
    cache.record("account", "assist", false, 100.0);
    assert_eq!(cache.read("account", 100.0 + ASSIST_DEFAULT_TTL - 1.0), Some("assist"));
    assert_eq!(cache.read("account", 100.0 + ASSIST_DEFAULT_TTL), None);
}

#[test]
fn assist_is_scoped_to_its_credential() {
    let mut cache = AssistCache::default();
    cache.record("first", "assist", false, 100.0);
    assert_eq!(cache.read("second", 101.0), None);
}

#[test]
fn new_response_replaces_previous_credit_policy() {
    let mut cache = AssistCache::default();
    cache.record("account", "previous", false, 100.0);
    cache.record("account", "current", true, 101.0);
    assert_eq!(cache.read("account", 102.0), Some("current"));
    assert_eq!(cache.read("account", 101.0 + ASSIST_BALANCE_TTL), None);
}
