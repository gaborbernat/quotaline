//! Checks settings visibility and notification copy.

use super::*;

fn provider(id: &str) -> Provider {
    Provider {
        id: id.into(),
        long_window: false,
        reset_credits: false,
        local_auth: false,
    }
}

#[test]
fn discovery_accepts_each_source_without_false_legacy_matches() {
    assert!(!discovered(Discovery::default()));
    for facts in [
        Discovery {
            snapshot: true,
            ..Discovery::default()
        },
        Discovery {
            analytics: true,
            ..Discovery::default()
        },
        Discovery {
            credential_usable: true,
            ..Discovery::default()
        },
    ] {
        assert!(discovered(facts));
    }
    assert!(!discovered(Discovery::default()));
}

#[test]
fn activation_requires_discovery_and_respects_explicit_choice() {
    let facts = Discovery {
        snapshot: true,
        ..Discovery::default()
    };
    assert!(active(facts, None));
    assert!(active(facts, Some(true)));
    assert!(!active(facts, Some(false)));
    assert!(!active(Discovery::default(), Some(true)));
}

#[test]
fn settings_row_can_be_revealed_without_provider_discovery() {
    let missing = Discovery::default();
    assert!(!visible(missing, false, false));
    assert!(visible(missing, true, false));
    assert!(visible(missing, false, true));
    assert!(visible(
        Discovery {
            snapshot: true,
            ..Discovery::default()
        },
        false,
        false
    ));
}

#[test]
fn provider_capabilities_reveal_their_controls() {
    let mut claude = provider("claude");
    claude.long_window = true;
    claude.reset_credits = true;
    assert_eq!(
        visibility(&[claude], Options::default()),
        Visibility {
            analytics: true,
            pace: true,
            token_refresh: true,
            expiring_credits: true,
        }
    );
}

#[test]
fn saved_nondefault_settings_keep_controls_visible_without_providers() {
    let visible = visibility(
        &[],
        Options {
            analytics_interval_changed: true,
            workdays_changed: true,
            token_refresh_enabled: true,
            expiring_credits_enabled: true,
            sandboxed: false,
        },
    );
    assert_eq!(
        visible,
        Visibility {
            analytics: true,
            pace: true,
            token_refresh: true,
            expiring_credits: true,
        }
    );
}

#[test]
fn token_refresh_respects_platform_and_local_auth() {
    for (id, sandboxed, local_auth, expected) in [
        ("codex", true, false, true),
        ("gemini", true, false, false),
        ("gemini", false, false, true),
        ("antigravity", false, true, false),
        ("antigravity", false, false, true),
        ("antigravity", true, false, false),
        ("cursor", false, false, false),
    ] {
        let mut active = provider(id);
        active.local_auth = local_auth;
        assert_eq!(
            visibility(
                &[active],
                Options {
                    sandboxed,
                    ..Options::default()
                }
            )
            .token_refresh,
            expected,
            "{id}: sandboxed={sandboxed}, local={local_auth}"
        );
    }
}

#[test]
fn codex_reveals_analytics_without_other_capabilities() {
    assert!(visibility(&[provider("codex")], Options::default()).analytics);
}
