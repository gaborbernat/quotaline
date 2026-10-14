use super::*;

#[test]
fn service_label_describes_default_hashed_and_unexpected_services() {
    for (service, expected) in [
        ("Claude Code-credentials", "the default configuration"),
        ("Claude Code-credentials-1234", "configuration 1234"),
        ("other", "configuration other"),
    ] {
        assert_eq!(service_label(service, "Claude Code-credentials"), expected);
    }
}

#[test]
fn transcript_scope_requires_the_selected_keychain_service_to_match() {
    for (source, configured, blocked) in [
        ("claude.keychain:chosen", Some("chosen"), false),
        ("claude.keychain:other", Some("chosen"), true),
        ("claude.file", Some("chosen"), false),
        ("claude.keychain:other", None, false),
    ] {
        assert_eq!(
            local_transcript_warning(source, configured).is_some(),
            blocked,
            "{source}"
        );
    }
}

#[test]
fn scan_keeps_configured_service_first_and_tracks_read_issues() {
    let discovery = ServiceDiscovery::new("default".into());
    let services = discovery.services(&["other".into(), "default".into(), "third".into()]);
    let result = discovery.scan(services, |index, _| match index {
        0 => ServiceState::Incomplete,
        1 => ServiceState::Failed,
        _ => ServiceState::Available,
    });
    assert_eq!(result.services, ["default", "other", "third"]);
    assert_eq!(result.available, [2]);
    assert_eq!(result.first_error, Some(1));
    assert_eq!(result.account_error(), None);
    assert!(result.incomplete);
    assert_eq!(discovery.select(&result), Some(2));
}

#[test]
fn scan_reports_first_error_only_without_a_usable_account() {
    let discovery = ServiceDiscovery::new("default".into());
    let result = discovery.scan(discovery.services(&["other".into()]), |index, _| {
        if index == 0 {
            ServiceState::Failed
        } else {
            ServiceState::Missing
        }
    });
    assert_eq!(result.account_error(), Some(0));
    assert!(!result.incomplete);
    assert_eq!(discovery.select(&result), None);
}

#[test]
fn configured_service_wins_and_selection_stays_sticky() {
    let discovery = ServiceDiscovery::new("default".into());
    let first = discovery.scan(discovery.services(&["other".into()]), |_, _| ServiceState::Available);
    assert_eq!(discovery.select(&first), Some(0));
    let later = discovery.scan(discovery.services(&["other".into()]), |index, _| {
        if index == 0 {
            ServiceState::Missing
        } else {
            ServiceState::Available
        }
    });
    assert_eq!(discovery.select(&later), None);
    assert_eq!(discovery.save_target(&later), "default");
}

#[test]
fn lexical_fallback_and_empty_save_target_follow_configured_service() {
    let discovery = ServiceDiscovery::new("default".into());
    let scan = discovery.scan(discovery.services(&["z".into(), "a".into()]), |index, _| {
        if index == 0 {
            ServiceState::Missing
        } else {
            ServiceState::Available
        }
    });
    assert_eq!(discovery.select(&scan), Some(2));
    assert_eq!(discovery.save_target(&scan), "a");
    let empty = ServiceDiscovery::new("configured".into());
    let scan = empty.scan(empty.services(&[]), |_, _| ServiceState::Missing);
    assert_eq!(empty.save_target(&scan), "configured");
}
