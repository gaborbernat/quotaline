//! Exercises immutable order drafts through the UniFFI boundary.

use super::*;

#[test]
fn draft_normalizes_provider_order() {
    assert_eq!(draft().providers(), ["claude", "codex"]);
}

#[test]
fn draft_normalizes_model_order() {
    assert_eq!(draft().models(), ["claude:b", "claude:a", "codex:a"]);
}

#[test]
fn moving_provider_preserves_original_draft() {
    let original = draft();
    let moved = original.move_provider("codex".into(), "claude".into(), false);
    assert_eq!(moved.providers(), ["codex", "claude"]);
    assert_eq!(original.providers(), ["claude", "codex"]);
}

#[test]
fn moving_model_preserves_original_draft() {
    let original = draft();
    let moved = original.move_model("claude:a".into(), "claude:b".into(), false);
    assert_eq!(moved.models(), ["claude:a", "claude:b", "codex:a"]);
    assert_eq!(original.models(), ["claude:b", "claude:a", "codex:a"]);
}

#[test]
fn moving_provider_by_offset_updates_order() {
    let moved = draft().move_provider_by("claude".into(), 1);
    assert_eq!(moved.providers(), ["codex", "claude"]);
}

#[test]
fn moving_model_by_offset_updates_order() {
    let moved = draft().move_model_by("claude:b".into(), 1);
    assert_eq!(moved.models(), ["claude:a", "claude:b", "codex:a"]);
}

#[test]
fn selection_uses_draft_order() {
    assert_eq!(
        draft().ordered_selection(vec!["codex:a".into(), "claude:a".into()]),
        ["claude:a", "codex:a"]
    );
}

fn draft() -> RustOrderDraft {
    RustOrderDraft::new(
        vec!["claude".into()],
        vec!["claude:b".into()],
        vec!["claude".into(), "codex".into()],
        vec!["claude:a".into(), "claude:b".into(), "codex:a".into()],
    )
}
