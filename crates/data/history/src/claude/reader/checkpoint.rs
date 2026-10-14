//! Restores and persists transcript progress without blocking future retries.

use super::Reader;
use quotaline_claude::claude_transcripts::TranscriptState;
use std::sync::Arc;

impl Reader {
    pub(super) fn load_state(&mut self) {
        if self.state_loaded {
            return;
        }
        self.state_loaded = true;
        self.next_checkpoint_at = f64::NEG_INFINITY;
        let Some(path) = &self.state_path else {
            return;
        };
        let Some(data) = quotaline_storage::read_optional_file(path).ok().flatten() else {
            return;
        };
        let Ok(state) = TranscriptState::from_checkpoint(&data) else {
            return;
        };
        self.scan_generation = state.maximum_scan_generation();
        if state.requires_checkpoint() {
            self.mark_dirty(0);
        }
        self.state = Arc::new(state);
    }

    pub(super) fn checkpoint_if_needed(&mut self, now: f64) {
        if !self.state_dirty {
            return;
        }
        if let Some(retry) = self.checkpoint_retry_at {
            if now < retry {
                return;
            }
        } else if now < self.next_checkpoint_at && self.uncheckpointed_bytes < self.config.maximum_checkpoint_bytes {
            return;
        }
        let Some(path) = &self.state_path else {
            self.state_dirty = false;
            self.uncheckpointed_bytes = 0;
            return;
        };
        self.workload.checkpoint_attempts += 1;
        let data = self
            .state
            .checkpoint()
            .expect("transcript state contains only JSON values");
        let written = quotaline_storage::write_private_file(path, &data).ok();
        let Some(written) = written else {
            self.checkpoint_retry_at = Some(now + self.config.checkpoint_interval.clamp(1.0, 60.0));
            return;
        };
        self.checkpoint_retry_at = None;
        self.workload.checkpoints += 1;
        self.workload.checkpoint_bytes_written += written as u64;
        self.workload.last_checkpoint_bytes = written as u64;
        self.uncheckpointed_bytes = 0;
        self.state_dirty = false;
        self.next_checkpoint_at = now + self.config.checkpoint_interval;
    }
}
