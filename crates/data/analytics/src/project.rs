//! Normalizes and hides local project identities.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn anonymous(identity: &str) -> String {
    format!("Project {}", hex::encode(&Sha256::digest(identity.as_bytes())[..6]))
}

pub fn export_label(identity: &str, metric: &str, hide_identity: bool) -> String {
    if hide_identity && matches!(metric, "projectCost" | "projectMessages") {
        anonymous(identity)
    } else {
        identity.to_owned()
    }
}

pub fn labels(identities: &[String], hide_identity: bool) -> BTreeMap<String, String> {
    let unique: BTreeSet<_> = identities.iter().collect();
    if hide_identity {
        return unique
            .into_iter()
            .map(|identity| (identity.clone(), anonymous(identity)))
            .collect();
    }

    let mut groups: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for identity in unique {
        groups
            .entry(identity.rsplit('/').find(|part| !part.is_empty()).unwrap_or(identity))
            .or_default()
            .push(identity);
    }
    let mut result = BTreeMap::new();
    for (basename, paths) in groups {
        if paths.len() == 1 {
            result.insert(paths[0].clone(), basename.to_owned());
            continue;
        }
        let mut counts = BTreeMap::new();
        for path in &paths {
            let parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
            for depth in 2..=parts.len().max(2) {
                *counts
                    .entry(parts[parts.len().saturating_sub(depth)..].join("/"))
                    .or_insert(0_usize) += 1;
            }
        }
        for path in paths {
            let parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
            let label = (2..=parts.len().max(2))
                .map(|depth| parts[parts.len().saturating_sub(depth)..].join("/"))
                .find(|suffix| counts[suffix] == 1)
                .unwrap_or_else(|| path.clone());
            result.insert(path.clone(), label);
        }
    }
    result
}

#[cfg(test)]
#[path = "../tests/unit/project/tests.rs"]
mod tests;
