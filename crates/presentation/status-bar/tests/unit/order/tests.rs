//! Provider and model ordering cases.

use super::*;
use crate::{Provider, Settings, Window};

fn draft() -> OrderDraft {
    OrderDraft::new(
        &["claude".into(), "codex".into(), "claude".into(), "unknown".into()],
        &["claude:b".into(), "codex:a".into(), "claude:b".into()],
        &["claude".into(), "codex".into(), "gemini".into()],
        &["claude:a".into(), "claude:b".into(), "codex:a".into()],
    )
}

#[test]
fn order_from_state_uses_provider_and_model_preferences() {
    let providers = BTreeMap::from([(
        "codex".into(),
        Provider::connected(
            "codex",
            "Pro".into(),
            "test".into(),
            String::new(),
            vec![Window::new("weekly", "Weekly", 12.0, None, "weekly")],
        ),
    )]);
    let settings = Settings {
        provider_order: vec!["codex".into(), "claude".into()],
        model_order: vec!["codex:weekly".into()],
        ..Settings::default()
    };

    let order = OrderDraft::from_state(&settings, &providers);
    assert_eq!(order.providers[..2], ["codex", "claude"]);
    assert_eq!(order.models, ["codex:weekly"]);
}

#[test]
fn order_normalizes_available_provider_and_model_keys() {
    let order = draft();
    assert_eq!(order.providers, ["claude", "codex", "gemini"]);
    assert_eq!(order.models, ["claude:b", "claude:a", "codex:a"]);
    assert_eq!(
        order.ordered_selection(&["codex:a".into(), "claude:a".into()]),
        ["claude:a", "codex:a"]
    );
}

#[test]
fn provider_moves_keep_models_grouped() {
    let mut order = draft();
    order.move_provider("codex", "claude", false);
    assert_eq!(order.providers, ["codex", "claude", "gemini"]);
    assert_eq!(order.models, ["claude:b", "claude:a", "codex:a"]);
    order.move_provider_by("codex", 2);
    assert_eq!(order.providers, ["claude", "gemini", "codex"]);
    order.move_provider_by("missing", -1);
    order.move_provider_by("codex", 3);
    assert_eq!(order.providers, ["claude", "gemini", "codex"]);
}

#[test]
fn swapping_visible_providers_keeps_hidden_positions() {
    let mut order = draft();
    order.swap_provider("claude", "gemini");
    assert_eq!(order.providers, ["gemini", "codex", "claude"]);
    order.swap_provider("missing", "gemini");
    assert_eq!(order.providers, ["gemini", "codex", "claude"]);
}

#[test]
fn model_moves_stay_within_provider() {
    let mut order = draft();
    order.move_model("claude:a", "claude:b", false);
    assert_eq!(order.models, ["claude:a", "claude:b", "codex:a"]);
    order.move_model_by("claude:a", 1);
    assert_eq!(order.models, ["claude:b", "claude:a", "codex:a"]);
    order.move_model("claude:a", "codex:a", false);
    order.move_model_by("missing", 1);
    order.move_model_by("claude:a", 2);
    assert_eq!(order.models, ["claude:b", "claude:a", "codex:a"]);
}

#[test]
fn no_op_moves_preserve_order() {
    let mut order = draft();
    let expected = order.clone();
    order.move_provider("claude", "claude", true);
    order.move_provider("missing", "claude", false);
    order.move_provider("claude", "missing", false);
    order.move_provider_by("claude", -1);
    order.move_model("claude:a", "claude:a", false);
    order.move_model_by("claude:b", -1);
    assert_eq!(order, expected);
}

#[test]
fn model_key_without_separator_uses_itself_as_provider() {
    let mut order = OrderDraft::new(
        &[],
        &[],
        &["standalone".into()],
        &["standalone".into(), "other:a".into()],
    );
    assert_eq!(order.models, ["standalone"]);
    order.move_model("standalone", "other:a", false);
    assert_eq!(order.models, ["standalone"]);
}
