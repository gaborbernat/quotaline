use super::*;
use crate::{RefreshPolicy, RefreshReason};
use std::collections::BTreeSet;

fn intent(usage: RefreshPolicy, providers: Option<&[&str]>) -> RefreshIntent {
    RefreshIntent {
        reason: RefreshReason::Scheduled,
        usage,
        analytics: RefreshPolicy::Skip,
        providers: providers.map(|providers| providers.iter().map(|provider| (*provider).into()).collect()),
    }
}

#[test]
fn queue_merges_pending_requests_and_suppresses_covered_requests() {
    let mut queue = RefreshQueue::default();
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    queue.enqueue(intent(RefreshPolicy::Force, Some(&["codex"])));
    assert_eq!(
        queue.take_pending(),
        Some(RefreshIntent {
            reason: RefreshReason::Scheduled,
            usage: RefreshPolicy::Force,
            analytics: RefreshPolicy::Skip,
            providers: Some(BTreeSet::from(["claude".into(), "codex".into()])),
        })
    );
    assert!(!queue.has_pending());
}

#[test]
fn active_request_blocks_covered_work_until_finished() {
    let mut queue = RefreshQueue::default();
    queue.enqueue(intent(RefreshPolicy::Force, None));
    assert!(queue.has_pending());
    assert_eq!(queue.take_pending(), Some(intent(RefreshPolicy::Force, None)));
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    assert!(!queue.has_pending());
    queue.finish_active();
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    assert!(queue.has_pending());
}

#[test]
fn queue_keeps_new_work_pending_while_active_request_runs() {
    let mut queue = RefreshQueue::default();
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    queue.take_pending();
    queue.enqueue(intent(RefreshPolicy::Force, Some(&["claude"])));
    assert_eq!(
        queue.take_pending(),
        Some(intent(RefreshPolicy::Force, Some(&["claude"])))
    );
}

#[test]
fn clear_drops_active_and_pending_work() {
    let mut queue = RefreshQueue::default();
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    queue.take_pending();
    queue.enqueue(intent(RefreshPolicy::Force, Some(&["codex"])));
    queue.clear();
    assert!(!queue.has_pending());
    assert_eq!(queue.take_pending(), None);
    queue.enqueue(intent(RefreshPolicy::IfDue, Some(&["claude"])));
    assert_eq!(
        queue.take_pending(),
        Some(intent(RefreshPolicy::IfDue, Some(&["claude"])))
    );
}
