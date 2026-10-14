//! Exposes macOS launch isolation decisions through UniFFI.

use quotaline_macos_platform::launch;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct LaunchConstantsRecord {
    pub verification_argument: String,
    pub relaunched_argument: String,
    pub verification_environment_key: String,
    pub verification_session_key: String,
    pub verification_support_directory_key: String,
    pub verification_suite_prefix: String,
    pub fixture_environment_key: String,
    pub visible_frame_width_environment_key: String,
    pub native_panels_environment_key: String,
    pub appearance_environment_key: String,
}

#[uniffi::export]
pub fn launch_constants() -> LaunchConstantsRecord {
    LaunchConstantsRecord {
        verification_argument: launch::VERIFICATION_ARGUMENT.into(),
        relaunched_argument: launch::RELAUNCHED_ARGUMENT.into(),
        verification_environment_key: launch::VERIFICATION_ENVIRONMENT_KEY.into(),
        verification_session_key: launch::VERIFICATION_SESSION_KEY.into(),
        verification_support_directory_key: launch::VERIFICATION_SUPPORT_DIRECTORY_KEY.into(),
        verification_suite_prefix: launch::VERIFICATION_SUITE_PREFIX.into(),
        fixture_environment_key: launch::FIXTURE_ENVIRONMENT_KEY.into(),
        visible_frame_width_environment_key: launch::VISIBLE_FRAME_WIDTH_ENVIRONMENT_KEY.into(),
        native_panels_environment_key: launch::NATIVE_PANELS_ENVIRONMENT_KEY.into(),
        appearance_environment_key: launch::APPEARANCE_ENVIRONMENT_KEY.into(),
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct LaunchProfileRecord {
    pub fixture: String,
    pub visible_frame_width: Option<f64>,
    pub native_panels: bool,
    pub appearance: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct LaunchPlanRecord {
    pub verification: bool,
    pub environment: HashMap<String, String>,
    pub defaults_suite_name: Option<String>,
    pub support_directory: Option<String>,
    pub profile: Option<LaunchProfileRecord>,
    pub yields_to_running_instance: bool,
    pub is_relaunch: bool,
    pub relaunch_environment: HashMap<String, String>,
}

#[uniffi::export]
pub fn resolve_launch(
    arguments: Vec<String>,
    environment: HashMap<String, String>,
    temporary_directory: String,
    verification_identifier: Option<String>,
    verification_only: bool,
) -> LaunchPlanRecord {
    let plan = launch::plan(
        &arguments,
        environment,
        &temporary_directory,
        verification_identifier.as_deref(),
        verification_only,
    );
    LaunchPlanRecord {
        verification: plan.verification,
        environment: plan.environment,
        defaults_suite_name: plan.defaults_suite_name,
        support_directory: plan.support_directory,
        profile: plan.profile.map(|profile| LaunchProfileRecord {
            fixture: profile.fixture,
            visible_frame_width: profile.visible_frame_width,
            native_panels: profile.native_panels,
            appearance: profile.appearance,
        }),
        yields_to_running_instance: plan.yields_to_running_instance,
        is_relaunch: plan.is_relaunch,
        relaunch_environment: plan.relaunch_environment,
    }
}

#[uniffi::export]
pub fn launch_relaunch_arguments(verification: bool, detailed_logging: bool) -> Vec<String> {
    launch::relaunch_arguments(verification, detailed_logging)
}

#[cfg(test)]
#[path = "../tests/unit/launch/tests.rs"]
mod tests;
