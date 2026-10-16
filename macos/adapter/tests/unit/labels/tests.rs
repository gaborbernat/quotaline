//! macOS label bridge cases.

use super::*;

fn windows() -> Vec<LabelWindowInput> {
    vec![
        LabelWindowInput {
            provider: "claude".into(),
            id: "session".into(),
            label: "Session".into(),
            scope: None,
        },
        LabelWindowInput {
            provider: "codex".into(),
            id: "session".into(),
            label: "Session".into(),
            scope: None,
        },
    ]
}

#[test]
fn label_bridge_keeps_short_defaults() {
    assert_eq!(short_label_limit(), 6);
    assert_eq!(short_label_draft("A👨‍👩‍👧‍👦BCDEF".into()), "A👨‍👩‍👧‍👦BCDE");
    assert_eq!(short_label_override("  ".into(), "CX".into()), None);
    assert_eq!(
        short_label_override("SPARKLES".into(), "CX".into()).as_deref(),
        Some("SPARKL")
    );
    assert_eq!(
        default_window_short_label("claude".into(), "session".into(), "Session".into(), None).as_deref(),
        Some("CC 5h")
    );
}

#[test]
fn label_bridge_resolves_and_previews_conflicts() {
    let defaults = derived_window_labels(windows());
    assert_eq!(defaults.get("claude:session").map(String::as_str), Some("CC 5h"));
    let overrides = HashMap::from([("claude:session".into(), "CUSTOM".into())]);
    assert_eq!(
        resolved_window_labels(windows(), overrides)
            .get("claude:session")
            .map(String::as_str),
        Some("CUSTOM")
    );
    assert_eq!(
        preview_window_label(windows(), HashMap::new(), "claude:session".into(), "CX 5h".into()),
        Some(LabelPreview {
            override_value: Some("CX 5h".into()),
            conflict_key: Some("codex:session".into()),
        })
    );
    assert_eq!(
        preview_window_label(windows(), HashMap::new(), "missing".into(), "X".into()),
        None
    );
}

#[test]
fn label_bridge_merges_valid_drafts() {
    assert_eq!(
        valid_window_label_overrides(
            windows(),
            HashMap::new(),
            HashMap::from([
                ("claude:session".into(), "CX 5h".into()),
                ("codex:session".into(), "NEW".into())
            ])
        ),
        HashMap::from([("codex:session".into(), "NEW".into())])
    );
}
