use super::*;
use quotaline_quota::Window;

#[test]
fn missing_unread_credential_is_checked_on_open_when_due() {
    let mut provider = Provider::empty("claude", "sign-in");
    provider.credential_health = CredentialHealth::Missing;
    let interval = Duration::from_secs(60);
    assert!(should_check_credentials(Some(&provider), true, None, interval));
    assert!(needs_credentials(Some(&provider)));
    assert!(should_check_credentials(
        Some(&provider),
        true,
        Some(interval),
        interval
    ));
    assert!(!should_check_credentials(
        Some(&provider),
        true,
        Some(Duration::from_secs(59)),
        interval,
    ));
}

#[test]
fn disabled_absent_or_read_provider_is_not_checked_on_open() {
    let mut provider = Provider::empty("claude", "sign-in");
    provider.credential_health = CredentialHealth::Missing;
    let interval = Duration::from_secs(60);
    assert!(!should_check_credentials(Some(&provider), false, None, interval));
    assert!(!should_check_credentials(None, true, None, interval));
    assert!(!needs_credentials(None));
    provider.credential_health = CredentialHealth::Valid;
    assert!(!needs_credentials(Some(&provider)));
    assert!(!should_check_credentials(Some(&provider), true, None, interval));
    provider.credential_health = CredentialHealth::Missing;
    provider.updated = Some(1);
    assert!(!should_check_credentials(Some(&provider), true, None, interval));
    provider.updated = None;
    provider
        .windows
        .push(Window::new("session", "Session", 0.0, None, "session"));
    assert!(!should_check_credentials(Some(&provider), true, None, interval));
}
