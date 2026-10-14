//! Checks project naming through the macOS boundary.

use super::*;

#[test]
fn project_identity_bridge_uses_shared_directory_fallback() {
    assert_eq!(
        claude_encoded_project_name("-Users-me-repo".into()),
        "directory:-Users-me-repo"
    );
    assert_eq!(
        claude_project_name("/projects/-Users-me-repo/session.jsonl".into(), "/projects".into()),
        "directory:-Users-me-repo"
    );
}

#[test]
fn transcript_scope_bridge_returns_a_warning_only_for_an_unrelated_keychain_service() {
    assert!(claude_local_transcript_warning("claude.keychain:local".into(), Some("local".into())).is_none());
    assert!(
        claude_local_transcript_warning("claude.keychain:other".into(), Some("local".into()))
            .unwrap()
            .contains("transcript directory")
    );
}
