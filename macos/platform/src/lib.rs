//! Owns macOS credential access policies and native adapters.

pub mod claude_discovery;
pub mod copilot;
pub mod export;
pub mod google_oauth;
pub mod keychain;
pub mod launch;
pub mod persistence;
#[cfg(target_os = "macos")]
pub mod process;
pub mod setup;
