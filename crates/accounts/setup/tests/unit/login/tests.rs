//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn opening_waits_for_launch_and_returns_once() {
    let mut flow = LoginFlow::default();
    assert!(flow.begin("claude"));
    assert!(flow.take_returned().is_empty());
    assert!(!flow.begin("claude"));
    flow.opened("claude");
    assert!(!flow.begin("claude"));
    assert_eq!(flow.take_returned(), ["claude"]);
    assert!(flow.take_returned().is_empty());
    assert!(flow.begin("claude"));
}

#[test]
fn failure_allows_retry() {
    let mut flow = LoginFlow::default();
    assert!(flow.begin("codex"));
    flow.failed("codex");
    assert!(flow.begin("codex"));
}

#[test]
fn return_preserves_other_launches() {
    let mut flow = LoginFlow::default();
    flow.begin("claude");
    flow.begin("codex");
    flow.opened("codex");
    assert_eq!(flow.take_returned(), ["codex"]);
    assert!(!flow.begin("claude"));
    assert!(flow.begin("codex"));
}
