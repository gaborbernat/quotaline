//! Describes macOS credential sources and sign-in instructions.

mod resources;
pub use resources::*;

use crate::access::ResourceAccessRecord;
use quotaline_setup as setup;
use quotaline_setup::CredentialCondition;
use quotaline_setup::IssueAction;
use quotaline_setup::IssueKind;
use quotaline_setup::RecoveryIssueDecision;
use quotaline_setup::ServiceHealthKind;
use quotaline_setup::SetupInput;
use quotaline_setup::SetupResource;
use quotaline_setup::action::Availability;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ServiceHealthRecord {
    pub kind: ServiceHealthKind,
}

#[uniffi::export]
pub fn provider_service_health(availability: Availability) -> ServiceHealthRecord {
    ServiceHealthRecord {
        kind: setup::service_health(availability),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProviderSetupRecord {
    pub sign_in_command: String,
    pub credential_sources: Vec<String>,
}

#[uniffi::export]
pub fn provider_setup_metadata(id: String) -> Option<ProviderSetupRecord> {
    let setup = quotaline_macos_platform::setup::metadata(&id)?;
    Some(ProviderSetupRecord {
        sign_in_command: setup.sign_in_command.into(),
        credential_sources: setup.credential_sources.iter().map(|source| (*source).into()).collect(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SetupCredentialRecord {
    Unchecked,
    Missing { detail: Option<String> },
    Valid,
    Expired,
    Unreadable { detail: String },
}

impl From<SetupCredentialRecord> for CredentialCondition {
    fn from(value: SetupCredentialRecord) -> Self {
        match value {
            SetupCredentialRecord::Unchecked => Self::Unchecked,
            SetupCredentialRecord::Missing { detail } => Self::Missing(detail),
            SetupCredentialRecord::Valid => Self::Valid,
            SetupCredentialRecord::Expired => Self::Expired,
            SetupCredentialRecord::Unreadable { detail } => Self::Unreadable(detail),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SetupResourceRecord {
    pub access: ResourceAccessRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SetupIssueInputRecord {
    pub sign_in_command: Option<String>,
    pub credential: SetupCredentialRecord,
    pub resources: Vec<SetupResourceRecord>,
}

impl From<SetupIssueInputRecord> for SetupInput {
    fn from(input: SetupIssueInputRecord) -> Self {
        Self {
            sign_in_command: input.sign_in_command,
            credential: input.credential.into(),
            resources: input
                .resources
                .into_iter()
                .map(|resource| SetupResource {
                    access: resource.access.into(),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SetupIssueActionRecord {
    CopyCommand { command: String },
    RefreshProvider,
    GrantAccess { index: u32 },
}

impl From<IssueAction> for SetupIssueActionRecord {
    fn from(value: IssueAction) -> Self {
        match value {
            IssueAction::CopyCommand(command) => Self::CopyCommand { command },
            IssueAction::RefreshProvider => Self::RefreshProvider,
            IssueAction::GrantAccess(index) => Self::GrantAccess { index: index as u32 },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SetupIssueRecord {
    pub kind: IssueKind,
    pub detail: Option<String>,
    pub action: SetupIssueActionRecord,
}

impl From<setup::SetupIssue> for SetupIssueRecord {
    fn from(issue: setup::SetupIssue) -> Self {
        Self {
            kind: issue.kind,
            detail: issue.detail,
            action: issue.action.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RecoveryIssueDecisionRecord {
    None,
    Setup,
    Issue { issue: SetupIssueRecord },
}

impl From<RecoveryIssueDecision> for RecoveryIssueDecisionRecord {
    fn from(decision: RecoveryIssueDecision) -> Self {
        match decision {
            RecoveryIssueDecision::None => Self::None,
            RecoveryIssueDecision::Setup => Self::Setup,
            RecoveryIssueDecision::Issue(issue) => Self::Issue { issue: issue.into() },
        }
    }
}

#[uniffi::export]
pub fn provider_setup_issue(input: SetupIssueInputRecord) -> Option<SetupIssueRecord> {
    setup::issue(&input.into()).map(Into::into)
}

#[uniffi::export]
pub fn provider_recovery_issue(
    input: SetupIssueInputRecord,
    setup_credential: SetupCredentialRecord,
    has_setup_issue: bool,
    availability: Availability,
    detail: Option<String>,
) -> RecoveryIssueDecisionRecord {
    setup::recovery_issue(
        &input.into(),
        setup_credential.into(),
        has_setup_issue,
        availability,
        detail.as_deref(),
    )
    .into()
}

#[cfg(test)]
#[path = "../tests/unit/setup/tests.rs"]
mod tests;
