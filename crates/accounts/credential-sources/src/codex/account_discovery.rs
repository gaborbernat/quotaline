//! Tracks Codex accounts across configured and default homes.

use std::path::PathBuf;

use parking_lot::Mutex;
use quotaline_credentials::source_chain::SourceState;

use super::select_codex_home;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountScan {
    pub available: Vec<usize>,
    pub first_error: Option<usize>,
}

pub struct AccountDiscovery {
    homes: Vec<PathBuf>,
    selected: Mutex<Option<PathBuf>>,
}

impl AccountDiscovery {
    pub fn new(homes: Vec<PathBuf>) -> Self {
        Self {
            homes,
            selected: Mutex::new(None),
        }
    }

    pub fn scan(&self, mut read: impl FnMut(usize) -> SourceState) -> AccountScan {
        let mut result = AccountScan {
            available: Vec::new(),
            first_error: None,
        };
        for index in 0..self.homes.len() {
            match read(index) {
                SourceState::Present => result.available.push(index),
                SourceState::Failed => {
                    result.first_error.get_or_insert(index);
                }
                SourceState::Missing => {}
            }
        }
        result
    }

    pub fn select(&self, available: &[usize]) -> Option<usize> {
        let paths = available
            .iter()
            .map(|&index| self.homes[index].clone())
            .collect::<Vec<_>>();
        let mut selected = self.selected.lock();
        let path = select_codex_home(selected.as_deref(), &paths)?;
        if selected.is_none() {
            *selected = Some(path.clone());
        }
        available.iter().copied().find(|&index| self.homes[index] == path)
    }

    pub fn save_target(&self, available: &[usize]) -> Option<usize> {
        self.select(available).or_else(|| (!self.homes.is_empty()).then_some(0))
    }

    pub fn selected_home(&self) -> Option<PathBuf> {
        self.selected.lock().clone()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/codex/account_discovery.rs"]
mod tests;
