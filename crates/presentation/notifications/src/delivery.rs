//! Suppresses repeated macOS notification deliveries across refreshes.

use std::collections::{BTreeMap, BTreeSet};

use super::Kind;

#[derive(Clone, Debug, PartialEq)]
pub enum CredentialKey {
    Missing(String),
    Expired(f64),
    Valid(Option<f64>),
}

pub struct DeliveryEvent {
    pub id: String,
    pub kind: Kind,
    pub window_id: Option<String>,
}

#[derive(Default)]
pub struct DeliveryLedger {
    sign_ins: BTreeMap<String, CredentialKey>,
    pace: BTreeMap<(String, String, Kind), String>,
}

impl DeliveryLedger {
    pub fn filter(
        &mut self,
        provider: &str,
        credential: Option<&CredentialKey>,
        active_windows: Option<&BTreeSet<String>>,
        events: &[DeliveryEvent],
    ) -> Vec<bool> {
        if let Some(windows) = active_windows {
            self.pace
                .retain(|(owner, window, _), _| owner != provider || windows.contains(window));
        }
        let authentication = events.iter().any(|event| event.kind == Kind::Authentication);
        let announce_sign_in = authentication && self.sign_ins.get(provider) != credential;
        if announce_sign_in {
            if let Some(credential) = credential {
                self.sign_ins.insert(provider.into(), credential.clone());
            } else {
                self.sign_ins.remove(provider);
            }
        }
        events
            .iter()
            .map(|event| {
                if event.kind == Kind::Authentication {
                    return announce_sign_in;
                }
                if matches!(event.kind, Kind::WillRunOut | Kind::AheadOfPace)
                    && let Some(window) = &event.window_id
                {
                    let key = (provider.into(), window.clone(), event.kind);
                    if self.pace.get(&key) == Some(&event.id) {
                        return false;
                    }
                    self.pace.insert(key, event.id.clone());
                }
                true
            })
            .collect()
    }

    pub fn clear_pace(&mut self) {
        self.pace.clear();
    }

    pub fn forget_pace(&mut self, provider: &str) {
        self.pace.retain(|(owner, _, _), _| owner != provider);
    }

    pub fn forget_sign_in(&mut self, provider: &str) {
        self.sign_ins.remove(provider);
    }

    pub fn retain_providers(&mut self, providers: &BTreeSet<String>) {
        self.sign_ins.retain(|provider, _| providers.contains(provider));
    }
}

#[cfg(test)]
#[path = "../tests/unit/delivery/tests.rs"]
mod tests;
