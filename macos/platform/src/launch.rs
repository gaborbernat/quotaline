//! Resolves macOS verification and relaunch isolation from process inputs.

use std::collections::HashMap;
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;

pub const VERIFICATION_ARGUMENT: &str = "--verify-ui";
pub const RELAUNCHED_ARGUMENT: &str = "--relaunched";
pub const VERIFICATION_ENVIRONMENT_KEY: &str = "QUOTALINE_VERIFY_UI";
pub const VERIFICATION_SESSION_KEY: &str = "QUOTALINE_VERIFY_SESSION";
pub const VERIFICATION_SUPPORT_DIRECTORY_KEY: &str = "QUOTALINE_VERIFY_SUPPORT_DIRECTORY";
pub const VERIFICATION_SUITE_PREFIX: &str = "tech.bernat.quotaline.verify";
pub const FIXTURE_ENVIRONMENT_KEY: &str = "QUOTALINE_VERIFY_FIXTURE";
pub const VISIBLE_FRAME_WIDTH_ENVIRONMENT_KEY: &str = "QUOTALINE_VERIFY_VISIBLE_WIDTH";
pub const NATIVE_PANELS_ENVIRONMENT_KEY: &str = "QUOTALINE_VERIFY_NATIVE_PANELS";
pub const APPEARANCE_ENVIRONMENT_KEY: &str = "QUOTALINE_VERIFY_APPEARANCE";

#[derive(Clone, Debug, PartialEq)]
pub struct VerificationProfile {
    pub fixture: String,
    pub visible_frame_width: Option<f64>,
    pub native_panels: bool,
    pub appearance: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaunchPlan {
    pub verification: bool,
    pub environment: HashMap<String, String>,
    pub defaults_suite_name: Option<String>,
    pub support_directory: Option<String>,
    pub profile: Option<VerificationProfile>,
    pub yields_to_running_instance: bool,
    pub is_relaunch: bool,
    pub relaunch_environment: HashMap<String, String>,
}

pub fn plan(
    arguments: &[String],
    environment: HashMap<String, String>,
    temporary_directory: &str,
    verification_identifier: Option<&str>,
    verification_only: bool,
) -> LaunchPlan {
    let is_relaunch = arguments.iter().any(|argument| argument == RELAUNCHED_ARGUMENT);
    let verification = verification_only
        || arguments.iter().any(|argument| argument == VERIFICATION_ARGUMENT)
        || environment.contains_key(VERIFICATION_ENVIRONMENT_KEY);
    if !verification {
        let relaunch_environment = relaunch_environment(&environment);
        return LaunchPlan {
            verification,
            environment,
            defaults_suite_name: None,
            support_directory: None,
            profile: None,
            yields_to_running_instance: !is_relaunch,
            is_relaunch,
            relaunch_environment,
        };
    }

    let session = environment
        .get(VERIFICATION_SESSION_KEY)
        .map(String::as_str)
        .or(verification_identifier)
        .unwrap_or("manual")
        .to_owned();
    let safe_session: String = session
        .graphemes(true)
        .map(|character| {
            if character == "-" || character.chars().any(char::is_alphanumeric) {
                character
            } else {
                "-"
            }
        })
        .collect();
    let safe_session = if safe_session.is_empty() {
        "session"
    } else {
        &safe_session
    };
    let support_directory = environment
        .get(VERIFICATION_SUPPORT_DIRECTORY_KEY)
        .cloned()
        .unwrap_or_else(|| {
            Path::new(temporary_directory)
                .join(format!("quotaline-verify-{safe_session}"))
                .to_string_lossy()
                .into_owned()
        });
    let profile = VerificationProfile {
        fixture: match environment.get(FIXTURE_ENVIRONMENT_KEY).map(String::as_str) {
            Some("long-text") => "long-text",
            Some("control-audit") => "control-audit",
            _ => "standard",
        }
        .into(),
        visible_frame_width: environment
            .get(VISIBLE_FRAME_WIDTH_ENVIRONMENT_KEY)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|width| width.is_finite() && *width > 0.0),
        native_panels: environment
            .get(NATIVE_PANELS_ENVIRONMENT_KEY)
            .is_some_and(|value| value == "1"),
        appearance: environment
            .get(APPEARANCE_ENVIRONMENT_KEY)
            .filter(|value| matches!(value.as_str(), "light" | "dark"))
            .cloned(),
    };
    let mut resolved_environment = environment;
    resolved_environment.insert("QUOTALINE_DEMO".into(), "1".into());
    resolved_environment.insert("QUOTALINE_OPEN_POPOVER".into(), "1".into());
    resolved_environment.insert(VERIFICATION_ENVIRONMENT_KEY.into(), "1".into());
    resolved_environment.insert(VERIFICATION_SESSION_KEY.into(), session);
    resolved_environment.insert(VERIFICATION_SUPPORT_DIRECTORY_KEY.into(), support_directory.clone());
    let relaunch_environment = relaunch_environment(&resolved_environment);
    LaunchPlan {
        verification,
        environment: resolved_environment,
        defaults_suite_name: Some(format!("{VERIFICATION_SUITE_PREFIX}.{safe_session}")),
        support_directory: Some(support_directory),
        profile: Some(profile),
        yields_to_running_instance: false,
        is_relaunch,
        relaunch_environment,
    }
}

fn relaunch_environment(environment: &HashMap<String, String>) -> HashMap<String, String> {
    environment
        .iter()
        .filter(|(key, _)| key.as_str() != "QUOTALINE_DEMO")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub fn relaunch_arguments(verification: bool, detailed_logging: bool) -> Vec<String> {
    let mut arguments = vec![RELAUNCHED_ARGUMENT.into()];
    if verification {
        arguments.push(VERIFICATION_ARGUMENT.into());
    }
    if detailed_logging {
        arguments.extend(["-detailedLogging".into(), "YES".into()]);
    }
    arguments
}

#[cfg(test)]
#[path = "../tests/unit/launch/tests.rs"]
mod tests;
