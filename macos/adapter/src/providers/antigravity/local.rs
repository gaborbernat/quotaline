//! Probes an Antigravity language server for quota and identity.

use std::sync::Arc;

use quotaline_antigravity::request;

use super::local_callback::AntigravityServerScanner;
use super::{AntigravityError, AntigravityQuotaResponse};
use crate::http::{NativeHttpPostJson, NativeHttpTransport, native_http_post_json};
use crate::provider_record::ProviderRecord;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct AntigravityLocalServer {
    pub pid: i32,
    pub port: u16,
    pub csrf_token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct AntigravityLocalInput {
    pub port: u16,
    pub csrf_token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityLocalSnapshot {
    pub provider: ProviderRecord,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityLocalResult {
    pub snapshot: Option<Box<AntigravityLocalSnapshot>>,
    pub debug_messages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityLocalReading {
    pub snapshot: Option<Box<AntigravityLocalSnapshot>>,
    pub fetched_at: Option<f64>,
    pub debug_messages: Vec<String>,
}

#[derive(Default)]
struct LocalState {
    server: Option<AntigravityLocalServer>,
    snapshot: Option<(AntigravityLocalSnapshot, f64)>,
}

#[derive(Default, uniffi::Object)]
pub struct AntigravityLocalSession(std::sync::Mutex<LocalState>);

#[uniffi::export]
impl AntigravityLocalSession {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch(
        &self,
        now: f64,
        scanner: Arc<dyn AntigravityServerScanner>,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> AntigravityLocalReading {
        let cached_server = {
            let state = self.0.lock().unwrap();
            if let Some((snapshot, fetched_at)) =
                state.snapshot.as_ref().filter(|(_, fetched_at)| now - fetched_at < 5.0)
            {
                return AntigravityLocalReading {
                    snapshot: Some(Box::new(snapshot.clone())),
                    fetched_at: Some(*fetched_at),
                    debug_messages: vec![],
                };
            }
            state.server.clone()
        };
        let mut servers = cached_server
            .clone()
            .map_or_else(|| scanner.discover_servers(), |server| vec![server]);
        let mut debug_messages = vec![];
        let mut index = 0;
        while index < servers.len() {
            let server = servers[index].clone();
            index += 1;
            let reading = antigravity_local_probe(
                AntigravityLocalInput {
                    port: server.port,
                    csrf_token: server.csrf_token.clone(),
                },
                transport.clone(),
            )
            .await;
            debug_messages.extend(reading.debug_messages);
            if let Some(snapshot) = reading.snapshot {
                let mut state = self.0.lock().unwrap();
                state.server = Some(server);
                state.snapshot = Some((*snapshot.clone(), now));
                return AntigravityLocalReading {
                    snapshot: Some(snapshot),
                    fetched_at: Some(now),
                    debug_messages,
                };
            }
            if index == 1 && cached_server.is_some() {
                self.0.lock().unwrap().server = None;
                servers.extend(
                    scanner
                        .discover_servers()
                        .into_iter()
                        .filter(|candidate| candidate != &server),
                );
            }
        }
        AntigravityLocalReading {
            snapshot: None,
            fetched_at: None,
            debug_messages,
        }
    }
}

#[uniffi::export]
pub async fn antigravity_local_probe(
    input: AntigravityLocalInput,
    transport: Arc<dyn NativeHttpTransport>,
) -> AntigravityLocalResult {
    let headers = super::antigravity_server_headers(input.csrf_token);
    let mut debug_messages = vec![];
    for scheme in request::SERVER_SCHEMES {
        let quota = native_http_post_json(
            transport.clone(),
            NativeHttpPostJson {
                url: request::server_quota_url(scheme, input.port),
                headers: headers.clone(),
                operation: "antigravity.local-quota".into(),
                body: super::antigravity_server_quota_body(),
                timeout_millis: Some(request::SERVER_TIMEOUT_MILLIS),
            },
        )
        .await;
        let quota = match quota {
            Ok(payload) => match AntigravityQuotaResponse::new(payload) {
                Ok(quota) => quota,
                Err(AntigravityError::InvalidJson) => {
                    debug_messages.push(format!("antigravity: invalid local quota response port={}", input.port));
                    continue;
                }
                Err(_) => {
                    debug_messages.push(format!(
                        "antigravity: local quota response rejected port={}",
                        input.port
                    ));
                    return AntigravityLocalResult {
                        snapshot: None,
                        debug_messages,
                    };
                }
            },
            Err(error) => {
                debug_messages.push(format!(
                    "antigravity: language server port={} scheme={scheme} {}",
                    input.port,
                    quotaline_http::HttpError::from(error).message(None)
                ));
                continue;
            }
        };
        let status = native_http_post_json(
            transport,
            NativeHttpPostJson {
                url: request::server_status_url(scheme, input.port),
                headers,
                operation: "antigravity.local-status".into(),
                body: super::antigravity_server_status_body(),
                timeout_millis: Some(request::SERVER_TIMEOUT_MILLIS),
            },
        )
        .await;
        let identity = status
            .map_err(|error| quotaline_http::HttpError::from(error).message(None))
            .and_then(|payload| super::antigravity_identity(payload).map_err(|error| error.to_string()));
        let identity = match identity {
            Ok(identity) => identity,
            Err(message) => {
                debug_messages.push(format!(
                    "antigravity: user status unavailable port={} {message}",
                    input.port
                ));
                super::antigravity_default_identity()
            }
        };
        return AntigravityLocalResult {
            snapshot: Some(Box::new(AntigravityLocalSnapshot {
                provider: quota.provider(identity),
            })),
            debug_messages,
        };
    }
    AntigravityLocalResult {
        snapshot: None,
        debug_messages,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/providers/antigravity/local/tests.rs"]
mod tests;
