//! Maps shared analytics watermarks into Swift-facing values.

use crate::http::{NativeHttpTransport, NativeTransport};
use quotaline_analytics::watermark::Watermarks;
use quotaline_analytics::{AnalyticsSnapshot, Endpoint};
use quotaline_codex::fetch;
use quotaline_codex::request;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexAnalyticsStart {
    pub endpoint: Endpoint,
    pub day: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CodexAnalyticsBatchResult {
    pub snapshot: Option<AnalyticsSnapshot>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CodexAnalyticsFetchInput {
    pub token: String,
    pub account_id: Option<String>,
    pub account: String,
    pub now: f64,
    pub retention_days: u32,
    pub home: String,
}

#[derive(uniffi::Object)]
pub struct CodexAnalyticsWatermarks {
    state: Mutex<Watermarks>,
}

#[uniffi::export]
impl CodexAnalyticsWatermarks {
    #[uniffi::constructor]
    pub fn new(raw: Option<Vec<u8>>, account: String, now: f64, retention_days: u32) -> Self {
        Self {
            state: Mutex::new(Watermarks::load(raw.as_deref(), account, now, retention_days)),
        }
    }

    pub fn plan(&self, fallback: String, end: String) -> Vec<CodexAnalyticsStart> {
        self.state
            .lock()
            .unwrap()
            .plan(&fallback, &end)
            .into_iter()
            .map(|(endpoint, day)| CodexAnalyticsStart { endpoint, day })
            .collect()
    }

    pub async fn fetch_batch(
        &self,
        input: CodexAnalyticsFetchInput,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> CodexAnalyticsBatchResult {
        let account = fetch::Account {
            base: request::BASE_URL,
            token: &input.token,
            account_id: input.account_id.as_deref(),
        };
        let context = fetch::AnalyticsContext {
            fingerprint: &input.account,
            now: input.now,
            retention_days: input.retention_days,
            home: Some(&input.home),
        };
        let result = fetch::fetch_analytics(&NativeTransport(transport), &account, &self.state, &context).await;
        CodexAnalyticsBatchResult {
            snapshot: result.snapshot,
            warnings: result.warnings,
        }
    }

    pub fn complete(&self, successful: Vec<Endpoint>, end: String, now: f64, retention_days: u32) {
        self.state
            .lock()
            .unwrap()
            .complete(&successful, &end, now, retention_days);
    }

    pub fn encoded_if_changed(&self) -> Option<Vec<u8>> {
        self.state.lock().unwrap().encoded_if_changed()
    }
}

#[cfg(test)]
#[path = "../tests/unit/watermark/tests.rs"]
mod tests;
