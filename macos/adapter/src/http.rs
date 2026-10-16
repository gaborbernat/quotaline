//! Exposes shared provider HTTP failure policy to URLSession.

use quotaline_http as http;

mod native;
mod native_callback;
pub use native::{NativeHttpCompletion, NativeHttpGet, NativeHttpPostForm, NativeHttpPostJson, NativeTransport};
pub(crate) use native::{post_form as native_http_post_form, post_json as native_http_post_json};
pub use native_callback::NativeHttpTransport;
pub use quotaline_http::{ProviderFailure, ProviderFailureKind};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct HttpOrigin {
    pub scheme: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
}

impl HttpOrigin {
    fn borrowed(&self) -> http::Origin<'_> {
        http::Origin {
            scheme: self.scheme.as_deref(),
            host: self.host.as_deref(),
            port: self.port,
        }
    }
}

#[uniffi::export]
pub fn http_same_origin(source: HttpOrigin, destination: HttpOrigin) -> bool {
    http::same_origin(source.borrowed(), destination.borrowed())
}

#[uniffi::export]
pub fn http_redacted_endpoint(value: String) -> String {
    http::redacted_endpoint(&value)
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum HttpErrorRecord {
    Response {
        status: i32,
        body: String,
        retry_after: Option<f64>,
    },
    VendorSecurityCheck {
        status: i32,
    },
    Network {
        detail: String,
    },
    Decoding {
        detail: String,
    },
}

impl From<HttpErrorRecord> for http::HttpError {
    fn from(error: HttpErrorRecord) -> Self {
        match error {
            HttpErrorRecord::Response {
                status,
                body,
                retry_after,
            } => Self::Response {
                status,
                body,
                retry_after,
            },
            HttpErrorRecord::VendorSecurityCheck { status } => Self::VendorSecurityCheck(status),
            HttpErrorRecord::Network { detail } => Self::Network(detail),
            HttpErrorRecord::Decoding { detail } => Self::Decoding(detail),
        }
    }
}

#[uniffi::export]
pub fn http_response_failure(
    status: u16,
    content_type: Option<String>,
    retry_header: Option<String>,
    body: Vec<u8>,
    now: f64,
) -> Option<http::ResponseFailure> {
    http::response_failure(status, content_type.as_deref(), retry_header.as_deref(), &body, now)
}

#[uniffi::export]
pub fn http_error_message(error: HttpErrorRecord, home: Option<String>) -> String {
    http::HttpError::from(error).message(home.as_deref())
}

#[uniffi::export]
pub fn http_provider_failure(error: HttpErrorRecord, hint: String, home: Option<String>) -> http::ProviderFailure {
    http::HttpError::from(error).provider_failure(&hint, home.as_deref())
}

#[uniffi::export]
pub fn http_authentication_failure(status: u16, body: Vec<u8>) -> bool {
    http::authentication_failure(status, &body)
}

#[cfg(test)]
#[path = "../tests/unit/http/tests.rs"]
mod tests;
