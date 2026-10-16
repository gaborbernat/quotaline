use super::*;

#[test]
fn retry_counts_are_isolated_and_reset_after_success() {
    let mut ledger = RetryLedger::default();
    let first = ledger.advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    let second = ledger.advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    let other = ledger.advance_with_minute("codex", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    assert_eq!(
        (
            first.rate_limit_strikes,
            second.rate_limit_strikes,
            other.rate_limit_strikes
        ),
        (1, 2, 1)
    );
    assert_eq!(
        ledger
            .advance_with_minute("claude", FetchOutcome::Success, None, 60.0, 1000.0, 60.0)
            .rate_limit_strikes,
        0
    );
    assert_eq!(
        ledger
            .advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0)
            .rate_limit_strikes,
        1
    );
}

#[test]
fn retry_kind_changes_clear_the_other_counter() {
    let mut ledger = RetryLedger::default();
    ledger.advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    let failed = ledger.advance_with_minute("claude", FetchOutcome::Failed, None, 60.0, 1000.0, 60.0);
    assert_eq!((failed.rate_limit_strikes, failed.failure_strikes), (0, 1));
    let limited = ledger.advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    assert_eq!((limited.rate_limit_strikes, limited.failure_strikes), (1, 0));
}

#[test]
fn removed_providers_lose_retry_history() {
    let mut ledger = RetryLedger::default();
    ledger.advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    ledger.advance_with_minute("codex", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0);
    ledger.retain_providers(&BTreeSet::from(["codex".into()]));
    assert_eq!(
        ledger
            .advance_with_minute("claude", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0)
            .rate_limit_strikes,
        1
    );
    assert_eq!(
        ledger
            .advance_with_minute("codex", FetchOutcome::RateLimited, None, 60.0, 1000.0, 60.0)
            .rate_limit_strikes,
        2
    );
}

#[test]
fn retry_ledger_scales_both_backoffs_for_fast_schedules() {
    let mut ledger = RetryLedger::default();
    let limited = ledger.advance_with_minute("claude", FetchOutcome::RateLimited, Some(2.0), 1.0, 1000.0, 1.0);
    let failed = ledger.advance_with_minute("codex", FetchOutcome::Failed, None, 1.0, 1000.0, 1.0);
    assert_eq!(limited.retry_not_before, Some(1005.0));
    assert_eq!(failed.retry_not_before, Some(1001.0));
}
