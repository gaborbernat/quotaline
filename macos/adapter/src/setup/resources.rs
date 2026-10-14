//! Exposes macOS sandbox resource metadata and paths to Swift.

use std::{collections::HashMap, path::Path};

use quotaline_macos_platform::setup::resources::{self, Need};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SandboxResourceNeed {
    Required,
    Optional,
    OneOf { group: String },
}

impl From<Need> for SandboxResourceNeed {
    fn from(need: Need) -> Self {
        match need {
            Need::Required => Self::Required,
            Need::Optional => Self::Optional,
            Need::OneOf(group) => Self::OneOf { group: group.into() },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SandboxResourceRecord {
    pub id: String,
    pub provider: String,
    pub relative_path: String,
    pub need: SandboxResourceNeed,
    pub explanation: String,
}

#[uniffi::export]
pub fn sandbox_resource_descriptors() -> Vec<SandboxResourceRecord> {
    resources::all()
        .iter()
        .map(|resource| SandboxResourceRecord {
            id: resource.id.into(),
            provider: resource.provider.into(),
            relative_path: resource.relative_path.into(),
            need: resource.need.into(),
            explanation: resource.explanation.into(),
        })
        .collect()
}

#[uniffi::export]
pub fn configured_sandbox_resource_path(id: String, environment: HashMap<String, String>, home: String) -> String {
    resources::configured_path(
        resources::all()
            .iter()
            .find(|resource| resource.id == id)
            .expect("sandbox resource came from its descriptor"),
        &environment,
        Path::new(&home),
    )
    .to_string_lossy()
    .into_owned()
}

#[uniffi::export]
pub fn provider_needs_sandbox_resources(provider: String, credential_source: Option<String>) -> bool {
    resources::needs_access(&provider, credential_source.as_deref())
}

#[uniffi::export]
pub fn sandbox_resources_requiring_access(environment: HashMap<String, String>) -> Vec<String> {
    resources::required_ids(&environment)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[uniffi::export]
pub fn configured_sandbox_providers(
    configured: Vec<String>,
    intervals: Vec<String>,
    bookmarked: Vec<String>,
) -> Vec<String> {
    resources::configured_providers(&configured, &intervals, &bookmarked)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[uniffi::export]
pub fn missing_sandbox_resource_ids(provider: String, bookmarked: Vec<String>) -> Vec<String> {
    resources::missing_ids(&provider, &bookmarked)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/setup/resources.rs"]
mod tests;
