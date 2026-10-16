//! Declares the Swift credential store callback used by Rust source selection.

use super::source_chain::CredentialSourceScan;

#[uniffi::export(foreign)]
pub trait CredentialSourceReader: Send + Sync {
    fn load(&self, index: u32) -> CredentialSourceScan;
}
