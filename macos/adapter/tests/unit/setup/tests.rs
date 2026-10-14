use super::{
    IssueKind, RecoveryIssueDecisionRecord, ServiceHealthKind, ServiceHealthRecord, SetupCredentialRecord,
    SetupIssueActionRecord, SetupIssueInputRecord, SetupIssueRecord, SetupResourceRecord, provider_recovery_issue,
    provider_service_health, provider_setup_issue, provider_setup_metadata,
};
use crate::access::ResourceAccessRecord;
use quotaline_setup::action::Availability;

fn input(credential: SetupCredentialRecord) -> SetupIssueInputRecord {
    SetupIssueInputRecord {
        sign_in_command: Some("codex login".into()),
        credential,
        resources: vec![],
    }
}

#[test]
fn catalog_has_each_supported_provider_and_its_credential_sources() {
    for (id, command, sources) in [
        (
            "claude",
            "claude auth login",
            ["claude.keychain", "claude.file"].as_slice(),
        ),
        ("codex", "codex login", ["codex.keyring", "codex.file"].as_slice()),
        ("gemini", "gemini", ["gemini.keychain", "gemini.file"].as_slice()),
        ("antigravity", "agy", ["antigravity.keychain"].as_slice()),
        (
            "cursor",
            "cursor-agent login",
            ["cursor.app", "cursor.agent"].as_slice(),
        ),
        (
            "copilot",
            "copilot login",
            [
                "copilot.environment",
                "copilot.keychain",
                "copilot.file",
                "copilot.legacy-file",
            ]
            .as_slice(),
        ),
    ] {
        let metadata = provider_setup_metadata(id.into()).unwrap();
        assert_eq!(metadata.sign_in_command, command, "{id}");
        assert_eq!(
            metadata
                .credential_sources
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            sources,
            "{id}"
        );
    }
}

#[test]
fn unknown_provider_has_no_catalog_entry() {
    assert_eq!(provider_setup_metadata("unknown".into()), None);
}

#[test]
fn bridge_maps_credential_conditions_to_issues() {
    for (case, credential, expected) in [
        ("unchecked", SetupCredentialRecord::Unchecked, None),
        ("valid", SetupCredentialRecord::Valid, None),
        (
            "missing",
            SetupCredentialRecord::Missing { detail: None },
            Some(SetupIssueRecord {
                kind: IssueKind::CredentialMissing,
                detail: None,
                action: SetupIssueActionRecord::CopyCommand {
                    command: "codex login".into(),
                },
            }),
        ),
        (
            "expired",
            SetupCredentialRecord::Expired,
            Some(SetupIssueRecord {
                kind: IssueKind::CredentialExpired,
                detail: None,
                action: SetupIssueActionRecord::CopyCommand {
                    command: "codex login".into(),
                },
            }),
        ),
        (
            "unreadable",
            SetupCredentialRecord::Unreadable {
                detail: "invalid JSON".into(),
            },
            Some(SetupIssueRecord {
                kind: IssueKind::CredentialUnreadable,
                detail: Some("invalid JSON".into()),
                action: SetupIssueActionRecord::RefreshProvider,
            }),
        ),
    ] {
        assert_eq!(provider_setup_issue(input(credential)), expected, "{case}");
    }
}

#[test]
fn bridge_preserves_resource_index_and_recovery_action() {
    let mut state = input(SetupCredentialRecord::Missing {
        detail: Some("missing".into()),
    });
    state.resources = vec![
        SetupResourceRecord {
            access: ResourceAccessRecord {
                alternatives_group: None,
                required: false,
                granted: false,
            },
        },
        SetupResourceRecord {
            access: ResourceAccessRecord {
                alternatives_group: Some("codex".into()),
                required: true,
                granted: false,
            },
        },
    ];
    assert_eq!(
        provider_setup_issue(state),
        Some(SetupIssueRecord {
            kind: IssueKind::ResourceAccess,
            detail: None,
            action: SetupIssueActionRecord::GrantAccess { index: 1 },
        })
    );
}

#[test]
fn bridge_maps_each_service_health_state() {
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
        assert_eq!(provider_service_health(availability), ServiceHealthRecord { kind },);
    }
}

#[test]
fn recovery_bridge_maps_service_and_setup_states() {
    for (availability, kind) in [
        (Availability::NetworkUnavailable, IssueKind::Network),
        (Availability::RateLimited, IssueKind::RateLimited),
        (Availability::Unavailable, IssueKind::Service),
    ] {
        let decision = provider_recovery_issue(
            input(SetupCredentialRecord::Valid),
            SetupCredentialRecord::Unchecked,
            false,
            availability,
            None,
        );
        assert_eq!(
            decision,
            RecoveryIssueDecisionRecord::Issue {
                issue: SetupIssueRecord {
                    kind,
                    detail: None,
                    action: SetupIssueActionRecord::RefreshProvider,
                },
            }
        );
    }
    assert_eq!(
        provider_recovery_issue(
            input(SetupCredentialRecord::Unchecked),
            SetupCredentialRecord::Missing { detail: None },
            false,
            Availability::Loading,
            None,
        ),
        RecoveryIssueDecisionRecord::Issue {
            issue: SetupIssueRecord {
                kind: IssueKind::CredentialMissing,
                detail: None,
                action: SetupIssueActionRecord::CopyCommand {
                    command: "codex login".into()
                },
            }
        }
    );
    assert_eq!(
        provider_recovery_issue(
            input(SetupCredentialRecord::Valid),
            SetupCredentialRecord::Valid,
            true,
            Availability::AuthenticationRequired,
            None,
        ),
        RecoveryIssueDecisionRecord::Setup
    );
    assert_eq!(
        provider_recovery_issue(
            input(SetupCredentialRecord::Valid),
            SetupCredentialRecord::Valid,
            false,
            Availability::Current,
            None,
        ),
        RecoveryIssueDecisionRecord::None
    );
}
