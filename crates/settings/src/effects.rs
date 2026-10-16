//! Describes follow-up work required by a settings change.

use crate::Settings;
use quotaline_catalog::IDS;

#[derive(Debug, PartialEq, Eq)]
pub struct SettingsUpdateEffects {
    pub prune_days: Option<u32>,
    pub refresh_all: bool,
    pub newly_enabled: Vec<&'static str>,
    pub reset_demo: bool,
    pub sync_history_visibility: bool,
    pub reset_history_session: bool,
}

impl SettingsUpdateEffects {
    pub fn between(previous: &Settings, changed: &Settings, key: &str) -> Self {
        Self {
            prune_days: (key == "retention_days" && changed.retention_days < previous.retention_days)
                .then_some(changed.retention_days),
            refresh_all: key == "demo" || (key == "allow_token_refresh" && changed.allow_token_refresh),
            newly_enabled: if key == "providers" || key.starts_with("provider:") {
                IDS.into_iter()
                    .filter(|id| {
                        changed.providers.get(*id) == Some(&true)
                            && (previous.providers.get(*id) == Some(&false)
                                || key.strip_prefix("provider:") == Some(*id))
                    })
                    .collect()
            } else {
                Vec::new()
            },
            reset_demo: key == "demo",
            sync_history_visibility: key == "history_hidden_keys",
            reset_history_session: matches!(
                key,
                "history_days" | "history_metric" | "history_rollup" | "history_utc"
            ),
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/effects/tests.rs"]
mod tests;
