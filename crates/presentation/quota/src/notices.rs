//! Selects provider notices that are not already shown by a full quota row.

use std::collections::BTreeSet;

pub fn notice_visible(limit_reached: bool, window_id: Option<&str>, full_windows: &BTreeSet<String>) -> bool {
    !limit_reached || window_id.is_none_or(|id| !full_windows.contains(id))
}

#[cfg(test)]
#[path = "../tests/unit/notices/tests.rs"]
mod tests;
