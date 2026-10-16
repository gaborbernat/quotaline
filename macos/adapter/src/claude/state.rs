//! Exposes immutable Claude transcript aggregates to Swift.

use std::sync::Arc;

use quotaline_claude::claude_transcripts::TranscriptState;

use quotaline_analytics::AnalyticsSnapshot;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ClaudeStateLocalUsage {
    pub window_tokens: i64,
    pub window_cost: f64,
    pub cost_per_hour: f64,
    pub today_tokens: i64,
    pub today_cost: f64,
    pub today_messages: i64,
}

#[derive(uniffi::Object)]
pub struct ClaudeTranscriptView {
    state: Arc<TranscriptState>,
    projects_complete: bool,
}

impl ClaudeTranscriptView {
    pub(super) fn from_state(state: Arc<TranscriptState>, projects_complete: bool) -> Arc<Self> {
        Arc::new(Self {
            state,
            projects_complete,
        })
    }
}

#[uniffi::export]
impl ClaudeTranscriptView {
    pub fn message_count(&self) -> i64 {
        self.state.message_count()
    }

    pub fn analytics_snapshot(&self, now: f64) -> Option<AnalyticsSnapshot> {
        self.state
            .analytics()
            .map(|analytics| analytics.snapshot(now, self.projects_complete))
    }

    pub fn local_usage(
        &self,
        reset_at: Option<f64>,
        duration: f64,
        today_start: f64,
        now: f64,
    ) -> Option<ClaudeStateLocalUsage> {
        self.state
            .local_usage_for_window(reset_at, duration, today_start, now)
            .map(|usage| ClaudeStateLocalUsage {
                window_tokens: usage.window_tokens,
                window_cost: usage.window_cost,
                cost_per_hour: usage.cost_per_hour,
                today_tokens: usage.today_tokens,
                today_cost: usage.today_cost,
                today_messages: usage.today_messages,
            })
    }
}
