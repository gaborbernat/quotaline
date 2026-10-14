//! Declares the Swift Keychain callback used by macOS credential discovery.

use super::keychain::KeychainLookup;

#[uniffi::export(foreign)]
pub trait KeychainReader: Send + Sync {
    fn load(&self, service: String, account: Option<String>) -> KeychainLookup;
}
