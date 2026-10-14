//! Tracks which providers still need a staggered refresh after their first poll.

use std::collections::BTreeSet;

#[derive(Default)]
pub struct RefreshStagger {
    pending: BTreeSet<String>,
}

impl RefreshStagger {
    pub fn new(providers: impl IntoIterator<Item = String>) -> Self {
        Self {
            pending: providers.into_iter().collect(),
        }
    }

    pub fn reset(&mut self, providers: impl IntoIterator<Item = String>) {
        self.pending = providers.into_iter().collect();
    }

    pub fn set_pending(&mut self, provider: &str, pending: bool) {
        if pending {
            self.pending.insert(provider.into());
        } else {
            self.pending.remove(provider);
        }
    }

    pub fn interval(&self, provider: &str, interval: f64, index: usize, visible: bool, count: usize) -> f64 {
        super::staggered_interval(interval, index, visible, count, self.pending.contains(provider))
    }
}

#[cfg(test)]
#[path = "../tests/unit/stagger/tests.rs"]
mod tests;
