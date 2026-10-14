//! Selects missing credentials for a panel-open refresh.

use std::time::Duration;

use quotaline_quota::{CredentialHealth, Provider};

pub fn should_check_credentials(
    provider: Option<&Provider>,
    enabled: bool,
    elapsed: Option<Duration>,
    interval: Duration,
) -> bool {
    enabled && needs_credentials(provider) && elapsed.is_none_or(|elapsed| elapsed >= interval)
}

pub fn needs_credentials(provider: Option<&Provider>) -> bool {
    provider.is_some_and(|provider| {
        provider.credential_health == CredentialHealth::Missing
            && provider.updated.is_none()
            && provider.windows.is_empty()
    })
}

#[cfg(test)]
#[path = "../tests/unit/panel_open/tests.rs"]
mod tests;
