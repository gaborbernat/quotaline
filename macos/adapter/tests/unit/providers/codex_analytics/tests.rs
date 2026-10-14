use super::*;

#[test]
fn endpoint_spec_exposes_shared_path_and_query() {
    let spec = codex_analytics_spec(Endpoint::Skills, "start".into(), "end".into());
    assert_eq!(spec.path, "wham/analytics/daily-skill-usage-metrics");
    assert_eq!(
        spec.query,
        [
            ("start_date", "start"),
            ("end_date", "end"),
            ("group_by", "day"),
            ("workspace_user", "true"),
            ("top_skill_limit", "20"),
        ]
        .map(|(name, value)| CodexQueryParameter {
            name: name.into(),
            value: value.into(),
        })
    );
}

#[test]
fn endpoint_list_follows_shared_order() {
    assert_eq!(
        codex_analytics_endpoints(),
        vec![
            Endpoint::TokenUsage,
            Endpoint::WorkspaceCounts,
            Endpoint::Skills,
            Endpoint::Plugins,
            Endpoint::CodeReview,
        ]
    );
}
