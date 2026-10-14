//! Decides which settings controls apply to the current providers.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default)]
pub struct Discovery {
    pub snapshot: bool,
    pub analytics: bool,
    pub credential_usable: bool,
}

pub fn discovered(facts: Discovery) -> bool {
    facts.snapshot || facts.analytics || facts.credential_usable
}

pub fn active(facts: Discovery, enabled_override: Option<bool>) -> bool {
    discovered(facts) && enabled_override.unwrap_or(true)
}

pub fn visible(facts: Discovery, show_all: bool, revealed: bool) -> bool {
    show_all || revealed || discovered(facts)
}

#[derive(Clone, Debug)]
pub struct Provider {
    pub id: String,
    pub long_window: bool,
    pub reset_credits: bool,
    pub local_auth: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub analytics_interval_changed: bool,
    pub workdays_changed: bool,
    pub token_refresh_enabled: bool,
    pub expiring_credits_enabled: bool,
    pub sandboxed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Visibility {
    pub analytics: bool,
    pub pace: bool,
    pub token_refresh: bool,
    pub expiring_credits: bool,
}

pub fn visibility(providers: &[Provider], options: Options) -> Visibility {
    Visibility {
        analytics: options.analytics_interval_changed
            || providers
                .iter()
                .any(|provider| matches!(provider.id.as_str(), "claude" | "codex")),
        pace: options.workdays_changed || providers.iter().any(|provider| provider.long_window),
        token_refresh: options.token_refresh_enabled
            || providers.iter().any(|provider| match provider.id.as_str() {
                "claude" | "codex" => true,
                "gemini" => !options.sandboxed,
                "antigravity" => !options.sandboxed && !provider.local_auth,
                _ => false,
            }),
        expiring_credits: options.expiring_credits_enabled || providers.iter().any(|provider| provider.reset_credits),
    }
}

#[cfg(test)]
#[path = "../tests/unit/context/tests.rs"]
mod tests;
