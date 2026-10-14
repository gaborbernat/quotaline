//! Bridges macOS Keychain access and denial policy to Swift.

use quotaline_macos_platform::keychain::{self, DenialCache, KeychainValue, Reader};
use std::sync::{Arc, Mutex};

use super::keychain_callback::KeychainReader;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct KeychainRetryService {
    pub service: String,
    pub including_variants: bool,
}

#[uniffi::export]
pub fn keychain_retry_service(provider: String) -> Option<KeychainRetryService> {
    keychain::retry_service(&provider).map(|(service, including_variants)| KeychainRetryService {
        service,
        including_variants,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum KeychainLookup {
    Missing,
    Item { data: Vec<u8>, account: Option<String> },
    Error { status: i32 },
}

pub(crate) struct KeychainBridge(pub Arc<dyn KeychainReader>);

impl Reader for KeychainBridge {
    fn load(&self, service: &str, account: Option<String>) -> Result<Option<KeychainValue>, i32> {
        match self.0.load(service.into(), account) {
            KeychainLookup::Missing => Ok(None),
            KeychainLookup::Item { data, account } => Ok(Some(KeychainValue { data, account })),
            KeychainLookup::Error { status } => Err(status),
        }
    }
}

#[derive(uniffi::Object)]
pub struct KeychainDenialCache {
    state: Mutex<DenialCache>,
}

#[uniffi::export]
impl KeychainDenialCache {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(DenialCache::default()),
        }
    }

    pub fn status(&self, service: String, account: Option<String>, now: f64) -> Option<i32> {
        self.state.lock().unwrap().status(&service, account.as_deref(), now)
    }

    pub fn record(&self, status: i32, service: String, account: Option<String>, now: f64) {
        self.state
            .lock()
            .unwrap()
            .record(status, &service, account.as_deref(), now);
    }

    pub fn invalidate(&self, service: String, including_variants: bool) {
        self.state.lock().unwrap().invalidate(&service, including_variants);
    }
}

impl Default for KeychainDenialCache {
    fn default() -> Self {
        Self::new()
    }
}

#[uniffi::export]
pub fn keychain_denial_cache_interval() -> f64 {
    keychain::DENIAL_CACHE_INTERVAL
}

#[uniffi::export]
pub fn keychain_is_denial(status: i32) -> bool {
    keychain::is_denial(status)
}

#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct KeychainItemRecord {
    pub data: Vec<u8>,
    pub account: Option<String>,
}

#[cfg(target_os = "macos")]
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum KeychainAccessError {
    #[error("Keychain status {status}")]
    Status { status: i32 },
}

#[cfg(target_os = "macos")]
#[derive(uniffi::Object)]
pub struct KeychainAccess {
    store: keychain::native::Store,
}

#[cfg(target_os = "macos")]
#[uniffi::export]
impl KeychainAccess {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::with_store(keychain::native::Store::system())
    }

    pub fn load(
        &self,
        service: String,
        account: Option<String>,
    ) -> Result<Option<KeychainItemRecord>, KeychainAccessError> {
        self.store
            .load(&service, account.as_deref())
            .map(|item| {
                item.map(|item| KeychainItemRecord {
                    data: item.data,
                    account: item.account,
                })
            })
            .map_err(|status| KeychainAccessError::Status { status })
    }

    pub fn services(&self, prefix: String) -> Result<Vec<String>, KeychainAccessError> {
        self.store
            .service_names(&prefix)
            .map_err(|status| KeychainAccessError::Status { status })
    }

    pub fn save(&self, data: Vec<u8>, service: String, account: String) -> Result<(), KeychainAccessError> {
        self.store
            .save(&data, &service, &account)
            .map_err(|status| KeychainAccessError::Status { status })
    }
}

#[cfg(target_os = "macos")]
impl KeychainAccess {
    pub fn with_store(store: keychain::native::Store) -> Self {
        Self { store }
    }
}

#[cfg(target_os = "macos")]
impl Default for KeychainAccess {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/keychain/tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../tests/unit/credentials/keychain/native.rs"]
mod native_tests;
