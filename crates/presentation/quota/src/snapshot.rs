//! Projects selected quotas and sign-in attention for compact snapshots.

use crate::Provider;
use std::collections::BTreeMap;

pub fn snapshot_attention(providers: &BTreeMap<String, Provider>, enabled: &BTreeMap<String, bool>) -> bool {
    providers.iter().any(|(id, provider)| {
        enabled.get(id) != Some(&false)
            && (matches!(provider.status.as_str(), "sign-in" | "auth")
                || provider.notice.starts_with("No sign-in found."))
    })
}

#[cfg(test)]
#[path = "../tests/unit/snapshot/tests.rs"]
mod tests;
