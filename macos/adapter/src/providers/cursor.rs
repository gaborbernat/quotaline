//! Maps Cursor responses through the shared projector.

use quotaline_cursor as cursor;
use quotaline_cursor::request;
use std::sync::Arc;

pub mod identity;

use crate::http::{NativeHttpTransport, NativeTransport, ProviderFailure};
use crate::provider_record::ProviderRecord;

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum CursorError {
    #[error("Invalid Cursor response JSON")]
    InvalidJson,
}

#[derive(Debug, PartialEq, uniffi::Object)]
pub struct CursorResponse {
    usage: cursor::UsageResponse,
}

#[derive(Debug, PartialEq, uniffi::Enum)]
pub enum CursorUsageFetch {
    Response(Arc<CursorResponse>),
    Failure(ProviderFailure),
}

#[uniffi::export]
pub async fn cursor_fetch_usage(
    cookie: String,
    token: String,
    hint: String,
    home: String,
    transport: Arc<dyn NativeHttpTransport>,
) -> CursorUsageFetch {
    match cursor::fetch::fetch_usage(
        &NativeTransport(transport),
        &cursor::fetch::Endpoints::default(),
        &cookie,
        &token,
        Some(&home),
    )
    .await
    {
        Ok(usage) => CursorUsageFetch::Response(Arc::new(CursorResponse { usage })),
        Err(error) => CursorUsageFetch::Failure(error.provider_failure(&hint, Some(&home))),
    }
}

#[uniffi::export]
impl CursorResponse {
    #[uniffi::constructor]
    pub fn new(payload: Vec<u8>, summary_error: Option<String>) -> Result<Self, CursorError> {
        let document: serde_json::Value = serde_json::from_slice(&payload).map_err(|_| CursorError::InvalidJson)?;
        let usage = match summary_error {
            Some(error) => cursor::UsageResponse::period(&document, &error),
            None => cursor::UsageResponse::summary(document),
        };
        Ok(Self { usage })
    }

    pub fn warning(&self) -> Option<String> {
        self.usage.warning.clone()
    }

    pub fn project(
        &self,
        fallback_membership: Option<String>,
        fallback_email: Option<String>,
        profile_email: Option<String>,
    ) -> ProviderRecord {
        cursor::project(
            &self.usage.summary,
            fallback_membership.as_deref(),
            fallback_email.as_deref(),
            profile_email.as_deref(),
            self.usage.period_message.as_deref(),
            String::new(),
        )
        .into()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/cursor/tests.rs"]
mod tests;
