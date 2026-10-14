use super::*;
use serde_json::json;

fn session(now: f64) -> Watermarks {
    Watermarks::load(None, "account".into(), now, 7)
}

#[test]
fn plan_uses_utc_dates_and_existing_watermarks() {
    let now = 1_760_400_000.0;
    let mut watermarks = session(now);
    let plan = CodexBatchPlan::new(&mut watermarks, now, 7);
    assert_eq!(plan.start, "2025-10-08");
    assert_eq!(plan.end, "2025-10-14");
    assert_eq!(plan.starts.len(), Endpoint::ALL.len());
    assert!(plan.starts.iter().all(|(_, start)| start == &plan.start));
    assert_eq!(CodexBatchPlan::new(&mut watermarks, now, 0).start, "2025-10-14");
}

#[test]
fn batch_filters_dates_and_advances_only_successful_endpoints() {
    let now = 1_760_400_000.0;
    let mut watermarks = session(now);
    let plan = CodexBatchPlan::new(&mut watermarks, now, 7);
    let result = codex_batch(
        &mut watermarks,
        &plan,
        vec![
            EndpointResponse {
                endpoint: Endpoint::TokenUsage,
                result: Ok(serde_json::to_vec(&json!({"data":[
                    {"date":"2025-10-14","product_surface_usage_values":{"cli":30}},
                    {"date":"2025-10-01","product_surface_usage_values":{"cli":40}}
                ]}))
                .unwrap()),
            },
            EndpointResponse {
                endpoint: Endpoint::WorkspaceCounts,
                result: Err("HTTP 503".into()),
            },
            EndpointResponse {
                endpoint: Endpoint::Skills,
                result: Ok(b"invalid".to_vec()),
            },
            EndpointResponse {
                endpoint: Endpoint::Plugins,
                result: Ok(br#"{"data":null}"#.to_vec()),
            },
        ],
        Ok(serde_json::to_vec(&json!({"data":[
            {"date":"2025-10-14","usage_id":"inside","credit_amount":3},
            {"date":"2025-10-01","usage_id":"outside","credit_amount":2},
            {"date":"2025-10-14","usage_id":"missing"}
        ]}))
        .unwrap()),
        "account".into(),
        now,
        7,
    );
    let snapshot = result.snapshot.unwrap();
    assert_eq!(snapshot.points.len(), 1);
    assert_eq!(snapshot.points[0].value, 30.0);
    assert_eq!(snapshot.credit_events.len(), 1);
    assert_eq!(snapshot.credit_events[0].id, "inside");
    assert_eq!(snapshot.covered_scopes.len(), 1);
    assert_eq!(
        snapshot.covered_scopes[0].metrics,
        vec!["surfaceUsagePercent", "modelCredits"]
    );
    assert_eq!(snapshot.account_fingerprint.as_deref(), Some("account"));
    assert_eq!(result.warnings.len(), 5);
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("WorkspaceCounts analytics unavailable: HTTP 503"))
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("Skills analytics unavailable: Unexpected response"))
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("Plugins analytics unavailable: Unexpected response"))
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("CodeReview analytics unavailable: response unavailable"))
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning == "Skipped 1 Codex credit event without a credit amount.")
    );
    let next = CodexBatchPlan::new(&mut watermarks, now, 7);
    assert_ne!(next.starts[0].1, plan.start);
    assert_eq!(next.starts[1].1, plan.start);
}

#[test]
fn batch_keeps_credit_events_when_analytics_fail() {
    let now = 1_760_400_000.0;
    let mut watermarks = session(now);
    let plan = CodexBatchPlan::new(&mut watermarks, now, 7);
    let result = codex_batch(
        &mut watermarks,
        &plan,
        vec![],
        Ok(serde_json::to_vec(&json!({"data":[
            {"date":"2025-10-14","credit_amount":1},
            {"date":"2025-10-14"},
            {"date":"2025-10-14"}
        ]}))
        .unwrap()),
        "account".into(),
        now,
        7,
    );
    assert_eq!(result.snapshot.unwrap().credit_events.len(), 1);
    assert!(
        result
            .warnings
            .contains(&"Skipped 2 Codex credit events without a credit amount.".into())
    );
    assert_eq!(CodexBatchPlan::new(&mut watermarks, now, 7).starts[0].1, plan.start);
}

#[test]
fn batch_reports_credit_failures_without_empty_snapshot() {
    let now = 1_760_400_000.0;
    for (credit_response, expected) in [
        (Err("HTTP 500".into()), "Credit usage history unavailable: HTTP 500"),
        (
            Ok(b"not json".to_vec()),
            "Credit usage history unavailable: Unexpected response: codex.credit-events: invalid analytics payload",
        ),
    ] {
        let mut watermarks = session(now);
        let plan = CodexBatchPlan::new(&mut watermarks, now, 7);
        let result = codex_batch(
            &mut watermarks,
            &plan,
            vec![],
            credit_response,
            "account".into(),
            now,
            7,
        );
        assert_eq!(result.snapshot, None);
        assert!(result.warnings.contains(&expected.to_string()));
    }
}

#[test]
fn batch_accepts_empty_credit_history_without_a_credit_warning() {
    let now = 1_760_400_000.0;
    let mut watermarks = session(now);
    let plan = CodexBatchPlan::new(&mut watermarks, now, 7);
    let result = codex_batch(
        &mut watermarks,
        &plan,
        vec![],
        Ok(br#"{"data":[]}"#.to_vec()),
        "account".into(),
        now,
        7,
    );
    assert_eq!(result.snapshot, None);
    assert!(
        result
            .warnings
            .iter()
            .all(|warning| !warning.starts_with("Credit usage history"))
    );
    assert_eq!(result.warnings.len(), Endpoint::ALL.len());
}

#[test]
fn batch_covers_every_endpoint_name() {
    let now = 1_760_400_000.0;
    let mut watermarks = session(now);
    let plan = CodexBatchPlan::new(&mut watermarks, now, 7);
    let replies = Endpoint::ALL
        .into_iter()
        .map(|endpoint| EndpointResponse {
            endpoint,
            result: Err("offline".into()),
        })
        .collect();
    let result = codex_batch(
        &mut watermarks,
        &plan,
        replies,
        Err("offline".into()),
        "account".into(),
        now,
        7,
    );
    for name in ["TokenUsage", "WorkspaceCounts", "Skills", "Plugins", "CodeReview"] {
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.starts_with(&format!("{name} analytics")))
        );
    }
}
