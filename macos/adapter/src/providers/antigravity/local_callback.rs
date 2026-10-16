//! Defines the native process scanner callback for local Antigravity discovery.

use super::local::AntigravityLocalServer;

#[uniffi::export(foreign)]
pub trait AntigravityServerScanner: Send + Sync {
    fn discover_servers(&self) -> Vec<AntigravityLocalServer>;
}
