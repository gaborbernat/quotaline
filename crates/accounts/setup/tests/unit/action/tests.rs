//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn primary_actions_follow_availability() {
    use Availability as A;
    use ProviderAction as P;

    for (availability, expected) in [
        (A::Loading, P::Refresh),
        (A::Current, P::Refresh),
        (A::Stale, P::Refresh),
        (A::AuthenticationRequired, P::SignIn),
        (A::NetworkUnavailable, P::Retry),
        (A::RateLimited, P::Retry),
        (A::Unavailable, P::Retry),
        (A::Disabled, P::Setup),
    ] {
        assert_eq!(primary(availability, None), expected, "{availability:?}");
    }
}

#[test]
fn access_recovery_opens_setup_only_when_quota_is_unavailable() {
    use Availability as A;
    use ProviderAction as P;
    use RecoveryIssue as R;

    for issue in [R::CredentialUnreadable, R::ResourceAccess, R::AccountUnsupported] {
        assert_eq!(primary(A::AuthenticationRequired, Some(issue)), P::Setup);
        assert_eq!(primary(A::Unavailable, Some(issue)), P::Setup);
        assert_eq!(primary(A::Current, Some(issue)), P::Refresh);
    }
    for issue in [
        R::CredentialMissing,
        R::CredentialExpired,
        R::Network,
        R::RateLimited,
        R::Service,
        R::CredentialPersistence,
    ] {
        assert_eq!(primary(A::AuthenticationRequired, Some(issue)), P::SignIn);
        assert_eq!(primary(A::Unavailable, Some(issue)), P::Retry);
    }
}

#[test]
fn service_statuses_use_the_same_action_policy() {
    use ProviderAction as P;

    for (status, recovery, expected) in [
        ("connected", None, P::Refresh),
        ("stale", None, P::Refresh),
        ("checking", None, P::Refresh),
        ("sign-in", None, P::SignIn),
        ("offline", None, P::Retry),
        ("rate-limited", None, P::Retry),
        ("error", None, P::Retry),
        ("disabled", None, P::Setup),
        ("connected", Some("unreadable"), P::Setup),
        ("connected", Some("unsupported"), P::Setup),
        ("stale", Some("sign-in"), P::SignIn),
    ] {
        assert_eq!(from_service_status(status, recovery), expected, "{status} {recovery:?}");
    }
    assert_eq!(serde_json::to_string(&P::SignIn).unwrap(), "\"sign-in\"");
}

#[test]
fn login_command_follows_the_selected_credential_source() {
    use LoginCommand as L;

    for (provider, source, expected) in [
        ("claude", None, Some(L::Claude)),
        ("codex", None, Some(L::Codex)),
        ("cursor", Some("cursor.agent"), Some(L::Cursor)),
        ("copilot", Some("copilot.keychain"), Some(L::Copilot)),
        ("copilot", Some("copilot.file"), Some(L::Copilot)),
        ("cursor", Some("cursor.app"), None),
        ("copilot", Some("copilot.environment"), None),
        ("gemini", None, None),
        ("antigravity", None, None),
    ] {
        assert_eq!(login_command(provider, source), expected, "{provider} {source:?}");
    }
}
