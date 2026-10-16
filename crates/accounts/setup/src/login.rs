//! Describes provider sign-in commands and setup actions.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Opening,
    AwaitingReturn,
}

#[derive(Default)]
pub struct LoginFlow {
    phases: BTreeMap<String, Phase>,
}

impl LoginFlow {
    pub fn begin(&mut self, provider: &str) -> bool {
        if self.phases.contains_key(provider) {
            return false;
        }
        self.phases.insert(provider.to_owned(), Phase::Opening);
        true
    }

    pub fn opened(&mut self, provider: &str) {
        self.phases.insert(provider.to_owned(), Phase::AwaitingReturn);
    }

    pub fn failed(&mut self, provider: &str) {
        self.phases.remove(provider);
    }

    pub fn take_returned(&mut self) -> Vec<String> {
        let returned: Vec<_> = self
            .phases
            .iter()
            .filter_map(|(provider, phase)| (*phase == Phase::AwaitingReturn).then_some(provider.clone()))
            .collect();
        self.phases.retain(|_, phase| *phase == Phase::Opening);
        returned
    }
}

#[cfg(test)]
#[path = "../tests/unit/login/tests.rs"]
mod tests;
