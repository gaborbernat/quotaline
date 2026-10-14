//! Chooses provider state changes after credential and resource discovery.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetupObservationInput {
    pub credential_checked: bool,
    pub credential_changed: bool,
    pub credential_usable: bool,
    pub resources_changed: bool,
    pub resource_issue: bool,
    pub authentication_required: bool,
    pub has_snapshot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoredAvailability {
    Loading,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetupObservation {
    pub apply_credential: bool,
    pub clear_issue: bool,
    pub restored_availability: Option<RestoredAvailability>,
}

pub fn setup_observation(input: SetupObservationInput) -> SetupObservation {
    let credential_changed = input.credential_checked && input.credential_changed;
    let restored_availability = (credential_changed && input.credential_usable && input.authentication_required)
        .then_some(if input.has_snapshot {
            RestoredAvailability::Stale
        } else {
            RestoredAvailability::Loading
        });
    SetupObservation {
        apply_credential: input.credential_checked,
        clear_issue: credential_changed || (input.resources_changed && input.resource_issue),
        restored_availability,
    }
}

#[cfg(test)]
#[path = "../tests/unit/setup_observation/tests.rs"]
mod tests;
