//! Maps Copilot responses through the shared projector.

use quotaline_copilot as copilot;
use quotaline_copilot::request;
use std::sync::Arc;

use crate::http::{NativeHttpTransport, NativeTransport, ProviderFailure};
use crate::provider_record::ProviderRecord;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CopilotRequest {
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum CopilotFetch {
    Provider(Box<ProviderRecord>),
    InvalidHost,
    Failure(ProviderFailure),
}

#[uniffi::export]
pub fn copilot_request(host: String, token: String) -> Option<CopilotRequest> {
    let host = quotaline_credentials::copilot_host(&host)?;
    Some(CopilotRequest {
        url: request::user_url(&host),
        headers: request::headers(&token)
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect(),
    })
}

#[uniffi::export]
pub async fn copilot_fetch(
    host: String,
    token: String,
    user: Option<String>,
    hint: String,
    home: String,
    transport: Arc<dyn NativeHttpTransport>,
) -> CopilotFetch {
    let Some(request) = copilot_request(host, token.clone()) else {
        return CopilotFetch::InvalidHost;
    };
    match copilot::fetch::fetch_user(&NativeTransport(transport), &request.url, &token).await {
        Ok(document) => CopilotFetch::Provider(Box::new(
            copilot::project(&document, String::new(), user.unwrap_or_default()).into(),
        )),
        Err(error) => CopilotFetch::Failure(error.provider_failure(&hint, Some(&home))),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/copilot/tests.rs"]
mod tests;
