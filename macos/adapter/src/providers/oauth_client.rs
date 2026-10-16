//! Exposes shared installed OAuth client parsing to Swift.

use quotaline_credential_sources::google_oauth;
use quotaline_macos_platform::google_oauth as macos_oauth;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct OAuthClientRecord {
    pub id: String,
    pub secret: String,
}

impl From<google_oauth::OAuthClient> for OAuthClientRecord {
    fn from(client: google_oauth::OAuthClient) -> Self {
        Self {
            id: client.id,
            secret: client.secret,
        }
    }
}

#[uniffi::export]
pub fn antigravity_oauth_client(
    home: String,
    environment: std::collections::HashMap<String, String>,
) -> Option<OAuthClientRecord> {
    macos_oauth::antigravity_client(
        std::path::Path::new(&home),
        std::path::Path::new("/Applications"),
        &environment,
    )
    .map(Into::into)
}

#[uniffi::export]
pub fn gemini_oauth_client(
    home: String,
    environment: std::collections::HashMap<String, String>,
) -> Option<OAuthClientRecord> {
    macos_oauth::gemini_client(
        std::path::Path::new(&home),
        &["/opt/homebrew".into(), "/usr/local".into()],
        &environment,
    )
    .map(Into::into)
}

#[cfg(test)]
#[path = "../../tests/unit/providers/oauth_client/tests.rs"]
mod tests;
