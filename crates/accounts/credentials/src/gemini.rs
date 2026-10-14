//! Projects Gemini CLI credentials into typed fields.

use serde_json::{Value, json};

use super::jwt_claims;

#[derive(Clone, Debug, PartialEq)]
pub struct GeminiCredentialDocument {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: Option<f64>,
    pub email: Option<String>,
    pub hosted_domain: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeminiStorage {
    File,
    Keychain,
}

pub fn gemini_storage_mode(value: Option<&str>) -> GeminiStorage {
    if value == Some("true") {
        GeminiStorage::Keychain
    } else {
        GeminiStorage::File
    }
}

pub fn gemini_initial_document(
    access_token: &str,
    refresh_token: Option<&str>,
    id_token: Option<&str>,
    expires_at_millis: Option<f64>,
) -> Value {
    let mut document = json!({"access_token":access_token,"token_type":"Bearer"});
    for (key, value) in [("refresh_token", refresh_token), ("id_token", id_token)] {
        if let Some(value) = value {
            document[key] = Value::String(value.into());
        }
    }
    if let Some(expires_at_millis) = expires_at_millis {
        document["expiry_date"] = json!(expires_at_millis);
    }
    document
}

pub fn gemini_keychain_document(document: &Value, updated_at_millis: f64) -> Option<Value> {
    let mut updated = document.as_object()?.clone();
    if updated.get("token").is_some_and(Value::is_object) {
        updated.entry("serverName").or_insert_with(|| json!("main-account"));
        updated.insert("updatedAt".into(), json!(updated_at_millis));
        return Some(Value::Object(updated));
    }
    let credential = gemini_credential_document(document)?;
    let mut token = json!({"accessToken":credential.access_token,"tokenType":"Bearer"});
    for (key, value) in [
        ("refreshToken", credential.refresh_token),
        ("idToken", credential.id_token),
    ] {
        if let Some(value) = value {
            token[key] = Value::String(value);
        }
    }
    if let Some(expires_at) = credential.expires_at {
        token["expiresAt"] = json!(expires_at * 1000.0);
    }
    Some(json!({"serverName":"main-account","token":token,"updatedAt":updated_at_millis}))
}

pub fn gemini_credential_document(document: &Value) -> Option<GeminiCredentialDocument> {
    let wrapped = &document["token"];
    let access_token = document["access_token"]
        .as_str()
        .or_else(|| wrapped["accessToken"].as_str())?
        .to_owned();
    let id_token = document["id_token"]
        .as_str()
        .or_else(|| wrapped["idToken"].as_str())
        .map(str::to_owned);
    let claims = id_token.as_deref().and_then(jwt_claims);
    Some(GeminiCredentialDocument {
        access_token,
        refresh_token: document["refresh_token"]
            .as_str()
            .or_else(|| wrapped["refreshToken"].as_str())
            .map(str::to_owned),
        id_token,
        expires_at: document["expiry_date"]
            .as_f64()
            .or_else(|| wrapped["expiresAt"].as_f64())
            .map(|value| value / 1000.0),
        email: claims
            .as_ref()
            .and_then(|claims| claims["email"].as_str())
            .map(str::to_owned),
        hosted_domain: claims
            .as_ref()
            .and_then(|claims| claims["hd"].as_str())
            .map(str::to_owned),
    })
}

#[cfg(test)]
#[path = "../tests/unit/gemini/tests.rs"]
mod tests;
