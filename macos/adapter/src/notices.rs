//! Bridges notice visibility to macOS usage cards.

use quotaline_quota::notices::notice_visible;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NoticeCandidate {
    pub limit_reached: bool,
    pub window_id: Option<String>,
}

#[uniffi::export]
pub fn visible_notice_indices(candidates: Vec<NoticeCandidate>, full_windows: Vec<String>) -> Vec<u64> {
    let full_windows = full_windows.into_iter().collect::<BTreeSet<_>>();
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, notice)| {
            notice_visible(notice.limit_reached, notice.window_id.as_deref(), &full_windows).then_some(index as u64)
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/notices/tests.rs"]
mod tests;
