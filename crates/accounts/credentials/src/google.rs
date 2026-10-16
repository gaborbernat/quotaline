//! Defines Google OAuth refresh payloads used by Gemini and Antigravity.

use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct GoogleRefresh {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: f64,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoogleRefreshError {
    InvalidResponse,
    InvalidExpiry,
}

impl GoogleRefreshError {
    pub fn message(self, provider: &str) -> String {
        match self {
            Self::InvalidResponse => format!("{provider} refresh returned no access token"),
            Self::InvalidExpiry => format!("{provider} refresh returned an invalid expiry"),
        }
    }
}

pub fn google_refresh_request(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Vec<(&'static str, String)> {
    vec![
        ("client_id", client_id.into()),
        ("client_secret", client_secret.into()),
        ("grant_type", "refresh_token".into()),
        ("refresh_token", refresh_token.into()),
    ]
}

pub fn google_refresh_response(response: &Value) -> Result<GoogleRefresh, GoogleRefreshError> {
    let fields = response.as_object().ok_or(GoogleRefreshError::InvalidResponse)?;
    let expires_in = fields
        .get("expires_in")
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        })
        .unwrap_or(3600.0);
    if !expires_in.is_finite() || expires_in <= 0.0 {
        return Err(GoogleRefreshError::InvalidExpiry);
    }
    Ok(GoogleRefresh {
        access_token: response["access_token"].as_str().map(str::to_owned),
        refresh_token: response["refresh_token"].as_str().map(str::to_owned),
        id_token: response["id_token"].as_str().map(str::to_owned),
        expires_in,
        error: response["error"].as_str().map(str::to_owned),
        error_description: response["error_description"].as_str().map(str::to_owned),
    })
}

pub fn gemini_refreshed_document(document: &Value, token: &GoogleRefresh, now_millis: i64) -> Option<Value> {
    let access_token = token.access_token.as_ref()?;
    let expires_at = now_millis.saturating_add((token.expires_in * 1000.0) as i64);
    let mut updated = document.clone();
    let root = updated.as_object_mut()?;
    let wrapped = root.get("token").is_some_and(Value::is_object);
    let fields = if wrapped {
        root.get_mut("token")?.as_object_mut()?
    } else {
        root
    };
    fields.insert(
        if wrapped { "accessToken" } else { "access_token" }.into(),
        Value::String(access_token.clone()),
    );
    fields.insert(
        if wrapped { "expiresAt" } else { "expiry_date" }.into(),
        json!(expires_at),
    );
    if let Some(refresh_token) = &token.refresh_token {
        fields.insert(
            if wrapped { "refreshToken" } else { "refresh_token" }.into(),
            Value::String(refresh_token.clone()),
        );
    }
    if let Some(id_token) = &token.id_token {
        fields.insert(
            if wrapped { "idToken" } else { "id_token" }.into(),
            Value::String(id_token.clone()),
        );
    }
    Some(updated)
}

pub fn antigravity_refreshed_document(document: &Value, token: &GoogleRefresh, now_seconds: i64) -> Option<Value> {
    let access_token = token.access_token.as_ref()?;
    let mut updated = document.clone();
    let root = updated.as_object_mut()?;
    let fields = if root.get("token").is_some_and(Value::is_object) {
        root.get_mut("token")?.as_object_mut()?
    } else {
        root
    };
    let key = if fields.contains_key("accessToken") {
        "accessToken"
    } else {
        "access_token"
    };
    fields.insert(key.into(), Value::String(access_token.clone()));
    fields.insert(
        "expiry".into(),
        json!(now_seconds.saturating_add(token.expires_in as i64)),
    );
    Some(updated)
}

#[cfg(test)]
#[path = "../tests/unit/google/tests.rs"]
mod tests;
