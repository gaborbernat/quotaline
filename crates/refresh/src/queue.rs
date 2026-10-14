//! Coalesces refresh requests while a provider refresh is running.

use super::RefreshIntent;

#[derive(Default)]
pub struct RefreshQueue {
    active: Option<RefreshIntent>,
    pending: Option<RefreshIntent>,
}

impl RefreshQueue {
    pub fn enqueue(&mut self, request: RefreshIntent) {
        if self.active.as_ref().is_some_and(|active| active.covers(&request))
            || self.pending.as_ref().is_some_and(|pending| pending.covers(&request))
        {
            return;
        }
        self.pending = Some(match self.pending.take() {
            Some(pending) => pending.merged(request),
            None => request,
        });
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn take_pending(&mut self) -> Option<RefreshIntent> {
        let request = self.pending.take()?;
        self.active = Some(request.clone());
        Some(request)
    }

    pub fn finish_active(&mut self) {
        self.active = None;
    }

    pub fn clear(&mut self) {
        self.active = None;
        self.pending = None;
    }
}

#[cfg(test)]
#[path = "../tests/unit/queue/tests.rs"]
mod tests;
