//! Chooses provider setup recovery issues from credential and resource state.

use crate::access::{ResourceAccess, first_missing_resource};
use crate::action::Availability;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum ServiceHealthKind {
    Unchecked,
    Checking,
    Available,
    Offline,
    RateLimited,
    Unavailable,
}

pub fn service_health(availability: Availability) -> ServiceHealthKind {
    use ServiceHealthKind as Kind;

    match availability {
        Availability::Loading => Kind::Checking,
        Availability::Current | Availability::Stale => Kind::Available,
        Availability::NetworkUnavailable => Kind::Offline,
        Availability::RateLimited => Kind::RateLimited,
        Availability::AuthenticationRequired | Availability::Unavailable => Kind::Unavailable,
        Availability::Disabled => Kind::Unchecked,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialCondition {
    Unchecked,
    Missing(Option<String>),
    Valid,
    Expired,
    Unreadable(String),
}

#[derive(Clone, Debug)]
pub struct SetupResource {
    pub access: ResourceAccess,
}

#[derive(Clone, Debug)]
pub struct SetupInput {
    pub sign_in_command: Option<String>,
    pub credential: CredentialCondition,
    pub resources: Vec<SetupResource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum IssueKind {
    CredentialMissing,
    CredentialExpired,
    CredentialUnreadable,
    ResourceAccess,
    Network,
    RateLimited,
    Service,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IssueAction {
    CopyCommand(String),
    RefreshProvider,
    GrantAccess(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupIssue {
    pub kind: IssueKind,
    pub detail: Option<String>,
    pub action: IssueAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryIssueDecision {
    None,
    Setup,
    Issue(SetupIssue),
}

pub fn issue(input: &SetupInput) -> Option<SetupIssue> {
    let action = || {
        input
            .sign_in_command
            .as_ref()
            .map(|command| IssueAction::CopyCommand(command.clone()))
            .unwrap_or(IssueAction::RefreshProvider)
    };
    let credential_issue = match &input.credential {
        CredentialCondition::Unchecked | CredentialCondition::Valid => None,
        CredentialCondition::Missing(detail) => Some(SetupIssue {
            kind: IssueKind::CredentialMissing,
            detail: detail.clone(),
            action: action(),
        }),
        CredentialCondition::Expired => Some(SetupIssue {
            kind: IssueKind::CredentialExpired,
            detail: None,
            action: action(),
        }),
        CredentialCondition::Unreadable(detail) => Some(SetupIssue {
            kind: IssueKind::CredentialUnreadable,
            detail: Some(detail.clone()),
            action: IssueAction::RefreshProvider,
        }),
    };
    if input.credential == CredentialCondition::Valid {
        return credential_issue;
    }
    resource_issue(input).or(credential_issue)
}

fn resource_issue(input: &SetupInput) -> Option<SetupIssue> {
    let access = input
        .resources
        .iter()
        .map(|resource| resource.access.clone())
        .collect::<Vec<_>>();
    first_missing_resource(&access).map(|index| SetupIssue {
        kind: IssueKind::ResourceAccess,
        detail: None,
        action: IssueAction::GrantAccess(index),
    })
}

pub fn recovery_issue(
    input: &SetupInput,
    setup_credential: CredentialCondition,
    has_setup_issue: bool,
    availability: Availability,
    detail: Option<&str>,
) -> RecoveryIssueDecision {
    use RecoveryIssueDecision as Decision;

    let mut input = input.clone();
    if input.credential == CredentialCondition::Unchecked {
        input.credential = setup_credential;
    }
    let retry = |kind, detail: Option<&str>| SetupIssue {
        kind,
        detail: detail.map(str::to_owned),
        action: IssueAction::RefreshProvider,
    };
    match availability {
        Availability::Disabled | Availability::Current | Availability::Stale => Decision::None,
        Availability::AuthenticationRequired => {
            if let Some(repair) = resource_issue(&input).or_else(|| issue(&input)) {
                Decision::Issue(repair)
            } else if has_setup_issue {
                Decision::Setup
            } else {
                Decision::Issue(SetupIssue {
                    kind: IssueKind::CredentialMissing,
                    detail: None,
                    action: input
                        .sign_in_command
                        .as_ref()
                        .map_or(IssueAction::RefreshProvider, |command| {
                            IssueAction::CopyCommand(command.clone())
                        }),
                })
            }
        }
        Availability::NetworkUnavailable => Decision::Issue(retry(IssueKind::Network, detail)),
        Availability::RateLimited => Decision::Issue(retry(IssueKind::RateLimited, detail)),
        Availability::Unavailable => Decision::Issue(retry(IssueKind::Service, detail)),
        Availability::Loading if input.credential == CredentialCondition::Valid => Decision::None,
        Availability::Loading => issue(&input).map(Decision::Issue).unwrap_or(if has_setup_issue {
            Decision::Setup
        } else {
            Decision::None
        }),
    }
}

pub mod access;
pub mod action;
pub mod login;

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
