//! Decides how fetched quota data replaces the previous account reading.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapshotTransitionInput<'a> {
    pub previous_account: Option<&'a str>,
    pub fetched_account: Option<&'a str>,
    pub previous_fetched_at: Option<f64>,
    pub fetched_at: Option<f64>,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotTransition {
    pub account_changed: bool,
    pub replace_snapshot: bool,
    pub record_history: bool,
}

pub fn snapshot_transition(input: SnapshotTransitionInput<'_>) -> SnapshotTransition {
    let account_changed = input
        .fetched_account
        .is_some_and(|account| input.previous_account != Some(account));
    SnapshotTransition {
        account_changed,
        replace_snapshot: input.complete
            || account_changed
            || input
                .previous_fetched_at
                .is_none_or(|previous| input.fetched_at.is_some_and(|fetched| fetched >= previous)),
        record_history: input.complete,
    }
}

#[cfg(test)]
#[path = "../tests/unit/snapshot/tests.rs"]
mod tests;
