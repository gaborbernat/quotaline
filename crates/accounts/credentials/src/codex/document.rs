//! Projects a Codex auth document into credential fields.

use serde_json::Value;

use super::super::{codex_subscription_until, jwt_claims};

pub const CODEX_API_KEY_USAGE_UNAVAILABLE: &str =
    "Codex API-key sign-in has no usage endpoint. Sign in with the Codex CLI to view usage.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexCredentialDocument {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub account_id: Option<String>,
    pub api_key: Option<String>,
    pub last_refresh: Option<String>,
    pub email: Option<String>,
    pub plan_type: Option<String>,
    pub subscription_active_until: Option<String>,
    pub supports_usage_api: bool,
}

pub fn codex_credential_available(document: &Value) -> bool {
    access_token(document).is_some()
}

fn access_token(document: &Value) -> Option<&str> {
    document["tokens"]["access_token"]
        .as_str()
        .or_else(|| document["OPENAI_API_KEY"].as_str())
}

pub fn codex_credential_document(document: &Value) -> Option<CodexCredentialDocument> {
    let tokens = &document["tokens"];
    let api_key = document["OPENAI_API_KEY"].as_str().map(str::to_owned);
    let access_token = access_token(document)?.to_owned();
    let id_token = tokens["id_token"].as_str().map(str::to_owned);
    let claims = id_token.as_deref().and_then(jwt_claims);
    let account_id = tokens["account_id"]
        .as_str()
        .or_else(|| {
            claims
                .as_ref()
                .and_then(|claims| claims["https://api.openai.com/auth"]["chatgpt_account_id"].as_str())
        })
        .map(str::to_owned);
    Some(CodexCredentialDocument {
        access_token,
        refresh_token: tokens["refresh_token"].as_str().map(str::to_owned),
        id_token: id_token.clone(),
        account_id,
        api_key,
        last_refresh: document["last_refresh"].as_str().map(str::to_owned),
        email: claims
            .as_ref()
            .and_then(|claims| claims["email"].as_str())
            .map(str::to_owned),
        plan_type: claims
            .as_ref()
            .and_then(|claims| claims["https://api.openai.com/auth"]["chatgpt_plan_type"].as_str())
            .map(str::to_owned),
        subscription_active_until: id_token.as_deref().and_then(codex_subscription_until),
        supports_usage_api: tokens["access_token"].as_str().is_some(),
    })
}

#[cfg(test)]
#[path = "../../tests/unit/codex/document/tests.rs"]
mod tests;
