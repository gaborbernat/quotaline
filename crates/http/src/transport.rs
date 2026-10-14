//! Lets shared provider pipelines send requests through each desktop's own HTTP client.

use std::future::Future;

use serde_json::Value;

use super::HttpError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    Empty,
    Json(Vec<u8>),
    Form(Vec<(String, String)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub operation: String,
    pub body: Body,
    pub timeout_millis: Option<u32>,
}

impl Request {
    pub fn get(url: impl Into<String>, headers: Vec<(String, String)>, operation: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers,
            operation: operation.into(),
            body: Body::Empty,
            timeout_millis: None,
        }
    }

    pub fn post_json(
        url: impl Into<String>,
        headers: Vec<(String, String)>,
        operation: impl Into<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            body: Body::Json(body),
            ..Self::get(url, headers, operation)
        }
    }
}

pub trait Transport {
    /// Sends one request and returns the body of a successful response.
    fn send(&self, request: Request) -> impl Future<Output = Result<Vec<u8>, HttpError>> + Send;
}

pub fn header_list<const N: usize>(headers: [(&'static str, String); N]) -> Vec<(String, String)> {
    headers.into_iter().map(|(name, value)| (name.into(), value)).collect()
}

/// Reads a successful response as JSON, naming the operation when it is not.
pub fn json_document(payload: &[u8], operation: &str) -> Result<Value, HttpError> {
    serde_json::from_slice(payload).map_err(|_| HttpError::Decoding(format!("{operation}: invalid payload")))
}
