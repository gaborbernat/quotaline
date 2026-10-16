//! Checks the macOS metric catalog boundary.

use super::*;

#[test]
fn boundary_exposes_metric_behavior_and_labels() {
    let definitions = history_metric_definitions();
    assert_eq!(definitions.len(), 20);
    let tokens = definitions
        .iter()
        .find(|definition| definition.id == "inputTokens")
        .unwrap();
    assert_eq!(tokens.title, "Input tokens");
    assert_eq!(tokens.unit, "tokens");
    assert_eq!(tokens.display_unit, "tokens");
    assert_eq!(tokens.suppliers, ["claude", "codex"]);
    assert!(tokens.stackable);
    let usage = &definitions[0];
    assert_eq!(usage.id, "windowUsagePercent");
    assert_eq!(usage.display_unit, "%");
    assert_eq!(usage.mark, "stepLine");
}
