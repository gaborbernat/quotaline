use super::{RediscoveryPolicy, RediscoveryTrigger};

#[test]
fn activation_is_throttled_until_interval_elapses() {
    let mut policy = RediscoveryPolicy::new(60.0, None);
    assert!(policy.begin(RediscoveryTrigger::ApplicationActivated, 100.0));
    assert!(!policy.begin(RediscoveryTrigger::ApplicationActivated, 159.0));
    assert!(policy.begin(RediscoveryTrigger::ApplicationActivated, 160.0));
}

#[test]
fn recorded_discovery_throttles_activation() {
    let mut policy = RediscoveryPolicy::new(60.0, None);
    policy.record_discovery(100.0);
    assert!(!policy.begin(RediscoveryTrigger::ApplicationActivated, 100.0));
}

#[test]
fn manual_discovery_ignores_throttle() {
    let mut policy = RediscoveryPolicy::new(60.0, Some(100.0));
    assert!(policy.begin(RediscoveryTrigger::UserInitiated, 101.0));
    assert!(!policy.begin(RediscoveryTrigger::ApplicationActivated, 102.0));
}

#[test]
fn clock_rollback_allows_discovery() {
    let mut policy = RediscoveryPolicy::new(60.0, Some(100.0));
    assert!(policy.begin(RediscoveryTrigger::ApplicationActivated, 99.0));
}
