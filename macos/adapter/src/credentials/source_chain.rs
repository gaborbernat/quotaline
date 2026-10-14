//! Bridges ordered macOS credential reads to shared source selection.

use std::sync::Arc;

use quotaline_credentials::ClaudeCredentialCandidate;
use quotaline_credentials::select_claude_credential;
use quotaline_credentials::source_chain::SourceState;
use quotaline_credentials::source_chain::first_available;
use quotaline_credentials::source_chain::last_error;
use quotaline_credentials::source_chain::save_target;

use super::source_chain_callback::CredentialSourceReader;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialSourcePolicy {
    First,
    Save,
    Claude,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum CredentialSourceScan {
    Missing,
    Present {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<f64>,
    },
    Incomplete,
    Failed,
}

impl CredentialSourceScan {
    pub(crate) fn state(&self) -> SourceState {
        match self {
            Self::Missing => SourceState::Missing,
            Self::Present { .. } => SourceState::Present,
            Self::Incomplete | Self::Failed => SourceState::Failed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CredentialSourceSelection {
    pub selected: Option<u32>,
    pub error: Option<u32>,
}

#[uniffi::export]
pub fn select_credential_source(
    policy: CredentialSourcePolicy,
    count: u32,
    reader: Arc<dyn CredentialSourceReader>,
) -> CredentialSourceSelection {
    let selection = match policy {
        CredentialSourcePolicy::First => first_available(count as usize, |index| reader.load(index as u32).state()),
        CredentialSourcePolicy::Save => {
            return CredentialSourceSelection {
                selected: save_target(count as usize, |index| reader.load(index as u32).state())
                    .map(|index| index as u32),
                error: None,
            };
        }
        CredentialSourcePolicy::Claude => {
            let mut scans = Vec::with_capacity(count as usize);
            let selection = last_error(count as usize, |index| {
                let scan = reader.load(index as u32);
                let state = scan.state();
                scans.push(scan);
                state
            });
            let (indices, candidates): (Vec<_>, Vec<_>) = scans
                .iter()
                .enumerate()
                .filter_map(|(index, scan)| match scan {
                    CredentialSourceScan::Present {
                        access_token,
                        refresh_token,
                        expires_at,
                    } => Some((
                        index,
                        ClaudeCredentialCandidate {
                            access_token,
                            refresh_token: refresh_token.as_deref(),
                            expires_at: *expires_at,
                        },
                    )),
                    CredentialSourceScan::Missing | CredentialSourceScan::Incomplete | CredentialSourceScan::Failed => {
                        None
                    }
                })
                .unzip();
            return CredentialSourceSelection {
                selected: select_claude_credential(&candidates).map(|index| indices[index] as u32),
                error: selection.error.map(|index| index as u32),
            };
        }
    };
    CredentialSourceSelection {
        selected: selection.present.map(|index| index as u32),
        error: selection.error.map(|index| index as u32),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/source_chain.rs"]
mod tests;
