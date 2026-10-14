//! Filters history windows by provider and the user's selection.

use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeKey {
    pub provider: String,
    pub storage_key: String,
}

pub fn scoped_indices(keys: &[ScopeKey], active_providers: &[String], selected_keys: Option<&[String]>) -> Vec<usize> {
    let active: HashSet<&str> = active_providers.iter().map(String::as_str).collect();
    let selected = selected_keys.map(|keys| keys.iter().map(String::as_str).collect::<HashSet<_>>());
    keys.iter()
        .enumerate()
        .filter_map(|(index, key)| {
            (active.contains(key.provider.as_str())
                && selected
                    .as_ref()
                    .is_none_or(|keys| keys.contains(key.storage_key.as_str())))
            .then_some(index)
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/scope/tests.rs"]
mod tests;
