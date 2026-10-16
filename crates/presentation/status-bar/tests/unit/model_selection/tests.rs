//! Checks how the stored selection mode chooses panel windows.

use super::*;

#[test]
fn selecting_no_limits_keeps_the_panel_empty() {
    let mut settings = Settings::default();
    settings.set("selected_windows", serde_json::json!([])).unwrap();
    assert_eq!(settings.has_custom_selection, Some(true));
    assert!(selected_window_keys(&settings, &demo()).is_empty());
}

#[test]
fn reverting_to_default_selection_restores_the_panel() {
    let mut settings = Settings::default();
    settings.set("selected_windows", serde_json::json!([])).unwrap();
    settings.set("has_custom_selection", serde_json::json!(false)).unwrap();
    assert!(!selected_window_keys(&settings, &demo()).is_empty());
}

#[test]
fn demo_selects_gemini_antigravity_limits_by_default() {
    let providers = demo();
    assert!(
        default_selected_windows(&providers)
            .windows(2)
            .any(|selection| selection == ["antigravity:gemini:session", "antigravity:gemini:weekly"])
    );
}
