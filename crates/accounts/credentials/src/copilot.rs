//! Projects Copilot CLI, legacy, and keyring credentials into shared values.

use serde_json::Value;
use std::collections::HashMap;

use super::copilot_host;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopilotCredential {
    pub token: String,
    pub user: Option<String>,
    pub host: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopilotAccount {
    pub account: Option<String>,
    pub credential: Option<CopilotCredential>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopilotSecretError {
    InvalidText,
    InvalidJson,
    InvalidHost,
}

pub fn copilot_environment_credential(environment: &HashMap<String, String>) -> Option<CopilotCredential> {
    let token = ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"]
        .into_iter()
        .find_map(|key| environment.get(key).filter(|value| !value.is_empty()))?;
    Some(CopilotCredential {
        token: token.clone(),
        user: None,
        host: environment
            .get("GH_HOST")
            .cloned()
            .unwrap_or_else(|| "github.com".into()),
    })
}

pub fn copilot_cli_accounts(document: &Value) -> Vec<CopilotAccount> {
    let users = &document["loggedInUsers"];
    let entries: Vec<_> = match users {
        Value::Object(entries) => entries.iter().map(|(key, value)| (Some(key.as_str()), value)).collect(),
        Value::Array(entries) => entries.iter().map(|value| (None, value)).collect(),
        _ => return Vec::new(),
    };
    entries
        .into_iter()
        .map(|(key, value)| {
            let user = ["user", "login", "username"]
                .into_iter()
                .find_map(|name| nonempty(&value[name]))
                .or_else(|| nonempty(value));
            let host = ["host", "url"]
                .into_iter()
                .find_map(|name| nonempty(&value[name]))
                .or(key)
                .unwrap_or("github.com");
            let account = nonempty(&value["keychainAccount"])
                .map(str::to_owned)
                .or_else(|| user.and_then(|user| copilot_host(host).map(|host| format!("https://{host}:{user}"))))
                .or_else(|| key.map(str::to_owned));
            let token = ["token", "oauthToken", "oauth_token"]
                .into_iter()
                .find_map(|name| nonempty(&value[name]));
            CopilotAccount {
                account,
                credential: token.map(|token| CopilotCredential {
                    token: token.into(),
                    user: user.map(str::to_owned),
                    host: host.into(),
                }),
            }
        })
        .collect()
}

pub fn copilot_legacy_credential(document: &Value) -> Option<CopilotCredential> {
    let entries = document.as_object()?;
    let mut keys: Vec<_> = entries.keys().collect();
    keys.sort_by_key(|key| (!key.contains("github.com"), *key));
    keys.into_iter().find_map(|key| {
        let value = &entries[key];
        Some(CopilotCredential {
            token: nonempty(&value["oauth_token"])?.into(),
            user: nonempty(&value["user"]).map(str::to_owned),
            host: key
                .split(':')
                .next()
                .filter(|host| !host.is_empty())
                .unwrap_or("github.com")
                .into(),
        })
    })
}

pub fn copilot_keyring_credential(
    bytes: &[u8],
    account: Option<&str>,
) -> Result<Option<CopilotCredential>, CopilotSecretError> {
    let raw = std::str::from_utf8(bytes)
        .map_err(|_| CopilotSecretError::InvalidText)?
        .trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let document = match serde_json::from_str::<Value>(raw) {
        Ok(document) => Some(document),
        Err(_) if raw.starts_with(['{', '[']) => return Err(CopilotSecretError::InvalidJson),
        Err(_) => None,
    };
    let (identity_host, identity_user) = account.map(account_identity).unwrap_or((None, None));
    let token = document
        .as_ref()
        .and_then(|document| {
            ["token", "access_token", "oauth_token"]
                .into_iter()
                .find_map(|name| nonempty(&document[name]))
                .or_else(|| nonempty(document))
        })
        .or_else(|| document.is_none().then_some(raw));
    let Some(token) = token else { return Ok(None) };
    let host = document
        .as_ref()
        .and_then(|document| nonempty(&document["host"]))
        .or(identity_host)
        .unwrap_or("github.com");
    let host = copilot_host(host).ok_or(CopilotSecretError::InvalidHost)?;
    Ok(Some(CopilotCredential {
        token: token.into(),
        user: document
            .as_ref()
            .and_then(|document| nonempty(&document["user"]).or_else(|| nonempty(&document["login"])))
            .or(identity_user)
            .map(str::to_owned),
        host,
    }))
}

fn account_identity(account: &str) -> (Option<&str>, Option<&str>) {
    let account = account.trim();
    let (value, had_scheme) = match account
        .strip_prefix("https://")
        .or_else(|| account.strip_prefix("http://"))
    {
        Some(value) => (value, true),
        None => (account, false),
    };
    let value = value.trim_matches('/');
    let Some((host, user)) = value.rsplit_once(':') else {
        return (None, (!had_scheme && !account.is_empty()).then_some(account));
    };
    let user = user.trim();
    if copilot_host(host).is_some() && !user.is_empty() && !user.contains('/') {
        (Some(host), Some(user))
    } else {
        (None, (!had_scheme && !account.is_empty()).then_some(account))
    }
}

fn nonempty(value: &Value) -> Option<&str> {
    value.as_str().filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "../tests/unit/copilot/tests.rs"]
mod tests;
