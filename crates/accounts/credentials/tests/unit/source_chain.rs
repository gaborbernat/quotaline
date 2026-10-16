use super::*;

#[test]
fn first_available_stops_on_first_present_and_preserves_first_error() {
    let states = [SourceState::Failed, SourceState::Present, SourceState::Failed];
    let mut reads = Vec::new();
    assert_eq!(
        first_available(states.len(), |index| {
            reads.push(index);
            states[index]
        }),
        SourceSelection {
            present: Some(1),
            error: Some(0)
        }
    );
    assert_eq!(reads, [0, 1]);
}

#[test]
fn first_available_reports_first_failure_without_a_credential() {
    for (states, expected) in [
        (
            vec![SourceState::Missing, SourceState::Failed, SourceState::Failed],
            SourceSelection {
                present: None,
                error: Some(1),
            },
        ),
        (
            vec![SourceState::Missing],
            SourceSelection {
                present: None,
                error: None,
            },
        ),
    ] {
        assert_eq!(first_available(states.len(), |index| states[index]), expected);
    }
}

#[test]
fn save_target_uses_existing_or_first_source() {
    for (states, expected) in [
        (vec![], None),
        (vec![SourceState::Failed], Some(0)),
        (vec![SourceState::Missing, SourceState::Present], Some(1)),
    ] {
        assert_eq!(save_target(states.len(), |index| states[index]), expected);
    }
}

#[test]
fn last_error_keeps_first_present_and_last_failure() {
    let states = [
        SourceState::Failed,
        SourceState::Present,
        SourceState::Present,
        SourceState::Failed,
    ];
    assert_eq!(
        last_error(states.len(), |index| states[index]),
        SourceSelection {
            present: Some(1),
            error: Some(3)
        }
    );
}

#[test]
fn last_error_accepts_only_missing_sources() {
    assert_eq!(
        last_error(1, |_| SourceState::Missing),
        SourceSelection {
            present: None,
            error: None
        }
    );
}
