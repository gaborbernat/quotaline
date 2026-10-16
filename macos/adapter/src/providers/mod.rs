//! Exports provider-specific UniFFI records and functions.

pub mod antigravity;
pub mod claude;
pub mod codex_analytics;
pub mod codex_credentials;
pub mod codex_reset;
pub mod codex_rollout_reader;
pub mod codex_usage;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod google_credentials;
pub mod oauth_client;
pub mod oauth_refresh;
pub mod refresh_completion;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProviderDefinition {
    pub id: String,
    pub name: String,
    pub code: String,
    pub minimum_minutes: u32,
    pub default_minutes: u32,
    pub supports_analytics: bool,
}

#[uniffi::export]
pub fn provider_definitions() -> Vec<ProviderDefinition> {
    quotaline_catalog::ALL
        .into_iter()
        .map(|provider| ProviderDefinition {
            id: provider.id.into(),
            name: provider.name.into(),
            code: provider.code.into(),
            minimum_minutes: provider.minimum_minutes,
            default_minutes: provider.default_minutes,
            supports_analytics: provider.supports_analytics,
        })
        .collect()
}

#[uniffi::export]
pub fn maximum_refresh_seconds() -> u32 {
    quotaline_catalog::MAXIMUM_REFRESH_MINUTES * 60
}

#[cfg(test)]
#[path = "../../tests/unit/providers/tests.rs"]
mod tests;
