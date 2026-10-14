//! Chooses the primary action for each provider state.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum ProviderAction {
    Refresh,
    Retry,
    SignIn,
    Setup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum Availability {
    Loading,
    Current,
    Stale,
    AuthenticationRequired,
    NetworkUnavailable,
    RateLimited,
    Unavailable,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum RecoveryIssue {
    CredentialMissing,
    CredentialExpired,
    CredentialUnreadable,
    ResourceAccess,
    Network,
    RateLimited,
    Service,
    AccountUnsupported,
    CredentialPersistence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginCommand {
    Claude,
    Codex,
    Cursor,
    Copilot,
}

pub fn login_command(provider: &str, source: Option<&str>) -> Option<LoginCommand> {
    match provider {
        "claude" => Some(LoginCommand::Claude),
        "codex" => Some(LoginCommand::Codex),
        "cursor" if source == Some("cursor.agent") => Some(LoginCommand::Cursor),
        "copilot" if matches!(source, Some("copilot.keychain" | "copilot.file")) => Some(LoginCommand::Copilot),
        _ => None,
    }
}

pub fn primary(availability: Availability, issue: Option<RecoveryIssue>) -> ProviderAction {
    use Availability as A;
    use ProviderAction as P;
    use RecoveryIssue as R;

    match availability {
        A::AuthenticationRequired | A::Unavailable
            if matches!(
                issue,
                Some(R::CredentialUnreadable | R::ResourceAccess | R::AccountUnsupported)
            ) =>
        {
            P::Setup
        }
        A::AuthenticationRequired => P::SignIn,
        A::NetworkUnavailable | A::RateLimited | A::Unavailable => P::Retry,
        A::Disabled => P::Setup,
        A::Current | A::Stale | A::Loading => P::Refresh,
    }
}

pub fn availability_from_service_status(status: &str, recovery: Option<&str>) -> Availability {
    match recovery.unwrap_or(status) {
        "connected" => Availability::Current,
        "stale" => Availability::Stale,
        "checking" => Availability::Loading,
        "sign-in" => Availability::AuthenticationRequired,
        "offline" => Availability::NetworkUnavailable,
        "rate-limited" => Availability::RateLimited,
        "disabled" => Availability::Disabled,
        _ => Availability::Unavailable,
    }
}

pub fn from_service_status(status: &str, recovery: Option<&str>) -> ProviderAction {
    let effective = recovery.unwrap_or(status);
    let issue = match effective {
        "unreadable" => Some(RecoveryIssue::CredentialUnreadable),
        "unsupported" => Some(RecoveryIssue::AccountUnsupported),
        _ => None,
    };
    primary(availability_from_service_status(status, recovery), issue)
}

#[cfg(test)]
#[path = "../tests/unit/action/tests.rs"]
mod tests;
