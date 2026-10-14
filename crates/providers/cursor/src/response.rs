//! Normalizes Cursor's summary and period responses.

use serde_json::Value;

use super::period_summary;

#[derive(Debug, PartialEq)]
pub struct UsageResponse {
    pub summary: Value,
    pub period_message: Option<String>,
    pub warning: Option<String>,
}

impl UsageResponse {
    pub fn summary(summary: Value) -> Self {
        Self {
            summary,
            period_message: None,
            warning: None,
        }
    }

    pub fn period(period: &Value, summary_error: &str) -> Self {
        Self {
            summary: period_summary(period),
            period_message: period["displayMessage"]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            warning: Some(format!("Dashboard summary unavailable: {summary_error}")),
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/response/tests.rs"]
mod tests;
