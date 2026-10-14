//! Checks claude transcripts behavior through the public API.

use super::*;

#[test]
fn pricing_warnings_name_each_unpriced_model() {
    assert_eq!(pricing_warnings(&[]), Vec::<String>::new());
    assert_eq!(
        pricing_warnings(&["unknown-a".into(), "unknown-b".into()]),
        [
            "Cost for unknown-a is estimated; the pricing table has no entry",
            "Cost for unknown-b is estimated; the pricing table has no entry",
        ]
    );
}

#[test]
fn transcript_project_falls_back_to_directory_or_filename() {
    let root = Path::new("/home/me/.claude/projects");
    assert_eq!(encoded_project_name("-Users-me-repo"), "directory:-Users-me-repo");
    assert_eq!(
        project_name(&root.join("-Users-me-repo/nested/session.jsonl"), root),
        "directory:-Users-me-repo"
    );
    assert_eq!(project_name(&root.join("loose.jsonl"), root), "loose");
    assert_eq!(project_name(Path::new("/elsewhere/session.jsonl"), root), "session");
    assert_eq!(project_name(Path::new("/"), root), "");
}

#[test]
fn parses_assistant_usage_without_retaining_text() {
    let line = br#"{"type":"assistant","timestamp":"2026-10-09T12:00:00Z","sessionId":"session","cwd":"/work/app","requestId":"req","message":{"id":"msg","model":"claude-sonnet-5-5","usage":{"input_tokens":1000000,"output_tokens":1000000,"cache_creation_input_tokens":0,"cache_read_input_tokens":0},"content":[{"type":"text","text":"private text"},{"type":"tool_use","name":"Read"}]}}"#;
    let row = parse(line).unwrap();
    assert_eq!(row.id, "msg:req");
    assert_eq!(row.tool_calls, 1);
    assert_eq!(row.cost, Some(12.0));
    assert_eq!(row.project.as_deref(), Some("/work/app"));
}

#[test]
fn project_paths_are_normalized_before_history_storage() {
    for (cwd, expected) in [
        ("/work//./app/", "/work/app"),
        ("/work/cache/../app", "/work/app"),
        ("/../../", "/"),
        ("work/app", "work/app"),
    ] {
        let line = serde_json::json!({"type":"assistant","timestamp":"2026-10-09T12:00:00Z",
                "cwd":cwd,"message":{"model":"claude-sonnet-5-5","usage":{"input_tokens":1}}});
        assert_eq!(
            parse(line.to_string().as_bytes()).unwrap().project.as_deref(),
            Some(expected),
            "{cwd}"
        );
    }
}

#[test]
fn incomplete_assistant_events_are_ignored() {
    for (name, message, timestamp) in [
        ("message", serde_json::Value::Null, Some("2026-10-09T12:00:00Z")),
        (
            "usage",
            serde_json::json!({"model":"claude-sonnet-5-5"}),
            Some("2026-10-09T12:00:00Z"),
        ),
        (
            "model",
            serde_json::json!({"usage":{"input_tokens":1}}),
            Some("2026-10-09T12:00:00Z"),
        ),
        (
            "timestamp",
            serde_json::json!({"model":"claude-sonnet-5-5","usage":{"input_tokens":1}}),
            None,
        ),
        (
            "invalid timestamp",
            serde_json::json!({"model":"claude-sonnet-5-5","usage":{"input_tokens":1}}),
            Some("invalid"),
        ),
    ] {
        let line = serde_json::json!({"type":"assistant","timestamp":timestamp,"message":message});
        assert!(parse(line.to_string().as_bytes()).is_none(), "{name}");
    }
}

#[test]
fn reported_cost_overrides_price_and_haiku_uses_prompt_tier() {
    let base = r#"{"type":"assistant","timestamp":"2026-10-09T12:00:00Z","costUSD":0.5,"message":{"model":"claude-haiku-5-5","usage":{"input_tokens":100,"output_tokens":10}}}"#;
    assert_eq!(parse(base.as_bytes()).unwrap().cost, Some(0.5));
    let long = r#"{"type":"assistant","timestamp":"2026-10-09T12:00:00Z","message":{"model":"claude-haiku-5-5","usage":{"input_tokens":200000,"output_tokens":0}}}"#;
    assert_eq!(parse(long.as_bytes()).unwrap().cost, Some(0.1));
    assert!(parse(br#"{"type":"user"}"#).is_none());
}

#[test]
fn invalid_reported_cost_falls_back_to_estimate() {
    for cost in [serde_json::json!(-1.0), serde_json::Value::Null] {
        let line = serde_json::json!({
            "type":"assistant","timestamp":"2026-10-09T12:00:00Z","costUSD":cost,
            "message":{"model":"claude-sonnet-5-5","usage":{"input_tokens":1_000_000}}
        });
        assert_eq!(parse(line.to_string().as_bytes()).unwrap().cost, Some(2.0));
    }
}

#[test]
fn unknown_model_has_no_invented_cost() {
    let line = serde_json::json!({
        "type":"assistant","timestamp":"2026-10-09T12:00:00Z",
        "message":{"model":"claude-future-9","usage":{"input_tokens":1_000_000}}
    });
    assert_eq!(parse(line.to_string().as_bytes()).unwrap().cost, None);
}

#[test]
fn new_point_release_uses_latest_price_from_its_family() {
    let line = serde_json::json!({
        "type":"assistant","timestamp":"2026-10-09T12:00:00Z",
        "message":{"model":"claude-opus-6","usage":{"input_tokens":1_000_000,"output_tokens":1_000_000}}
    });
    assert_eq!(parse(line.to_string().as_bytes()).unwrap().cost, Some(24.0));
}

#[test]
fn negative_token_counts_do_not_reduce_local_usage() {
    let line = serde_json::json!({
        "type":"assistant","timestamp":"2026-10-09T12:00:00Z",
        "message":{"model":"claude-sonnet-5-5","usage":{
            "input_tokens":-10,"output_tokens":-20,
            "cache_creation_input_tokens":-30,"cache_read_input_tokens":-40
        }}
    });
    let record = parse(line.to_string().as_bytes()).unwrap();
    assert_eq!(
        (record.input, record.output, record.cache_write, record.cache_read),
        (0, 0, 0, 0)
    );
    assert_eq!(record.cost, Some(0.0));
}

#[test]
fn fast_mode_only_doubles_opus_cost_estimates() {
    for (model, expected) in [
        ("claude-opus-5-5", 48.0),
        ("claude-opus-4-8", 60.0),
        ("claude-sonnet-5-5", 12.0),
    ] {
        let line = serde_json::json!({
            "type":"assistant",
            "timestamp":"2026-10-09T12:00:00Z",
            "message":{"model":model,"usage":{"input_tokens":1_000_000,"output_tokens":1_000_000,"speed":"fast"}}
        });
        assert_eq!(
            parse(line.to_string().as_bytes()).unwrap().cost,
            Some(expected),
            "{model}"
        );
    }
}

#[test]
fn one_hour_cache_creation_uses_the_higher_write_price() {
    let line = serde_json::json!({
        "type":"assistant",
        "timestamp":"2026-10-09T12:00:00Z",
        "message":{"model":"claude-sonnet-5-5","usage":{"cache_creation_input_tokens":1_000_000,
            "cache_creation":{"ephemeral_1h_input_tokens":1_000_000}}}
    });
    assert_eq!(parse(line.to_string().as_bytes()).unwrap().cost, Some(4.0));
}

#[test]
fn pricing_prefers_the_longest_known_model_prefix() {
    let usage = TokenCounts {
        input: 1_000_000,
        output: 1_000_000,
        cache_write: 1_000_000,
        cache_read: 1_000_000,
        cache_write_one_hour: 0,
    };
    for (model, expected) in [
        ("claude-opus-5", 36.75),
        ("claude-haiku-4-5-20251001", 7.35),
        ("claude-sonnet-4-6", 22.05),
        ("Claude-Sonnet-5", 14.7),
        ("claude-fable-5-1", 72.75),
        ("claude-opus-5-5", 29.2),
        ("CLAUDE-OPUS-5-5-20260922", 29.2),
        ("claude-sonnet-5-5", 14.7),
        ("claude-sonnet-5-5-20260928", 14.7),
        ("claude-sonnet-4-5-20250929", 22.05),
        ("claude-opus-4-5-20251101", 36.75),
    ] {
        assert_eq!(cost(model, usage, None), Some(expected), "{model}");
    }
}

#[test]
fn unknown_models_use_the_newest_price_in_the_same_family() {
    assert_eq!(price("claude-opus-6", 0), price("claude-opus-5-5", 0));
    assert_eq!(price("claude-opus-4-1-20250805", 0), price("claude-opus-5-5", 0));
    for model in ["gpt-5", "unknown", "claude-zeta-1", "claude"] {
        assert_eq!(price(model, 0), None, "{model}");
    }
    assert_eq!(price("claude-fable-5", 0), price("claude-mythos-5", 0));
    assert_eq!(price("claude-fable-5-1", 0), price("claude-mythos-5-1", 0));
}

#[test]
fn cache_duration_tokens_cannot_exceed_total_cache_writes() {
    let line = serde_json::json!({
        "type":"assistant",
        "timestamp":"2026-10-09T12:00:00Z",
        "message":{"model":"claude-sonnet-5-5","usage":{"cache_creation_input_tokens":1_000_000,
            "cache_creation":{"ephemeral_1h_input_tokens":2_000_000}}}
    });
    assert_eq!(parse(line.to_string().as_bytes()).unwrap().cost, Some(4.0));
}
