//! Assembles Codex analytics responses and advances account coverage.

use super::{
    AnalyticsSnapshot, CoverageScope, CreditEvent, Endpoint, Point, codex_credit_events, codex_document,
    codex_endpoint_batch,
};
use crate::utc_day::{midnight, retained_utc_days};
use crate::watermark::Watermarks;
use quotaline_text::text::humanize;

pub struct EndpointResponse {
    pub endpoint: Endpoint,
    pub result: Result<Vec<u8>, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexBatchPlan {
    pub start: String,
    pub end: String,
    pub starts: Vec<(Endpoint, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodexBatchResult {
    pub snapshot: Option<AnalyticsSnapshot>,
    pub warnings: Vec<String>,
}

impl CodexBatchPlan {
    pub fn new(watermarks: &mut Watermarks, now: f64, retention_days: u32) -> Self {
        let (start, end) = retained_utc_days(now, retention_days);
        Self {
            starts: watermarks.plan(&start, &end),
            start,
            end,
        }
    }
}

pub fn codex_batch(
    watermarks: &mut Watermarks,
    plan: &CodexBatchPlan,
    responses: Vec<EndpointResponse>,
    credit_response: Result<Vec<u8>, String>,
    account: String,
    now: f64,
    retention_days: u32,
) -> CodexBatchResult {
    let mut points = Vec::<Point>::new();
    let mut warnings = Vec::new();
    let mut successful = Vec::new();
    let mut covered_scopes = Vec::new();
    for (endpoint, start) in &plan.starts {
        let result = responses
            .iter()
            .find(|response| response.endpoint == *endpoint)
            .map(|response| response.result.as_ref().map_err(String::as_str));
        match result {
            Some(Ok(payload)) => match codex_document(payload)
                .and_then(|document| codex_endpoint_batch(*endpoint, &document, &account, &plan.start, &plan.end))
            {
                Ok(batch) => {
                    points.extend(batch.points);
                    successful.push(*endpoint);
                    covered_scopes.push(CoverageScope {
                        metrics: endpoint.metrics().iter().map(|metric| (*metric).into()).collect(),
                        start_day: start.clone(),
                        end_day: plan.end.clone(),
                    });
                }
                Err(_) => warnings.push(format!(
                    "{} analytics unavailable: Unexpected response: codex.{}: invalid analytics payload",
                    humanize(endpoint.name()),
                    endpoint.name()
                )),
            },
            Some(Err(error)) => warnings.push(format!("{} analytics unavailable: {error}", humanize(endpoint.name()))),
            None => warnings.push(format!(
                "{} analytics unavailable: response unavailable",
                humanize(endpoint.name())
            )),
        }
    }
    if !successful.is_empty() {
        watermarks.complete(&successful, &plan.end, now, retention_days);
    }
    let credit_events = match credit_response {
        Ok(payload) => match codex_document(&payload) {
            Ok(document) => {
                let mapped = codex_credit_events(&document);
                if mapped.missing_amount_count > 0 {
                    let noun = if mapped.missing_amount_count == 1 {
                        "event"
                    } else {
                        "events"
                    };
                    warnings.push(format!(
                        "Skipped {} Codex credit {noun} without a credit amount.",
                        mapped.missing_amount_count
                    ));
                }
                mapped.events
            }
            Err(_) => {
                warnings.push("Credit usage history unavailable: Unexpected response: codex.credit-events: invalid analytics payload".into());
                Vec::new()
            }
        },
        Err(error) => {
            warnings.push(format!("Credit usage history unavailable: {error}"));
            Vec::new()
        }
    };
    let from = midnight(&plan.start).expect("planned day is valid");
    let until = midnight(&plan.end).expect("planned day is valid") + 86_400.0;
    let credit_events: Vec<CreditEvent> = credit_events
        .into_iter()
        .filter(|event| event.timestamp >= from && event.timestamp < until)
        .collect();
    let snapshot =
        (!points.is_empty() || !credit_events.is_empty() || !covered_scopes.is_empty()).then_some(AnalyticsSnapshot {
            provider: "codex".into(),
            points,
            credit_events,
            fetched_at: now,
            account_fingerprint: Some(account),
            covered_scopes,
        });
    CodexBatchResult { snapshot, warnings }
}

#[cfg(test)]
#[path = "../tests/unit/codex_batch/tests.rs"]
mod tests;
