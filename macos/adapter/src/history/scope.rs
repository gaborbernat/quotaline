//! Bridges shared history window scope filtering to Swift.

use quotaline_analytics as analytics;
use quotaline_analytics::ScopeKey;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct HistoryScopeKey {
    pub provider: String,
    pub storage_key: String,
}

#[uniffi::export]
pub fn history_scoped_indices(
    keys: Vec<HistoryScopeKey>,
    active_providers: Vec<String>,
    selected_keys: Option<Vec<String>>,
) -> Vec<u64> {
    analytics::scoped_indices(
        &keys
            .into_iter()
            .map(|key| ScopeKey {
                provider: key.provider,
                storage_key: key.storage_key,
            })
            .collect::<Vec<_>>(),
        &active_providers,
        selected_keys.as_deref(),
    )
    .into_iter()
    .map(|index| index as u64)
    .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/history/scope/tests.rs"]
mod tests;
