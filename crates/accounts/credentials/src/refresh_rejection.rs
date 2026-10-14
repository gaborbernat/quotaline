//! Identifies OAuth refresh failures that require a new sign-in.

use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct RefreshRejectionLedger {
    rejected: BTreeMap<String, (Value, String)>,
}

impl RefreshRejectionLedger {
    pub fn rejected_code(&mut self, provider: &str, credential: &Value) -> Option<&str> {
        if self
            .rejected
            .get(provider)
            .is_some_and(|(rejected, _)| rejected != credential)
        {
            self.rejected.remove(provider);
        }
        self.rejected.get(provider).map(|(_, code)| code.as_str())
    }

    pub fn record_failure(&mut self, provider: &str, credential: &Value, status: u16, body: &Value) -> Option<String> {
        let code = terminal_refresh_error_code(status, body)?.to_owned();
        self.rejected
            .insert(provider.into(), (credential.clone(), code.clone()));
        Some(code)
    }

    pub fn clear(&mut self, provider: &str) {
        self.rejected.remove(provider);
    }
}

pub fn terminal_refresh_error_code(status: u16, body: &Value) -> Option<&str> {
    if !(400..500).contains(&status) {
        return None;
    }
    let code = body["error"].as_str()?;
    [
        "invalid_grant",
        "invalid_client",
        "unauthorized_client",
        "refresh_token_expired",
        "refresh_token_reused",
        "refresh_token_invalidated",
    ]
    .contains(&code)
    .then_some(code)
}

pub fn refresh_rejected_message(code: &str) -> String {
    format!("the provider rejected the stored sign-in ({code}). Sign in with the CLI again.")
}

/// Every refresh failure leaves the stored sign-in unusable, so desktops report it as a sign-in problem.
pub fn refresh_failed_message(provider: &str, detail: &str) -> String {
    format!("{provider} token refresh failed: {detail}")
}

#[cfg(test)]
#[path = "../tests/unit/refresh_rejection/tests.rs"]
mod tests;
