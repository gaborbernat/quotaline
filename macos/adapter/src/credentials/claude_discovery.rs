//! Bridges macOS Claude Keychain service discovery to Swift.

use std::sync::Arc;

use quotaline_macos_platform::claude_discovery::{
    ServiceDiscovery as Discovery, ServiceScan as PlatformScan, ServiceState,
};

use super::{source_chain::CredentialSourceScan, source_chain_callback::CredentialSourceReader};

#[uniffi::export]
pub fn claude_service_label(service: String, default_service: String) -> String {
    quotaline_macos_platform::claude_discovery::service_label(&service, &default_service)
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ClaudeServiceScan {
    pub services: Vec<String>,
    pub available: Vec<u32>,
    pub account_error: Option<u32>,
    pub first_error: Option<u32>,
    pub incomplete: bool,
}

impl From<ClaudeServiceScan> for PlatformScan {
    fn from(value: ClaudeServiceScan) -> Self {
        Self {
            services: value.services,
            available: value.available.into_iter().map(|index| index as usize).collect(),
            first_error: value.first_error.map(|index| index as usize),
            incomplete: value.incomplete,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ClaudeServiceSelection {
    pub selected: Option<u32>,
    pub error: Option<u32>,
    pub incomplete: bool,
    pub save_service: String,
}

#[derive(uniffi::Object)]
pub struct ClaudeServiceDiscovery {
    state: Discovery,
}

#[uniffi::export]
impl ClaudeServiceDiscovery {
    #[uniffi::constructor]
    pub fn new(configured: String) -> Self {
        Self {
            state: Discovery::new(configured),
        }
    }

    pub fn service_names(&self, listed: Vec<String>) -> Vec<String> {
        self.state.services(&listed)
    }

    pub fn scan(&self, services: Vec<String>, reader: Arc<dyn CredentialSourceReader>) -> ClaudeServiceScan {
        let result = self.state.scan(services, |index, _| match reader.load(index as u32) {
            CredentialSourceScan::Missing => ServiceState::Missing,
            CredentialSourceScan::Present { .. } => ServiceState::Available,
            CredentialSourceScan::Incomplete => ServiceState::Incomplete,
            CredentialSourceScan::Failed => ServiceState::Failed,
        });
        ClaudeServiceScan {
            account_error: result.account_error().map(|index| index as u32),
            services: result.services,
            available: result.available.into_iter().map(|index| index as u32).collect(),
            first_error: result.first_error.map(|index| index as u32),
            incomplete: result.incomplete,
        }
    }

    pub fn select(&self, scan: ClaudeServiceScan) -> ClaudeServiceSelection {
        let scan: PlatformScan = scan.into();
        let selected = self.state.select(&scan);
        ClaudeServiceSelection {
            selected: selected.map(|index| index as u32),
            error: selected
                .is_none()
                .then_some(scan.first_error)
                .flatten()
                .map(|index| index as u32),
            incomplete: selected.is_none() && scan.first_error.is_none() && scan.incomplete,
            save_service: self.state.save_target(&scan),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/claude_discovery.rs"]
mod tests;
