//! Bridges macOS process and socket inspection to Swift.

use super::AntigravityLocalServer;
use quotaline_macos_platform::process::{LocalServer, Scanner};

impl From<LocalServer> for AntigravityLocalServer {
    fn from(server: LocalServer) -> Self {
        Self {
            pid: server.pid,
            port: server.port,
            csrf_token: server.csrf_token,
        }
    }
}

#[derive(uniffi::Object)]
pub struct AntigravityNativeScanner {
    scanner: Scanner,
}

#[uniffi::export]
impl AntigravityNativeScanner {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::with_scanner(Scanner::system())
    }

    pub fn discover_servers(&self) -> Vec<AntigravityLocalServer> {
        self.scanner.discover_servers().into_iter().map(Into::into).collect()
    }
}

impl AntigravityNativeScanner {
    pub fn with_scanner(scanner: Scanner) -> Self {
        Self { scanner }
    }
}

impl Default for AntigravityNativeScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/providers/antigravity/native.rs"]
mod tests;
