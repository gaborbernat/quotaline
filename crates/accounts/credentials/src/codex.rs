//! Defines Codex OAuth refresh payloads and credential updates.

use serde_json::{Value, json};

mod document;
pub use document::{
    CODEX_API_KEY_USAGE_UNAVAILABLE, CodexCredentialDocument, codex_credential_available, codex_credential_document,
};

pub const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CODEX_ORIGINATOR: &str = "codex_cli_rs";

pub fn codex_initial_document(
    access_token: &str,
    refresh_token: Option<&str>,
    id_token: Option<&str>,
    account_id: Option<&str>,
    last_refresh: Option<&str>,
) -> Value {
    let mut tokens = json!({"access_token":access_token});
    for (key, value) in [
        ("refresh_token", refresh_token),
        ("id_token", id_token),
        ("account_id", account_id),
    ] {
        if let Some(value) = value {
            tokens[key] = Value::String(value.into());
        }
    }
    let mut document = json!({"tokens":tokens,"auth_mode":"chatgpt","OPENAI_API_KEY":null});
    if let Some(last_refresh) = last_refresh {
        document["last_refresh"] = Value::String(last_refresh.into());
    }
    document
}

pub fn codex_request_headers(token: &str, account_id: Option<&str>) -> Vec<(&'static str, String)> {
    let mut headers = vec![
        ("Authorization", format!("Bearer {token}")),
        ("originator", CODEX_ORIGINATOR.into()),
        ("User-Agent", format!("{CODEX_ORIGINATOR}/quotaline")),
    ];
    if let Some(account_id) = account_id {
        headers.push(("ChatGPT-Account-Id", account_id.into()));
    }
    headers
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexRefresh {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub error: Option<String>,
}

pub fn codex_refresh_request(refresh_token: &str) -> Value {
    json!({"client_id":CODEX_CLIENT_ID,"grant_type":"refresh_token","refresh_token":refresh_token})
}

pub fn codex_refresh_response(response: &Value) -> Option<CodexRefresh> {
    response.as_object().map(|_| CodexRefresh {
        access_token: response["access_token"].as_str().map(str::to_owned),
        refresh_token: response["refresh_token"].as_str().map(str::to_owned),
        id_token: response["id_token"].as_str().map(str::to_owned),
        error: response["error"].as_str().map(str::to_owned),
    })
}

pub fn codex_refreshed_document(document: &Value, token: &CodexRefresh, refreshed_at: &str) -> Option<Value> {
    let access_token = token.access_token.as_ref()?;
    let mut updated = document.clone();
    let object = updated.as_object_mut()?;
    let tokens = object.entry("tokens").or_insert_with(|| json!({}));
    if !tokens.is_object() {
        *tokens = json!({});
    }
    let tokens = tokens.as_object_mut().expect("tokens was made an object");
    tokens.insert("access_token".into(), Value::String(access_token.clone()));
    if let Some(refresh_token) = &token.refresh_token {
        tokens.insert("refresh_token".into(), Value::String(refresh_token.clone()));
    }
    if let Some(id_token) = &token.id_token {
        tokens.insert("id_token".into(), Value::String(id_token.clone()));
    }
    updated
        .as_object_mut()?
        .insert("last_refresh".into(), Value::String(refreshed_at.into()));
    Some(updated)
}

#[cfg(test)]
#[path = "../tests/unit/codex/tests.rs"]
mod tests;
