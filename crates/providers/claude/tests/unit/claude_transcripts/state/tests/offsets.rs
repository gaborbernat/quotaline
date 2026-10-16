//! Checks scan offsets and retention changes through the state API.

use super::*;

#[test]
fn offsets_track_generations_and_prune_removed_files() {
    let mut state = TranscriptState::default();
    assert_eq!(state.offset("missing"), None);
    assert_eq!(state.maximum_scan_generation(), 0);
    state.set_offset("old".into(), 10, 1);
    state.set_offset("current".into(), 20, 2);
    state.touch_offset("old", 2);
    state.touch_offset("missing", 2);
    assert_eq!(state.offset("old").unwrap().scan_generation, 2);
    assert_eq!(state.maximum_scan_generation(), 2);
    assert!(!state.retain_offsets(2));
    state.set_offset("old".into(), 10, 1);
    assert!(state.retain_offsets(2));
    assert_eq!(state.offset("old"), None);
    assert_eq!(state.offset("current").unwrap().bytes, 20);
}

#[test]
fn expanding_retention_rewinds_offsets_but_shrinking_it_does_not() {
    let mut state = TranscriptState::default();
    state.set_offset("file".into(), 12, 4);
    assert!(!state.configure_retention(60));
    assert!(state.configure_retention(90));
    assert_eq!(state.offset("file").unwrap().bytes, 0);
    state.set_offset("file".into(), 20, 4);
    assert!(state.configure_retention(30));
    assert_eq!(state.offset("file").unwrap().bytes, 20);
}
