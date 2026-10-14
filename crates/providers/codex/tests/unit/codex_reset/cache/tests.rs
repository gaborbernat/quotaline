use super::*;

#[test]
fn complete_inline_credit_replaces_the_cache() {
    let mut cache = ResetCreditCache::default();
    cache.record("account", Some(2), None, 100.0);
    assert_eq!(
        cache.read("account", Some(5), true, 101.0),
        ResetCreditCacheRead {
            value: Some(5),
            warning: None,
            should_fetch: false
        }
    );
    assert_eq!(cache.read("account", None, false, 102.0).value, Some(5));
}

#[test]
fn success_cache_expires_at_the_fifteenth_minute() {
    let mut cache = ResetCreditCache::default();
    cache.record("account", Some(2), None, 100.0);
    assert!(
        !cache
            .read("account", None, false, 100.0 + RESET_CREDIT_SUCCESS_TTL - 1.0)
            .should_fetch
    );
    assert_eq!(
        cache.read("account", Some(3), false, 100.0 + RESET_CREDIT_SUCCESS_TTL),
        ResetCreditCacheRead {
            value: Some(2),
            warning: None,
            should_fetch: true
        }
    );
}

#[test]
fn failure_cache_expires_at_the_fifth_minute() {
    let mut cache = ResetCreditCache::default();
    cache.record("account", None, Some("unavailable".into()), 100.0);
    assert_eq!(
        cache.read("account", Some(3), false, 100.0 + RESET_CREDIT_FAILURE_TTL - 1.0),
        ResetCreditCacheRead {
            value: Some(3),
            warning: Some("unavailable".into()),
            should_fetch: false
        }
    );
    assert_eq!(
        cache.read("account", Some(3), false, 100.0 + RESET_CREDIT_FAILURE_TTL),
        ResetCreditCacheRead {
            value: Some(3),
            warning: None,
            should_fetch: true
        }
    );
}

#[test]
fn warning_shortens_the_ttl_even_when_credits_are_available() {
    let mut cache = ResetCreditCache::default();
    cache.record("account", Some(2), Some("unavailable".into()), 100.0);
    assert_eq!(
        cache.read("account", None, false, 100.0 + RESET_CREDIT_FAILURE_TTL - 1.0),
        ResetCreditCacheRead {
            value: Some(2),
            warning: Some("unavailable".into()),
            should_fetch: false
        }
    );
    assert_eq!(
        cache.read("account", None, false, 100.0 + RESET_CREDIT_FAILURE_TTL),
        ResetCreditCacheRead {
            value: Some(2),
            warning: None,
            should_fetch: true
        }
    );
}

#[test]
fn cached_credit_is_scoped_to_its_account() {
    let mut cache = ResetCreditCache::default();
    cache.record("first", Some(2), None, 100.0);
    assert_eq!(
        cache.read("second", None, false, 101.0),
        ResetCreditCacheRead {
            value: None,
            warning: None,
            should_fetch: true
        }
    );
    assert_eq!(cache.read("first", None, false, 101.0).value, Some(2));
}

#[test]
fn old_accounts_are_discarded_when_the_cache_fills() {
    let mut cache = ResetCreditCache::default();
    for index in 0..MAX_ACCOUNTS {
        cache.record(&index.to_string(), Some(index as i32), None, 100.0);
    }
    cache.record("0", Some(42), None, 101.0);
    assert_eq!(cache.read("0", None, false, 102.0).value, Some(42));
    cache.record("new", Some(10), None, 102.0);
    assert!(cache.read("1", None, false, 103.0).should_fetch);
    assert_eq!(cache.read("new", None, false, 103.0).value, Some(10));
}

#[test]
fn clear_discards_cached_credit() {
    let mut cache = ResetCreditCache::default();
    cache.record("account", Some(2), None, 100.0);
    cache.clear();
    assert_eq!(cache.read("account", None, false, 101.0).value, None);
}
