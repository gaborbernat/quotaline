use super::*;

#[test]
fn launch_constants_bridge_matches_platform_policy() {
    let keys = launch_constants();
    assert_eq!(keys.verification_argument, launch::VERIFICATION_ARGUMENT);
    assert_eq!(keys.relaunched_argument, launch::RELAUNCHED_ARGUMENT);
    assert_eq!(keys.verification_environment_key, launch::VERIFICATION_ENVIRONMENT_KEY);
    assert_eq!(keys.verification_session_key, launch::VERIFICATION_SESSION_KEY);
    assert_eq!(
        keys.verification_support_directory_key,
        launch::VERIFICATION_SUPPORT_DIRECTORY_KEY
    );
    assert_eq!(keys.verification_suite_prefix, launch::VERIFICATION_SUITE_PREFIX);
    assert_eq!(keys.fixture_environment_key, launch::FIXTURE_ENVIRONMENT_KEY);
    assert_eq!(
        keys.visible_frame_width_environment_key,
        launch::VISIBLE_FRAME_WIDTH_ENVIRONMENT_KEY
    );
    assert_eq!(
        keys.native_panels_environment_key,
        launch::NATIVE_PANELS_ENVIRONMENT_KEY
    );
    assert_eq!(keys.appearance_environment_key, launch::APPEARANCE_ENVIRONMENT_KEY);
}

#[test]
fn launch_bridge_returns_isolated_plan_and_profile() {
    let planned = resolve_launch(
        vec!["--verify-ui".into()],
        HashMap::from([("QUOTALINE_VERIFY_FIXTURE".into(), "long-text".into())]),
        "/work/cache".into(),
        Some("test".into()),
        false,
    );
    assert!(planned.verification);
    assert_eq!(
        planned.defaults_suite_name.as_deref(),
        Some("tech.bernat.quotaline.verify.test")
    );
    assert_eq!(
        planned.support_directory.as_deref(),
        Some("/work/cache/quotaline-verify-test")
    );
    assert_eq!(planned.profile.unwrap().fixture, "long-text");
    assert!(!planned.yields_to_running_instance);
    assert!(!planned.is_relaunch);
    assert!(!planned.relaunch_environment.contains_key("QUOTALINE_DEMO"));
    assert_eq!(
        launch_relaunch_arguments(true, true),
        ["--relaunched", "--verify-ui", "-detailedLogging", "YES"]
    );
}

#[test]
fn launch_bridge_keeps_standard_mode() {
    let planned = resolve_launch(vec![], HashMap::new(), "/work/cache".into(), None, false);
    assert!(!planned.verification);
    assert_eq!(planned.profile, None);
    assert!(planned.yields_to_running_instance);
}
