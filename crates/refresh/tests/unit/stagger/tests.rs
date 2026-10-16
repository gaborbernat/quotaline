use super::*;

#[test]
fn tracks_pending_providers_and_resets() {
    let mut stagger = RefreshStagger::new(["claude".into()]);
    assert_eq!(stagger.interval("claude", 600.0, 2, false, 3), 624.0);
    assert_eq!(stagger.interval("codex", 600.0, 2, false, 3), 600.0);
    stagger.set_pending("claude", false);
    stagger.set_pending("codex", true);
    assert_eq!(stagger.interval("claude", 600.0, 2, false, 3), 600.0);
    assert_eq!(stagger.interval("codex", 600.0, 2, false, 3), 624.0);
    stagger.reset(["gemini".into()]);
    assert_eq!(stagger.interval("codex", 600.0, 2, false, 3), 600.0);
    assert_eq!(stagger.interval("gemini", 600.0, 2, false, 3), 624.0);
}
