//! Tracks Claude Code Keychain services and their selected account.

use std::sync::Mutex;

pub fn service_label(service: &str, default_service: &str) -> String {
    if service == default_service {
        return "the default configuration".into();
    }
    let suffix = service
        .strip_prefix(default_service)
        .and_then(|value| value.strip_prefix('-'))
        .unwrap_or(service);
    format!("configuration {suffix}")
}

pub fn local_transcript_warning(source_id: &str, configured_service: Option<&str>) -> Option<&'static str> {
    let selected_service = source_id.strip_prefix("claude.keychain:");
    if selected_service
        .zip(configured_service)
        .is_some_and(|(selected, configured)| selected != configured)
    {
        Some(
            "Local transcript totals are unavailable for the selected Claude configuration. Its Keychain entry does not identify the transcript directory.",
        )
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Missing,
    Available,
    Incomplete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceScan {
    pub services: Vec<String>,
    pub available: Vec<usize>,
    pub first_error: Option<usize>,
    pub incomplete: bool,
}

impl ServiceScan {
    pub fn account_error(&self) -> Option<usize> {
        self.available.is_empty().then_some(self.first_error).flatten()
    }
}

pub struct ServiceDiscovery {
    configured: String,
    selected: Mutex<Option<String>>,
}

impl ServiceDiscovery {
    pub fn new(configured: String) -> Self {
        Self {
            configured,
            selected: Mutex::new(None),
        }
    }

    pub fn services(&self, listed: &[String]) -> Vec<String> {
        std::iter::once(self.configured.clone())
            .chain(listed.iter().filter(|service| *service != &self.configured).cloned())
            .collect()
    }

    pub fn scan(&self, services: Vec<String>, mut read: impl FnMut(usize, &str) -> ServiceState) -> ServiceScan {
        let mut result = ServiceScan {
            services,
            available: Vec::new(),
            first_error: None,
            incomplete: false,
        };
        for (index, service) in result.services.iter().enumerate() {
            match read(index, service) {
                ServiceState::Available => result.available.push(index),
                ServiceState::Incomplete => result.incomplete = true,
                ServiceState::Failed => {
                    result.first_error.get_or_insert(index);
                }
                ServiceState::Missing => {}
            }
        }
        result
    }

    pub fn select(&self, scan: &ServiceScan) -> Option<usize> {
        let mut selected = self.selected.lock().unwrap();
        if selected.is_none() {
            *selected = scan
                .available
                .iter()
                .map(|&index| &scan.services[index])
                .find(|service| *service == &self.configured)
                .or_else(|| scan.available.iter().map(|&index| &scan.services[index]).min())
                .cloned();
        }
        scan.available
            .iter()
            .copied()
            .find(|&index| selected.as_deref() == Some(scan.services[index].as_str()))
    }

    pub fn save_target(&self, scan: &ServiceScan) -> String {
        self.select(scan)
            .map(|index| scan.services[index].clone())
            .unwrap_or_else(|| self.configured.clone())
    }
}

#[cfg(test)]
#[path = "../tests/unit/claude_discovery.rs"]
mod tests;
