//! Exposes shared Codex analytics endpoint specifications to Swift.

use quotaline_analytics::Endpoint;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexQueryParameter {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexAnalyticsSpec {
    pub path: String,
    pub query: Vec<CodexQueryParameter>,
}

#[uniffi::export]
pub fn codex_analytics_endpoints() -> Vec<Endpoint> {
    Endpoint::ALL.into()
}

#[uniffi::export]
pub fn codex_analytics_spec(endpoint: Endpoint, start: String, end: String) -> CodexAnalyticsSpec {
    CodexAnalyticsSpec {
        path: endpoint.path().into(),
        query: endpoint
            .query(&start, &end)
            .into_iter()
            .map(|(name, value)| CodexQueryParameter {
                name: name.into(),
                value,
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/codex_analytics/tests.rs"]
mod tests;
