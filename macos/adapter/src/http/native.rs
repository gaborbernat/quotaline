//! Completes native HTTP requests without blocking the Rust future.

use super::{HttpErrorRecord, NativeHttpTransport};
use futures_channel::oneshot;
use quotaline_http::HttpError;
use quotaline_http::transport::Body;
use quotaline_http::transport::Request;
use quotaline_http::transport::Transport;
use std::future::Future;
use std::sync::{Arc, Mutex};

type HttpResult = Result<Vec<u8>, HttpErrorRecord>;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NativeHttpGet {
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub operation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NativeHttpPostJson {
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub operation: String,
    pub body: Vec<u8>,
    pub timeout_millis: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NativeHttpPostForm {
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub operation: String,
    pub fields: std::collections::HashMap<String, String>,
}

#[derive(uniffi::Object)]
pub struct NativeHttpCompletion(Mutex<Option<oneshot::Sender<HttpResult>>>);

#[uniffi::export]
impl NativeHttpCompletion {
    pub fn succeed(&self, data: Vec<u8>) {
        if let Some(sender) = self.0.lock().unwrap().take() {
            let _ = sender.send(Ok(data));
        }
    }

    pub fn fail(&self, error: HttpErrorRecord) {
        if let Some(sender) = self.0.lock().unwrap().take() {
            let _ = sender.send(Err(error));
        }
    }
}

pub async fn get(transport: Arc<dyn NativeHttpTransport>, request: NativeHttpGet) -> HttpResult {
    execute(transport, |transport, completion| {
        transport.start_get(request, completion)
    })
    .await
}

pub async fn post_json(transport: Arc<dyn NativeHttpTransport>, request: NativeHttpPostJson) -> HttpResult {
    execute(transport, |transport, completion| {
        transport.start_post_json(request, completion)
    })
    .await
}

pub async fn post_form(transport: Arc<dyn NativeHttpTransport>, request: NativeHttpPostForm) -> HttpResult {
    execute(transport, |transport, completion| {
        transport.start_post_form(request, completion)
    })
    .await
}

async fn execute(
    transport: Arc<dyn NativeHttpTransport>,
    start: impl FnOnce(Arc<dyn NativeHttpTransport>, Arc<NativeHttpCompletion>),
) -> HttpResult {
    let (sender, receiver) = oneshot::channel();
    start(transport, Arc::new(NativeHttpCompletion(Mutex::new(Some(sender)))));
    receiver.await.unwrap_or_else(|_| {
        Err(HttpErrorRecord::Network {
            detail: "request canceled".into(),
        })
    })
}

/// Sends shared provider pipeline requests through URLSession.
pub struct NativeTransport(pub Arc<dyn NativeHttpTransport>);

impl Transport for NativeTransport {
    fn send(&self, request: Request) -> impl Future<Output = Result<Vec<u8>, HttpError>> + Send {
        let transport = self.0.clone();
        async move {
            let headers = request.headers.into_iter().collect();
            let result = match request.body {
                Body::Empty => {
                    get(
                        transport,
                        NativeHttpGet {
                            url: request.url,
                            headers,
                            operation: request.operation,
                        },
                    )
                    .await
                }
                Body::Json(body) => {
                    post_json(
                        transport,
                        NativeHttpPostJson {
                            url: request.url,
                            headers,
                            operation: request.operation,
                            body,
                            timeout_millis: request.timeout_millis,
                        },
                    )
                    .await
                }
                Body::Form(fields) => {
                    post_form(
                        transport,
                        NativeHttpPostForm {
                            url: request.url,
                            headers,
                            operation: request.operation,
                            fields: fields.into_iter().collect(),
                        },
                    )
                    .await
                }
            };
            result.map_err(Into::into)
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/http/native/tests.rs"]
mod tests;
