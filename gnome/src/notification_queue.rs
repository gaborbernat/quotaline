//! Holds planned notices until GNOME Shell is active to receive them.

use quotaline_notifications::Event;
use std::collections::VecDeque;

pub(super) struct Planned {
    pub(super) provider: String,
    pub(super) event: Event,
    pub(super) sign_in: Option<String>,
    pub(super) fetched_at: i64,
}

pub(super) struct NotificationQueue<const CAPACITY: usize>(VecDeque<Planned>);

impl<const CAPACITY: usize> Default for NotificationQueue<CAPACITY> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<const CAPACITY: usize> NotificationQueue<CAPACITY> {
    pub(super) fn extend(&mut self, events: impl IntoIterator<Item = Planned>) {
        self.0.extend(events);
        if self.0.len() > CAPACITY {
            self.0.drain(..self.0.len() - CAPACITY);
        }
    }

    pub(super) fn take(&mut self) -> Vec<Planned> {
        std::mem::take(&mut self.0).into_iter().collect()
    }

    #[cfg(feature = "fixture-api")]
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
#[path = "../tests/unit/notification_queue/tests.rs"]
mod tests;
