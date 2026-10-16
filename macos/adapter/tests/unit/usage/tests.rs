use super::*;

#[test]
fn usage_card_visibility_matches_shared_policy() {
    assert!(usage_card_visible(true, true, false, false));
    assert!(usage_card_visible(true, false, true, false));
    assert!(usage_card_visible(true, false, false, true));
    assert!(!usage_card_visible(true, false, false, false));
    assert!(!usage_card_visible(false, true, false, false));
}
