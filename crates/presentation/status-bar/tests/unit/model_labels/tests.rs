//! Checks short-label previews through the shared model API.

use super::*;

#[test]
fn label_maps_keep_defaults_and_apply_valid_overrides() {
    let overrides = BTreeMap::from([("claude:session".into(), "CUSTOM".into())]);
    let (defaults, resolved) = short_label_maps(&demo(), &overrides);
    assert_eq!(defaults.get("claude:session").map(String::as_str), Some("CC 5h"));
    assert_eq!(resolved.get("claude:session").map(String::as_str), Some("CUSTOM"));
    assert_eq!(resolved.get("codex:session"), defaults.get("codex:session"));
}

#[test]
fn label_maps_ignore_unknown_provider_snapshots() {
    let mut providers = demo();
    let mut unknown = Provider::empty("unknown", "connected");
    unknown
        .windows
        .push(Window::new("session", "Session", 20.0, None, "session"));
    providers.insert("unknown".into(), unknown);
    let (defaults, resolved) = short_label_maps(&providers, &BTreeMap::new());
    assert!(!defaults.contains_key("unknown:session"));
    assert_eq!(resolved, defaults);
}

#[test]
fn preview_rejects_a_default_label_owned_by_another_window() {
    let preview = preview_short_label(&demo(), &BTreeMap::new(), "claude:session", "CX 5h").unwrap();
    assert_eq!(preview.override_value.as_deref(), Some("CX 5h"));
    assert_eq!(preview.conflict_key.as_deref(), Some("codex:session"));
}

#[test]
fn preview_rejects_an_existing_override_without_case_sensitivity() {
    let overrides = BTreeMap::from([("claude:session".into(), "TEST12".into())]);
    let preview = preview_short_label(&demo(), &overrides, "codex:session", "test12").unwrap();
    assert_eq!(preview.conflict_key.as_deref(), Some("claude:session"));
}

#[test]
fn preview_restores_the_derived_default_for_an_empty_or_matching_draft() {
    for draft in ["", "CC 5h"] {
        let preview = preview_short_label(&demo(), &BTreeMap::new(), "claude:session", draft).unwrap();
        assert_eq!(preview.override_value, None);
        assert_eq!(preview.conflict_key, None);
    }
}

#[test]
fn preview_limits_an_override_to_six_graphemes() {
    let preview = preview_short_label(&demo(), &BTreeMap::new(), "claude:session", "A👨‍👩‍👧‍👦BCDEF").unwrap();
    assert_eq!(preview.override_value.as_deref(), Some("A👨‍👩‍👧‍👦BCDE"));
    assert_eq!(preview.conflict_key, None);
}

#[test]
fn preview_rejects_unknown_window_keys() {
    assert_eq!(preview_short_label(&demo(), &BTreeMap::new(), "missing", "NEW"), None);
}

#[test]
fn preview_accepts_an_unrelated_override_while_a_legacy_conflict_exists() {
    let overrides = BTreeMap::from([("claude:session".into(), "CX 5h".into())]);
    let preview = preview_short_label(&demo(), &overrides, "codex:weekly", "WEEK").unwrap();
    assert_eq!(preview.override_value.as_deref(), Some("WEEK"));
    assert_eq!(preview.conflict_key, None);
}
