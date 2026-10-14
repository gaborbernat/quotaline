use super::*;

fn input() -> SetupObservationInput {
    SetupObservationInput {
        credential_checked: true,
        credential_changed: true,
        credential_usable: true,
        resources_changed: false,
        resource_issue: false,
        authentication_required: true,
        has_snapshot: false,
    }
}

#[test]
fn bridge_restores_loading_and_stale_availability() {
    assert_eq!(
        setup_observation(input()),
        SetupObservationResult {
            apply_credential: true,
            clear_issue: true,
            restored_availability: Some(SetupRestoredAvailability::Loading),
        }
    );
    assert_eq!(
        setup_observation(SetupObservationInput {
            has_snapshot: true,
            ..input()
        })
        .restored_availability,
        Some(SetupRestoredAvailability::Stale)
    );
}

#[test]
fn bridge_keeps_unchecked_credentials_and_clears_resource_issue() {
    assert_eq!(
        setup_observation(SetupObservationInput {
            credential_checked: false,
            resources_changed: true,
            resource_issue: true,
            ..input()
        }),
        SetupObservationResult {
            apply_credential: false,
            clear_issue: true,
            restored_availability: None,
        }
    );
}
