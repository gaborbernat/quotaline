//! Reads credentials from provider owned local files.

mod account;
mod codex;
mod copilot;
mod credential_file;
mod cursor;
pub mod google_oauth;

pub use account::codex_account_label;
pub use codex::rollouts as codex_rollouts;
pub use codex::{
    AccountDiscovery as CodexAccountDiscovery, AccountScan as CodexAccountScan, CodexCredentialStoreMode,
    codex_credential_store_mode, codex_homes, read_codex_credential_store_mode, select_codex_home,
};
pub use copilot::{
    CopilotCli, CopilotDiscovery, CopilotDiscoveryError, CopilotFound, CopilotLegacy, CopilotLegacyRecovery,
    CopilotOrigin, CopilotPending, CopilotSourceError, begin_copilot_discovery, discover_copilot_legacy,
    read_copilot_cli,
};
pub use credential_file::{CredentialFileError, CredentialProvider, read_credential_file, usable_credential_payload};
pub use cursor::{CursorAuth, CursorDiscovery, CursorOrigin, CursorSourceError, discover_cursor};
