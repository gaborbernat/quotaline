use super::*;

fn verification(environment: HashMap<String, String>) -> LaunchPlan {
    plan(&[VERIFICATION_ARGUMENT.into()], environment, "/work/cache", None, false)
}

#[test]
fn standard_launch_preserves_environment_and_hands_off() {
    let environment = HashMap::from([
        ("EXISTING".into(), "value".into()),
        ("QUOTALINE_DEMO".into(), "1".into()),
    ]);
    let planned = plan(&[], environment.clone(), "/work/cache", None, false);
    assert!(!planned.verification);
    assert_eq!(planned.environment, environment);
    assert_eq!(
        planned.relaunch_environment,
        HashMap::from([("EXISTING".into(), "value".into())])
    );
    assert_eq!(planned.defaults_suite_name, None);
    assert_eq!(planned.support_directory, None);
    assert_eq!(planned.profile, None);
    assert!(planned.yields_to_running_instance);
    assert!(!planned.is_relaunch);
}

#[test]
fn relaunch_does_not_hand_off_to_an_existing_instance() {
    let planned = plan(
        &[RELAUNCHED_ARGUMENT.into()],
        HashMap::new(),
        "/work/cache",
        None,
        false,
    );
    assert!(!planned.yields_to_running_instance);
    assert!(planned.is_relaunch);
}

#[test]
fn verification_argument_isolates_defaults_and_support_directory() {
    let planned = verification(HashMap::from([
        (VERIFICATION_SESSION_KEY.into(), "test/session 1".into()),
        ("EXISTING".into(), "value".into()),
    ]));
    assert!(planned.verification);
    assert_eq!(
        planned.defaults_suite_name.as_deref(),
        Some("tech.bernat.quotaline.verify.test-session-1")
    );
    assert_eq!(
        planned.support_directory.as_deref(),
        Some("/work/cache/quotaline-verify-test-session-1")
    );
    assert_eq!(planned.environment.get("EXISTING").map(String::as_str), Some("value"));
    assert_eq!(planned.environment.get("QUOTALINE_DEMO").map(String::as_str), Some("1"));
    assert_eq!(
        planned.environment.get("QUOTALINE_OPEN_POPOVER").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        planned
            .environment
            .get(VERIFICATION_ENVIRONMENT_KEY)
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        planned.environment.get(VERIFICATION_SESSION_KEY).map(String::as_str),
        Some("test/session 1")
    );
    assert_eq!(
        planned.environment.get(VERIFICATION_SUPPORT_DIRECTORY_KEY),
        planned.support_directory.as_ref()
    );
    assert!(!planned.relaunch_environment.contains_key("QUOTALINE_DEMO"));
    assert!(!planned.yields_to_running_instance);
}

#[test]
fn verification_can_be_selected_by_environment_or_build() {
    for (environment, verification_only) in [
        (HashMap::from([(VERIFICATION_ENVIRONMENT_KEY.into(), "".into())]), false),
        (HashMap::new(), true),
    ] {
        let planned = plan(&[], environment, "/work/cache", Some("generated-42"), verification_only);
        assert!(planned.verification);
        assert_eq!(
            planned.defaults_suite_name.as_deref(),
            Some("tech.bernat.quotaline.verify.generated-42")
        );
    }
}

#[test]
fn verification_uses_explicit_support_path_and_safe_empty_session() {
    let planned = verification(HashMap::from([
        (VERIFICATION_SESSION_KEY.into(), String::new()),
        (VERIFICATION_SUPPORT_DIRECTORY_KEY.into(), "/work/shared".into()),
    ]));
    assert_eq!(
        planned.defaults_suite_name.as_deref(),
        Some("tech.bernat.quotaline.verify.session")
    );
    assert_eq!(planned.support_directory.as_deref(), Some("/work/shared"));
    assert_eq!(
        planned.environment.get(VERIFICATION_SESSION_KEY).map(String::as_str),
        Some("")
    );
    assert_eq!(
        verification(HashMap::new()).defaults_suite_name.as_deref(),
        Some("tech.bernat.quotaline.verify.manual")
    );
}

#[test]
fn session_sanitization_keeps_letters_digits_graphemes_and_hyphens() {
    let planned = verification(HashMap::from([(
        VERIFICATION_SESSION_KEY.into(),
        "A9- e\u{301}/_😀".into(),
    )]));
    assert_eq!(
        planned.defaults_suite_name.as_deref(),
        Some("tech.bernat.quotaline.verify.A9--e\u{301}---")
    );
}

#[test]
fn verification_profile_accepts_known_fixture_and_appearance() {
    for (fixture, appearance) in [("long-text", "light"), ("control-audit", "dark")] {
        let planned = verification(HashMap::from([
            (FIXTURE_ENVIRONMENT_KEY.into(), fixture.into()),
            (APPEARANCE_ENVIRONMENT_KEY.into(), appearance.into()),
            (VISIBLE_FRAME_WIDTH_ENVIRONMENT_KEY.into(), "752".into()),
            (NATIVE_PANELS_ENVIRONMENT_KEY.into(), "1".into()),
        ]));
        assert_eq!(
            planned.profile,
            Some(VerificationProfile {
                fixture: fixture.into(),
                visible_frame_width: Some(752.0),
                native_panels: true,
                appearance: Some(appearance.into()),
            })
        );
    }
}

#[test]
fn verification_profile_rejects_unknown_and_invalid_values() {
    for width in ["nan", "infinity", "-1", "0", "narrow"] {
        let planned = verification(HashMap::from([
            (FIXTURE_ENVIRONMENT_KEY.into(), "unknown".into()),
            (APPEARANCE_ENVIRONMENT_KEY.into(), "unknown".into()),
            (VISIBLE_FRAME_WIDTH_ENVIRONMENT_KEY.into(), width.into()),
            (NATIVE_PANELS_ENVIRONMENT_KEY.into(), "0".into()),
        ]));
        assert_eq!(
            planned.profile,
            Some(VerificationProfile {
                fixture: "standard".into(),
                visible_frame_width: None,
                native_panels: false,
                appearance: None,
            }),
            "{width}"
        );
    }
    assert!(!verification(HashMap::new()).profile.unwrap().native_panels);
}

#[test]
fn relaunch_arguments_preserve_verification_and_logging_flags() {
    for (verification, logging, expected) in [
        (false, false, vec!["--relaunched"]),
        (true, false, vec!["--relaunched", "--verify-ui"]),
        (false, true, vec!["--relaunched", "-detailedLogging", "YES"]),
        (
            true,
            true,
            vec!["--relaunched", "--verify-ui", "-detailedLogging", "YES"],
        ),
    ] {
        assert_eq!(relaunch_arguments(verification, logging), expected);
    }
}
