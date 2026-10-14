//! Fetches Antigravity cloud quota and plan details through a desktop's transport.

use serde_json::Value;

use super::{QuotaError, request, validate};
use quotaline_http::transport::{Request, Transport, header_list, json_document};
use quotaline_http::{HttpError, ProviderFailureKind};

#[derive(Debug, PartialEq)]
pub struct CloudQuota {
    pub result: Result<(Value, String), HttpError>,
    pub debug_messages: Vec<String>,
}

/// Tries each cloud endpoint in turn and returns the first readable quota with the endpoint that served it.
pub async fn fetch_quota(
    transport: &impl Transport,
    bases: &[&str],
    token: &str,
    hint: &str,
    home: Option<&str>,
) -> CloudQuota {
    let mut debug_messages = Vec::new();
    let mut last_error = None;
    for base in bases {
        let body = serde_json::to_vec(&request::cloud_quota_body()).expect("Antigravity quota body serializes");
        let quota = transport
            .send(Request::post_json(
                request::cloud_quota_url(base),
                header_list(request::cloud_headers(token)),
                "antigravity.quota",
                body,
            ))
            .await
            .and_then(|payload| json_document(&payload, "antigravity.quota"))
            .and_then(|document| validate(&document).map(|()| document).map_err(quota_error));
        match quota {
            Ok(document) => {
                return CloudQuota {
                    result: Ok((document, (*base).into())),
                    debug_messages,
                };
            }
            Err(error) if error.provider_failure(hint, home).kind == ProviderFailureKind::NotAuthenticated => {
                return CloudQuota {
                    result: Err(error),
                    debug_messages,
                };
            }
            Err(error) => {
                debug_messages.push(format!("antigravity: {base} {}", error.message(home)));
                last_error = Some(error);
            }
        }
    }
    CloudQuota {
        result: Err(last_error.expect("Antigravity has a cloud endpoint")),
        debug_messages,
    }
}

/// Returns the plan details, or the warning to show while they are unavailable.
pub async fn fetch_plan(
    transport: &impl Transport,
    base: &str,
    token: &str,
    home: Option<&str>,
) -> Result<Value, String> {
    let body = serde_json::to_vec(&request::cloud_assist_body()).expect("Antigravity assist body serializes");
    let payload = transport
        .send(Request::post_json(
            request::cloud_assist_url(base),
            header_list(request::cloud_headers(token)),
            "antigravity.load-code-assist",
            body,
        ))
        .await
        .map_err(|error| error.message(home))?;
    json_document(&payload, "antigravity.load-code-assist").map_err(|error| error.message(home))
}

fn quota_error(error: QuotaError) -> HttpError {
    match error {
        QuotaError::Unauthenticated => HttpError::Response {
            status: 401,
            body: "unauthenticated".into(),
            retry_after: None,
        },
        QuotaError::RpcFailure => HttpError::Decoding("Antigravity quota request failed at the RPC layer".into()),
        QuotaError::MissingGroups => HttpError::Decoding("Antigravity response contains no quota groups".into()),
    }
}

#[cfg(test)]
#[path = "../tests/unit/fetch/tests.rs"]
mod tests;
