//! Checks structured, legacy, and empty Claude quota responses.

use super::*;

#[test]
fn structured_limits_keep_ids_groups_scopes_and_activity() {
    let provider = projection(
        json!({"limits":[
            {"percent":20,"kind":"session","resets_at":"2026-10-10T12:00:00Z"},
            {"percent":30,"kind":"weekly_all"},
            {"percent":40,"kind":"weekly_scoped","scope":{"model":{"display_name":"Opus 4"}},"is_active":false},
            {"percent":50,"kind":"surface_limit","scope":{"surface":"Code Review"},"group":"monthly"},
            {"percent":60,"kind":"daily_limit","group":"session"},
            {"percent":70,"kind":"other_limit","group":"weekly"},
            {"percent":80,"kind":"new_limit","group":"strange"}
        ]}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(
        provider.windows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        [
            "session",
            "weekly",
            "weekly:opus-4",
            "surface_limit:code-review",
            "daily_limit",
            "other_limit",
            "new_limit"
        ]
    );
    assert_eq!(
        provider
            .windows
            .iter()
            .map(|row| row.group.as_str())
            .collect::<Vec<_>>(),
        ["session", "weekly", "weekly", "monthly", "session", "weekly", "other"]
    );
    assert_eq!(provider.windows[2].scope.as_deref(), Some("Opus 4"));
    assert!(!provider.windows[2].active);
    assert_eq!(provider.windows[3].label, "Surface Limit Code Review");
    assert_eq!(provider.windows[0].duration_seconds, Some(18_000));
    assert_eq!(provider.windows[1].duration_seconds, Some(604_800));
    assert_eq!(provider.windows[3].duration_seconds, None);
}

#[test]
fn missing_scope_uses_a_stable_scoped_id() {
    let provider = projection(
        json!({"limits":[{"percent":1,"kind":"weekly_scoped"}]}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows[0].id, "weekly:scoped");
    assert_eq!(provider.windows[0].label, "Scoped weekly");
}

#[test]
fn scoped_id_normalizes_leading_and_repeated_delimiters() {
    let provider = projection(
        json!({"limits":[{"percent":1,"kind":"weekly_scoped","scope":{"model":{"display_name":"!Opus  4"}}}]}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows[0].id, "weekly:opus-4");
}

#[test]
fn malformed_structured_limits_are_skipped_individually() {
    let provider = projection(
        json!({"limits":[{"percent":"bad","kind":"session"},{"percent":20},{"percent":"30","kind":"session"}]}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows.len(), 1);
    assert_eq!(provider.windows[0].used, 30.0);
}

#[test]
fn structured_windows_replace_matching_legacy_windows() {
    let provider = projection(
        json!({"limits":[{"percent":42,"kind":"session"},{"percent":43,"kind":"weekly_all"},{"percent":44,"kind":"weekly_scoped","scope":{"model":{"display_name":"Opus"}}}],"five_hour":{"utilization":90},"seven_day":{"utilization":91},"seven_day_opus":{"utilization":92}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(
        provider.windows.iter().map(|row| row.used).collect::<Vec<_>>(),
        [42.0, 43.0, 44.0]
    );
}

#[test]
fn legacy_model_buckets_keep_known_and_new_windows() {
    let provider = projection(
        json!({"five_hour":{"utilization":10},"seven_day":{"utilization":20},"seven_day_opus":{"utilization":30},"seven_day_sonnet":{"utilization":40},"seven_day_oauth_apps":{"utilization":50},"seven_day_cowork":{"utilization":60},"seven_day_new_model":{"utilization":70},"other_bucket":{"utilization":80}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows.len(), 8);
    assert_eq!(provider.windows[0].id, "session");
    assert_eq!(
        provider
            .windows
            .iter()
            .find(|row| row.id == "seven_day_new_model")
            .unwrap()
            .group,
        "weekly"
    );
    assert_eq!(
        provider
            .windows
            .iter()
            .find(|row| row.id == "other_bucket")
            .unwrap()
            .group,
        "other"
    );
}

#[test]
fn unknown_bucket_does_not_repeat_a_structured_scope() {
    let provider = projection(
        json!({"limits":[{"percent":20,"kind":"weekly_scoped","scope":{"surface":"New Model"}}],"seven_day_new_model":{"utilization":30}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows.len(), 1);
}

#[test]
fn full_window_emits_a_reset_aware_notice() {
    let provider = projection(
        json!({"five_hour":{"utilization":100,"resets_at":"2026-10-10T12:00:00Z"}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.notices.len(), 1);
    assert_eq!(provider.notices[0].window_id.as_deref(), Some("session"));
    assert_eq!(provider.notices[0].resets_at.as_deref(), Some("2026-10-10T12:00:00Z"));
}

#[test]
fn explicitly_empty_usage_is_a_readable_response() {
    for data in [
        json!({"limits":[]}),
        json!({"limits":null}),
        json!({"spend":null}),
        json!({"five_hour":null}),
        json!({"extra_usage":null}),
    ] {
        assert!(projection(data, Value::Null, None).is_ok());
    }
}

#[test]
fn unreadable_usage_is_rejected() {
    for data in [
        json!({}),
        json!({"limits":"bad"}),
        json!({"spend":"bad"}),
        json!({"extra_usage":[]}),
        json!({"five_hour":{"utilization":"bad"}}),
        json!({"other_bucket":{"utilization":null}}),
        json!(null),
        json!({"error":{"message":"denied"}}),
        json!({"limits":[false],"five_hour":null}),
    ] {
        assert_eq!(
            projection(data, Value::Null, None).err(),
            Some(ProjectionError::NoReadableQuotas)
        );
    }
}

#[test]
fn malformed_field_prevents_an_empty_quota_response_from_appearing_valid() {
    assert_eq!(
        projection(json!({"five_hour": null, "spend": "bad"}), Value::Null, None).err(),
        Some(ProjectionError::NoReadableQuotas)
    );
}

#[test]
fn unknown_field_without_utilization_does_not_invalidate_an_empty_quota_response() {
    let provider = projection(json!({"five_hour": null, "other": {}}), Value::Null, None).unwrap();
    assert!(provider.windows.is_empty());
    assert!(provider.warnings.is_empty());
}

#[test]
fn readable_windows_keep_warnings_for_unreadable_fields() {
    let provider = projection(
        json!({"five_hour":{"utilization":52},"spend":123,"limits":[{"kind":"session","percent":"bad"}]}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows[0].used, 52.0);
    assert_eq!(
        provider.warnings,
        [
            "Claude returned unreadable fields: spend. Other reported values are shown.",
            "Skipped 1 malformed Claude usage limit."
        ]
    );
}

#[test]
fn multiple_dropped_limits_use_plural_warning() {
    let provider = projection(
        json!({"limits":[null,{"kind":"session"}],"seven_day":{"utilization":10}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.warnings, ["Skipped 2 malformed Claude usage limits."]);
}
