use super::*;

fn input() -> SetupObservationInput {
    SetupObservationInput {
        credential_checked: true,
        credential_changed: false,
        credential_usable: false,
        resources_changed: false,
        resource_issue: false,
        authentication_required: false,
        has_snapshot: false,
    }
}

#[test]
fn observes_checked_credentials() {
    assert_eq!(
        setup_observation(input()),
        SetupObservation {
            apply_credential: true,
            clear_issue: false,
            restored_availability: None,
        }
    );
    assert!(
        !setup_observation(SetupObservationInput {
            credential_checked: false,
            ..input()
        })
        .apply_credential
    );
}

#[test]
fn clears_changed_credentials_and_resolved_resource_issues() {
    for change in [
        SetupObservationInput {
            credential_changed: true,
            ..input()
        },
        SetupObservationInput {
            resources_changed: true,
            resource_issue: true,
            ..input()
        },
    ] {
        assert!(setup_observation(change).clear_issue);
    }
    for unchanged in [
        SetupObservationInput {
            credential_checked: false,
            credential_changed: true,
            ..input()
        },
        SetupObservationInput {
            resources_changed: true,
            ..input()
        },
        SetupObservationInput {
            resource_issue: true,
            ..input()
        },
    ] {
        assert!(!setup_observation(unchanged).clear_issue);
    }
}

#[test]
fn restores_authenticated_availability_from_new_credentials() {
    for (has_snapshot, expected) in [
        (false, RestoredAvailability::Loading),
        (true, RestoredAvailability::Stale),
    ] {
        let decision = setup_observation(SetupObservationInput {
            credential_changed: true,
            credential_usable: true,
            authentication_required: true,
            has_snapshot,
            ..input()
        });
        assert_eq!(decision.restored_availability, Some(expected));
    }
    for change in [
        SetupObservationInput {
            credential_changed: true,
            credential_usable: true,
            ..input()
        },
        SetupObservationInput {
            credential_changed: true,
            authentication_required: true,
            ..input()
        },
        SetupObservationInput {
            credential_usable: true,
            authentication_required: true,
            ..input()
        },
    ] {
        assert_eq!(setup_observation(change).restored_availability, None);
    }
}
