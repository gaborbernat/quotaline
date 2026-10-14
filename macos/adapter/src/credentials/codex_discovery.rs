//! Bridges shared Codex multi-home discovery to Swift.

use std::{path::PathBuf, sync::Arc};

use quotaline_credential_sources::CodexAccountDiscovery as Discovery;

#[uniffi::export]
pub fn codex_account_label(email: Option<String>, account_home: String, user_home: String) -> String {
    quotaline_credential_sources::codex_account_label(
        email.as_deref(),
        std::path::Path::new(&account_home),
        std::path::Path::new(&user_home),
    )
}

use super::source_chain_callback::CredentialSourceReader;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexAccountScan {
    pub available: Vec<u32>,
    pub first_error: Option<u32>,
}

#[derive(uniffi::Object)]
pub struct CodexAccountDiscovery {
    state: Discovery,
}

#[uniffi::export]
impl CodexAccountDiscovery {
    #[uniffi::constructor]
    pub fn new(homes: Vec<String>) -> Self {
        Self {
            state: Discovery::new(homes.into_iter().map(PathBuf::from).collect()),
        }
    }

    pub fn scan(&self, reader: Arc<dyn CredentialSourceReader>) -> CodexAccountScan {
        let result = self.state.scan(|index| reader.load(index as u32).state());
        CodexAccountScan {
            available: result.available.into_iter().map(|index| index as u32).collect(),
            first_error: result.first_error.map(|index| index as u32),
        }
    }

    pub fn select(&self, available: Vec<u32>) -> Option<u32> {
        self.state
            .select(&available.into_iter().map(|index| index as usize).collect::<Vec<_>>())
            .map(|index| index as u32)
    }

    pub fn save_target(&self, available: Vec<u32>) -> Option<u32> {
        self.state
            .save_target(&available.into_iter().map(|index| index as usize).collect::<Vec<_>>())
            .map(|index| index as u32)
    }

    pub fn selected_home(&self) -> Option<String> {
        self.state
            .selected_home()
            .map(|path| path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/codex_discovery.rs"]
mod tests;
