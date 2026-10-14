//! Verifies macOS UniFFI mappings against shared policy.

use super::*;

#[test]
fn account_fingerprint_matches_core_identity() {
    assert_eq!(
        account_fingerprint("USER@example.com".into(), String::new()),
        "398dac8df0ca8c21b6c559ac611983faa10d668ae2a0b024e39545928918c01a"
    );
}

#[test]
fn project_identity_bridge_matches_shared_rules() {
    assert_eq!(anonymous_project("/work/repo".into()), "Project ddc5e473a09b");
    assert_eq!(
        project_labels(vec!["/a/repo".into(), "/b/repo".into()], false),
        std::collections::HashMap::from([("/a/repo".into(), "a/repo".into()), ("/b/repo".into(), "b/repo".into()),])
    );
}

#[test]
fn primary_action_bridge_covers_every_availability_and_recovery_issue() {
    for (availability, expected) in [
        (Availability::Loading, ProviderAction::Refresh),
        (Availability::Current, ProviderAction::Refresh),
        (Availability::Stale, ProviderAction::Refresh),
        (Availability::AuthenticationRequired, ProviderAction::SignIn),
        (Availability::NetworkUnavailable, ProviderAction::Retry),
        (Availability::RateLimited, ProviderAction::Retry),
        (Availability::Unavailable, ProviderAction::Retry),
        (Availability::Disabled, ProviderAction::Setup),
    ] {
        assert_eq!(
            provider_primary_action(availability, None),
            expected,
            "{availability:?}"
        );
    }
    for issue in [
        RecoveryIssue::CredentialMissing,
        RecoveryIssue::CredentialExpired,
        RecoveryIssue::CredentialUnreadable,
        RecoveryIssue::ResourceAccess,
        RecoveryIssue::Network,
        RecoveryIssue::RateLimited,
        RecoveryIssue::Service,
        RecoveryIssue::AccountUnsupported,
        RecoveryIssue::CredentialPersistence,
    ] {
        let expected = if matches!(
            issue,
            RecoveryIssue::CredentialUnreadable | RecoveryIssue::ResourceAccess | RecoveryIssue::AccountUnsupported
        ) {
            ProviderAction::Setup
        } else {
            ProviderAction::SignIn
        };
        assert_eq!(
            provider_primary_action(Availability::AuthenticationRequired, Some(issue)),
            expected
        );
    }
}

#[test]
fn deadline_updates_match_core_policy() {
    for (name, kind, target, expected) in [
        ("age", DeadlineKind::Age, Some(970.0), Some(1030.0)),
        ("reset", DeadlineKind::Reset, Some(1030.0), Some(1030.0)),
        ("missing", DeadlineKind::Reset, None, None),
    ] {
        assert_eq!(next_deadline_update(kind, target, 1000.0), expected, "{name}");
    }
}

#[test]
fn rate_limit_backoff_bridges_to_core() {
    assert_eq!(rate_limit_backoff(2, Some(900.0), 60.0), 900.0);
}

#[test]
fn refresh_limits_match_shared_policy() {
    assert_eq!(
        refresh_limits(),
        RefreshLimits {
            rate_limit_backoff: 300.0,
            minimum_backoff: 60.0,
            maximum_backoff: 1800.0,
            network_backoff: 60.0,
            idle_threshold: 3600.0,
        }
    );
}
