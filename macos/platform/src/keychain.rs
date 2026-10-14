//! Caches denied macOS Keychain reads until the user retries or the entry expires.

use std::collections::HashMap;

#[cfg(target_os = "macos")]
pub mod native;

pub struct KeychainValue {
    pub data: Vec<u8>,
    pub account: Option<String>,
}

pub trait Reader {
    fn load(&self, service: &str, account: Option<String>) -> Result<Option<KeychainValue>, i32>;
}

pub const DENIAL_CACHE_INTERVAL: f64 = 30.0 * 60.0;
pub const CODEX_SERVICE: &str = "Codex Auth";
pub const GEMINI_SERVICE: &str = "gemini-cli-oauth";
pub const ANTIGRAVITY_SERVICE: &str = "gemini";
pub const COPILOT_SERVICE: &str = "copilot-cli";

pub fn retry_service(provider: &str) -> Option<(String, bool)> {
    match provider {
        "claude" => Some((quotaline_credentials::claude_keychain_service(None), true)),
        "codex" => Some((CODEX_SERVICE.into(), false)),
        "gemini" => Some((GEMINI_SERVICE.into(), false)),
        "antigravity" => Some((ANTIGRAVITY_SERVICE.into(), false)),
        "copilot" => Some((COPILOT_SERVICE.into(), false)),
        _ => None,
    }
}

pub fn is_denial(status: i32) -> bool {
    matches!(status, -25293 | -128 | -25308)
}

pub fn services(prefix: &str, names: &[String]) -> Vec<String> {
    let mut matches: Vec<_> = names.iter().filter(|name| name.starts_with(prefix)).cloned().collect();
    matches.sort();
    matches.dedup();
    matches
}

#[derive(Default)]
pub struct DenialCache {
    entries: HashMap<(String, Option<String>), (i32, f64)>,
}

impl DenialCache {
    pub fn status(&self, service: &str, account: Option<&str>, now: f64) -> Option<i32> {
        self.entries
            .get(&(service.to_owned(), account.map(str::to_owned)))
            .filter(|(_, until)| *until > now)
            .map(|(status, _)| *status)
    }

    pub fn record(&mut self, status: i32, service: &str, account: Option<&str>, now: f64) {
        self.entries.insert(
            (service.into(), account.map(str::to_owned)),
            (status, now + DENIAL_CACHE_INTERVAL),
        );
    }

    pub fn invalidate(&mut self, service: &str, including_variants: bool) {
        let variant = format!("{service}-");
        self.entries
            .retain(|(stored, _), _| stored != service && !(including_variants && stored.starts_with(&variant)));
    }
}

#[cfg(test)]
#[path = "../tests/unit/keychain/tests.rs"]
mod tests;
