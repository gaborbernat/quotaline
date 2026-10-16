use super::{
    CredentialCondition, IssueAction, IssueKind, RecoveryIssueDecision, ServiceHealthKind, SetupInput, SetupIssue,
    SetupResource, issue, recovery_issue as core_recovery_issue, service_health,
};
use crate::access::ResourceAccess;
use crate::action::Availability;

fn input(credential: CredentialCondition) -> SetupInput {
    SetupInput {
        sign_in_command: Some("codex login".into()),
        credential,
        resources: vec![],
    }
}

fn recovery_issue(
    input: &SetupInput,
    setup_credential: CredentialCondition,
    availability: Availability,
    detail: Option<&str>,
) -> Option<SetupIssue> {
    match core_recovery_issue(input, setup_credential, false, availability, detail) {
        RecoveryIssueDecision::Issue(issue) => Some(issue),
        RecoveryIssueDecision::None | RecoveryIssueDecision::Setup => None,
    }
}

#[test]
fn recovery_issue_selects_existing_setup_issue_when_no_new_issue_applies() {
    assert_eq!(
        core_recovery_issue(
            &input(CredentialCondition::Valid),
            CredentialCondition::Valid,
            true,
            Availability::AuthenticationRequired,
            None,
        ),
        RecoveryIssueDecision::Setup
    );
    assert_eq!(
        core_recovery_issue(
            &input(CredentialCondition::Unchecked),
            CredentialCondition::Unchecked,
            true,
            Availability::Loading,
            None,
        ),
        RecoveryIssueDecision::Setup
    );
}

#[test]
fn recovery_issue_maps_service_availability() {
    for (availability, kind) in [
        (Availability::NetworkUnavailable, IssueKind::Network),
        (Availability::RateLimited, IssueKind::RateLimited),
        (Availability::Unavailable, IssueKind::Service),
    ] {
        let result = recovery_issue(
            &input(CredentialCondition::Valid),
            CredentialCondition::Unchecked,
            availability,
            None,
        )
        .unwrap();
        assert_eq!((result.kind, result.detail), (kind, None));
        assert_eq!(result.action, IssueAction::RefreshProvider);
        assert_eq!(
            recovery_issue(
                &input(CredentialCondition::Valid),
                CredentialCondition::Unchecked,
                availability,
                Some("provider detail")
            )
            .unwrap()
            .detail,
            Some("provider detail".into())
        );
    }
}

#[test]
fn recovery_issue_uses_sign_in_and_resource_repair() {
    let signed_out = recovery_issue(
        &input(CredentialCondition::Missing(None)),
        CredentialCondition::Unchecked,
        Availability::AuthenticationRequired,
        None,
    )
    .unwrap();
    assert_eq!(signed_out.kind, IssueKind::CredentialMissing);
    let default = recovery_issue(
        &input(CredentialCondition::Valid),
        CredentialCondition::Unchecked,
        Availability::AuthenticationRequired,
        None,
    )
    .unwrap();
    assert_eq!(default.action, IssueAction::CopyCommand("codex login".into()));
    let mut with_resource = input(CredentialCondition::Missing(None));
    with_resource.resources.push(resource(true, false));
    let blocked = recovery_issue(
        &with_resource,
        CredentialCondition::Unchecked,
        Availability::AuthenticationRequired,
        None,
    )
    .unwrap();
    assert_eq!(blocked.kind, IssueKind::ResourceAccess);
    assert_eq!(blocked.action, IssueAction::GrantAccess(0));
}

#[test]
fn recovery_issue_uses_setup_health_while_current_health_is_unchecked() {
    let missing = recovery_issue(
        &input(CredentialCondition::Unchecked),
        CredentialCondition::Missing(Some("auth missing".into())),
        Availability::Loading,
        None,
    )
    .unwrap();
    assert_eq!(missing.detail.as_deref(), Some("auth missing"));
    assert_eq!(
        recovery_issue(
            &input(CredentialCondition::Valid),
            CredentialCondition::Missing(None),
            Availability::Loading,
            None
        ),
        None
    );
    assert_eq!(
        recovery_issue(
            &input(CredentialCondition::Unchecked),
            CredentialCondition::Unchecked,
            Availability::Loading,
            None
        ),
        None
    );
}

#[test]
fn recovery_issue_hides_repair_when_data_is_current_stale_or_disabled() {
    for availability in [Availability::Current, Availability::Stale, Availability::Disabled] {
        assert_eq!(
            recovery_issue(
                &input(CredentialCondition::Missing(None)),
                CredentialCondition::Unchecked,
                availability,
                None
            ),
            None
        );
    }
}

fn resource(required: bool, granted: bool) -> SetupResource {
    SetupResource {
        access: ResourceAccess {
            alternatives_group: None,
            required,
            granted,
        },
    }
}

#[test]
fn credential_conditions_choose_the_expected_recovery_issue() {
    for (case, credential, expected) in [
        ("unchecked", CredentialCondition::Unchecked, None),
        ("valid", CredentialCondition::Valid, None),
        (
            "missing",
            CredentialCondition::Missing(None),
            Some(SetupIssue {
                kind: IssueKind::CredentialMissing,
                detail: None,
                action: IssueAction::CopyCommand("codex login".into()),
            }),
        ),
        (
            "missing with detail",
            CredentialCondition::Missing(Some("auth.json vanished".into())),
            Some(SetupIssue {
                kind: IssueKind::CredentialMissing,
                detail: Some("auth.json vanished".into()),
                action: IssueAction::CopyCommand("codex login".into()),
            }),
        ),
        (
            "expired",
            CredentialCondition::Expired,
            Some(SetupIssue {
                kind: IssueKind::CredentialExpired,
                detail: None,
                action: IssueAction::CopyCommand("codex login".into()),
            }),
        ),
        (
            "unreadable",
            CredentialCondition::Unreadable("invalid JSON".into()),
            Some(SetupIssue {
                kind: IssueKind::CredentialUnreadable,
                detail: Some("invalid JSON".into()),
                action: IssueAction::RefreshProvider,
            }),
        ),
    ] {
        assert_eq!(issue(&input(credential)), expected, "{case}");
    }
}

#[test]
fn missing_command_uses_provider_refresh() {
    let mut state = input(CredentialCondition::Missing(None));
    state.sign_in_command = None;
    assert_eq!(issue(&state).unwrap().action, IssueAction::RefreshProvider);
}

#[test]
fn missing_resource_overrides_credential_issue_unless_credential_is_valid() {
    for (case, credential) in [
        ("missing", CredentialCondition::Missing(None)),
        ("expired", CredentialCondition::Expired),
        ("unchecked", CredentialCondition::Unchecked),
    ] {
        let mut state = input(credential);
        state.resources = vec![resource(false, false), resource(true, false)];
        assert_eq!(
            issue(&state),
            Some(SetupIssue {
                kind: IssueKind::ResourceAccess,
                detail: None,
                action: IssueAction::GrantAccess(1),
            }),
            "{case}"
        );
    }
    let mut valid = input(CredentialCondition::Valid);
    valid.resources = vec![resource(true, false)];
    assert_eq!(issue(&valid), None);
}

#[test]
fn granted_resource_leaves_credential_issue() {
    let mut state = input(CredentialCondition::Expired);
    state.resources = vec![resource(true, true)];
    assert_eq!(issue(&state).unwrap().kind, IssueKind::CredentialExpired);
}

#[test]
fn service_health_maps_availability() {
    for (availability, kind) in [
        (Availability::Loading, ServiceHealthKind::Checking),
        (Availability::Current, ServiceHealthKind::Available),
        (Availability::Stale, ServiceHealthKind::Available),
        (Availability::NetworkUnavailable, ServiceHealthKind::Offline),
        (Availability::RateLimited, ServiceHealthKind::RateLimited),
        (Availability::AuthenticationRequired, ServiceHealthKind::Unavailable),
        (Availability::Unavailable, ServiceHealthKind::Unavailable),
        (Availability::Disabled, ServiceHealthKind::Unchecked),
    ] {
        assert_eq!(service_health(availability), kind);
    }
}
