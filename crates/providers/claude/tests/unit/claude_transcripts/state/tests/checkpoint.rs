//! Checks published Swift checkpoint dates and project migration.

use super::*;

#[test]
fn empty_state_round_trips_with_current_project_version() {
    let state = TranscriptState::default();
    let checkpoint = state.checkpoint().unwrap();
    assert_eq!(
        TranscriptState::from_checkpoint(&checkpoint).unwrap().message_count(),
        0
    );
    let value: serde_json::Value = serde_json::from_slice(&checkpoint).unwrap();
    assert_eq!(value["projectIdentityVersion"], 2);
    assert_eq!(value["activeRetentionDays"], 60);
}

#[test]
fn current_checkpoint_reuses_seen_ids_when_project_ids_are_missing() {
    let data = serde_json::json!({
        "offsets": {"file": {"bytes": 12, "scanGeneration": 7}},
        "seenByDay": {"2026-10-09": ["one:"]},
        "days": {}, "recent": {}, "projectIdentityVersion": 2
    });
    let state = TranscriptState::from_checkpoint(data.to_string().as_bytes()).unwrap();
    assert_eq!(
        state.offsets["file"],
        TranscriptOffset {
            bytes: 12,
            scan_generation: 7
        }
    );
    assert_eq!(state.project_seen_by_day["2026-10-09"], BTreeSet::from(["one:".into()]));
    assert!(!state.requires_checkpoint());
}

#[test]
fn old_checkpoint_rewinds_offsets_and_preserves_legacy_projects() {
    let data = serde_json::json!({
        "offsets": {"file": 12}, "seenByDay": {},
        "days": {"2026-10-09": {"models": {}, "messages": 1, "sessions": [], "toolCalls": 0,
            "projects": {"repo": {"cost": 0.5, "messages": 1}}}},
        "recent": {}
    });
    let state = TranscriptState::from_checkpoint(data.to_string().as_bytes()).unwrap();
    assert_eq!(
        state.offsets["file"],
        TranscriptOffset {
            bytes: 0,
            scan_generation: 0
        }
    );
    assert!(state.requires_checkpoint());
    assert_eq!(state.days["2026-10-09"].projects["legacy:repo"].cost, 0.5);
}

#[test]
fn checkpoint_preserves_swift_reference_date_encoding() {
    let data = serde_json::json!({
        "offsets": {}, "seenByDay": {}, "days": {},
        "recent": {"978307200": {"timestamp": 0.0, "tokens": 2, "cost": 0.5, "messages": 1}}
    });
    let state = TranscriptState::from_checkpoint(data.to_string().as_bytes()).unwrap();
    assert_eq!(state.recent["978307200"].timestamp, SwiftDate(978_307_200.0));
    let checkpoint: serde_json::Value = serde_json::from_slice(&state.checkpoint().unwrap()).unwrap();
    assert_eq!(checkpoint["recent"]["978307200"]["timestamp"], 0.0);
}

#[test]
fn malformed_checkpoint_reports_a_parse_error() {
    assert!(TranscriptState::from_checkpoint(b"{").is_err());
    let invalid = serde_json::json!({
        "offsets": {"file": "bad"}, "seenByDay": {}, "days": {}, "recent": {}
    });
    assert!(TranscriptState::from_checkpoint(invalid.to_string().as_bytes()).is_err());
}
