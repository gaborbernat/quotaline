//! Maps shared setup observation decisions to UniFFI records.

use quotaline_refresh as refresh;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SetupObservationInput {
    pub credential_checked: bool,
    pub credential_changed: bool,
    pub credential_usable: bool,
    pub resources_changed: bool,
    pub resource_issue: bool,
    pub authentication_required: bool,
    pub has_snapshot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SetupRestoredAvailability {
    Loading,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SetupObservationResult {
    pub apply_credential: bool,
    pub clear_issue: bool,
    pub restored_availability: Option<SetupRestoredAvailability>,
}

#[uniffi::export]
pub fn setup_observation(input: SetupObservationInput) -> SetupObservationResult {
    let decision = refresh::setup_observation(refresh::SetupObservationInput {
        credential_checked: input.credential_checked,
        credential_changed: input.credential_changed,
        credential_usable: input.credential_usable,
        resources_changed: input.resources_changed,
        resource_issue: input.resource_issue,
        authentication_required: input.authentication_required,
        has_snapshot: input.has_snapshot,
    });
    SetupObservationResult {
        apply_credential: decision.apply_credential,
        clear_issue: decision.clear_issue,
        restored_availability: decision.restored_availability.map(|availability| match availability {
            refresh::RestoredAvailability::Loading => SetupRestoredAvailability::Loading,
            refresh::RestoredAvailability::Stale => SetupRestoredAvailability::Stale,
        }),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/refresh/setup_observation/tests.rs"]
mod tests;
