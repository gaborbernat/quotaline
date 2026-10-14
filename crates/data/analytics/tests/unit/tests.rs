//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn endpoints_use_the_codex_daily_api_paths() {
    assert_eq!(
        Endpoint::ALL.map(Endpoint::path),
        [
            "wham/usage/daily-token-usage-breakdown",
            "wham/analytics/daily-workspace-usage-counts",
            "wham/analytics/daily-skill-usage-metrics",
            "wham/analytics/daily-plugin-usage-metrics",
            "wham/analytics/daily-code-review-metrics",
        ]
    );
}

#[test]
fn codex_daily_fixtures_cover_each_endpoint() {
    let fixtures = [
        (
            Endpoint::TokenUsage,
            include_str!("../fixtures/codex_daily_token_usage.json"),
        ),
        (
            Endpoint::WorkspaceCounts,
            include_str!("../fixtures/codex_daily_workspace_usage.json"),
        ),
        (Endpoint::Skills, include_str!("../fixtures/codex_daily_skills.json")),
        (Endpoint::Plugins, include_str!("../fixtures/codex_daily_plugins.json")),
        (
            Endpoint::CodeReview,
            include_str!("../fixtures/codex_daily_code_review.json"),
        ),
    ];
    for (endpoint, fixture) in fixtures {
        let payload: Value = serde_json::from_str(fixture).unwrap();
        let points = mapped_points(endpoint, &payload);
        assert_eq!(points.is_empty(), endpoint == Endpoint::CodeReview);
        assert!(points.iter().all(|point| point.provider == "codex"));
        assert!(
            points
                .iter()
                .all(|point| endpoint.metrics().contains(&point.metric.as_str()))
        );
    }
}

#[test]
fn codex_endpoint_batch_rejects_missing_data() {
    for document in [
        serde_json::json!({}),
        serde_json::json!({"data":null}),
        serde_json::json!({"data":{}}),
    ] {
        assert_eq!(
            codex_endpoint_batch(Endpoint::Skills, &document, "account", "2026-10-08", "2026-10-14").err(),
            Some(CodexPayloadError::MissingData)
        );
    }
}

#[test]
fn codex_endpoint_batch_keeps_only_points_inside_the_requested_days() {
    let document = serde_json::json!({"data":[
        {"date":"2026-10-07","skill_usage_overviews":[{"skill_name":"old","invocation_counts":2}]},
        {"date":"2026-10-14","skill_usage_overviews":[{"skill_name":"current","invocation_counts":3}]},
        {"date":"2026-10-15","skill_usage_overviews":[{"skill_name":"future","invocation_counts":4}]}
    ]});
    let batch = codex_endpoint_batch(Endpoint::Skills, &document, "account", "2026-10-08", "2026-10-14").unwrap();
    assert_eq!(batch.endpoint, Endpoint::Skills);
    assert_eq!(batch.account_fingerprint, "account");
    assert_eq!((batch.start.as_str(), batch.end.as_str()), ("2026-10-08", "2026-10-14"));
    assert_eq!(batch.points.len(), 1);
    assert_eq!(
        (batch.points[0].day.as_str(), batch.points[0].series.as_str()),
        ("2026-10-14", "current")
    );
}

#[test]
fn mapper_keeps_zero_and_combines_duplicate_plugins() {
    let payload = serde_json::json!({"data":[{"date":"2026-10-09","plugin_usage_overviews":[
        {"plugin_name":"first","invocation_counts":0}, {"plugin_name":"first","invocation_counts":"3"},
        {"display_name":"second","invocation_counts":2}
    ]}]});
    let points = mapped_points(Endpoint::Plugins, &payload);
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].value, 3.0);
    assert_eq!(points[1].value, 2.0);
    assert!(mapped_points(Endpoint::Plugins, &serde_json::json!({"data":[{"date":"invalid"}]})).is_empty());
    let reviews = mapped_points(
        Endpoint::CodeReview,
        &serde_json::json!({"data":[{"date":"2026-10-09","accepted":2,"rejected":0}]}),
    );
    assert_eq!(reviews.len(), 2);
}

#[test]
fn token_usage_ignores_missing_and_nonfinite_values() {
    let payload = serde_json::json!({"data":[{"date":"2026-10-09","models":[
        {"credits":7},{"model":"valid","credits":"2.5"},{"model":"bad","credits":"NaN"}
    ]}]});
    let points = mapped_points(Endpoint::TokenUsage, &payload);
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].series, "valid");
    assert_eq!(points[0].value, 2.5);
}

#[test]
fn workspace_counts_skip_entries_without_names() {
    let payload = serde_json::json!({"data":[{"date":"2026-10-09","models":[
            {"turns":4},{"model":"gpt-fixture","turns":3}
        ],"clients":[{"threads":2},{"client_id":"cli","threads":1}]}]});
    let points = mapped_points(Endpoint::WorkspaceCounts, &payload);
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].series, "model:gpt-fixture");
    assert_eq!(points[1].series, "surface:cli");
}

#[test]
fn skill_and_plugin_overviews_ignore_incomplete_rows() {
    let payload = serde_json::json!({"data":[{"date":"2026-10-09",
        "skill_usage_overviews":[{"invocation_counts":2},{"skill_name":"fallback","invocation_counts":"3"}],
        "plugin_usage_overviews":[{"plugin_name":"missing-count"},{"invocation_counts":2},
            {"display_name":"fallback","invocation_counts":4}]
    }]});
    let skills = mapped_points(Endpoint::Skills, &payload);
    let plugins = mapped_points(Endpoint::Plugins, &payload);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].value, 3.0);
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].series, "fallback");
}

#[test]
fn empty_code_review_days_still_define_their_metric() {
    assert!(mapped_points(Endpoint::CodeReview, &serde_json::json!({"data":[]})).is_empty());
    for (endpoint, expected) in [
        (Endpoint::TokenUsage, &["surfaceUsagePercent", "modelCredits"][..]),
        (
            Endpoint::WorkspaceCounts,
            &[
                "inputTokens",
                "cachedInputTokens",
                "outputTokens",
                "turns",
                "threads",
                "credits",
            ][..],
        ),
        (Endpoint::Skills, &["skillInvocations"][..]),
        (Endpoint::Plugins, &["pluginInvocations"][..]),
        (Endpoint::CodeReview, &["codeReviews"][..]),
    ] {
        assert_eq!(endpoint.metrics(), expected);
    }
}

#[test]
fn credit_events_accept_each_backend_field_and_skip_invalid_rows() {
    let payload = serde_json::json!({"data":[
        {"usage_id":"work","id":"old","date":"2026-08-01","product_surface":"CHATGPT","credit_amount":12.5},
        {"id":"e2","created_at":"2026-08-02T10:00:00Z","service":"Review","credits_used":1.5},
        {"timestamp":"2026-08-03T00:00:00.500Z","product":"CLI","credits":2},
        {"date":"2026-08-04","amount":"3.25"},
        {"date":"2026-08-05","credits_used":0},
        {"date":"2026-08-06"},
        {"date":"2026-08-07","amount":"NaN"},
        {"date":"bad","credits_used":1},
        {"credits_used":1}
    ]});
    let mapping = codex_credit_events(&payload);
    assert_eq!(mapping.missing_amount_count, 2);
    assert_eq!(mapping.events.len(), 5);
    assert_eq!(
        mapping.events.iter().map(|event| event.id.as_str()).collect::<Vec<_>>(),
        [
            "work",
            "e2",
            "2026-08-03T00:00:00.500Z-2",
            "2026-08-04-3",
            "2026-08-05-4"
        ]
    );
    assert_eq!(
        mapping
            .events
            .iter()
            .map(|event| event.service.as_str())
            .collect::<Vec<_>>(),
        ["CHATGPT", "Review", "CLI", "Codex", "Codex"]
    );
    assert_eq!(
        mapping
            .events
            .iter()
            .map(|event| event.credits_used)
            .collect::<Vec<_>>(),
        [12.5, 1.5, 2.0, 3.25, 0.0]
    );
    assert_eq!(mapping.events[0].timestamp, 1_785_542_400.0);
    assert_eq!(mapping.events[2].timestamp.fract(), 0.5);
}

#[test]
fn missing_credit_event_data_is_empty() {
    assert_eq!(
        codex_credit_events(&serde_json::json!({})),
        CreditEvents {
            events: vec![],
            missing_amount_count: 0,
        }
    );
}

#[test]
fn codex_analytics_query_matches_each_endpoint() {
    for (endpoint, extra) in [
        (Endpoint::TokenUsage, vec![]),
        (Endpoint::WorkspaceCounts, vec![("workspace_user", "true")]),
        (
            Endpoint::Skills,
            vec![("workspace_user", "true"), ("top_skill_limit", "20")],
        ),
        (
            Endpoint::Plugins,
            vec![("workspace_user", "true"), ("top_plugin_limit", "20")],
        ),
        (Endpoint::CodeReview, vec![("workspace_user", "true")]),
    ] {
        let expected = [
            ("start_date", "2026-10-01"),
            ("end_date", "2026-10-13"),
            ("group_by", "day"),
        ]
        .into_iter()
        .chain(extra)
        .map(|(key, value)| (key, value.to_string()))
        .collect::<Vec<_>>();
        assert_eq!(endpoint.query("2026-10-01", "2026-10-13"), expected, "{endpoint:?}");
    }
}

fn mapped_points(endpoint: Endpoint, document: &Value) -> Vec<Point> {
    codex_endpoint_batch(endpoint, document, "test", "2020-01-01", "2030-12-31")
        .unwrap()
        .points
}
