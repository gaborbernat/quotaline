use super::*;

#[test]
fn bridge_discovers_the_app_and_agent_sources() {
    let folder = tempfile::tempdir().unwrap();
    let state = folder.path().join("state.vscdb");
    let agent = folder.path().join("auth.json");
    let paths = || {
        (
            state.to_string_lossy().into_owned(),
            agent.to_string_lossy().into_owned(),
        )
    };
    let (state_path, agent_path) = paths();
    assert!(discover_cursor_credentials(state_path, agent_path).unwrap().is_none());
    std::fs::write(&agent, r#"{"accessToken":"agent"}"#).unwrap();
    let (state_path, agent_path) = paths();
    let found = discover_cursor_credentials(state_path, agent_path).unwrap().unwrap();
    assert_eq!(
        (found.origin, found.credentials.access_token.as_str()),
        (CursorCredentialOrigin::Agent, "agent")
    );
    let db = rusqlite::Connection::open(&state).unwrap();
    db.execute_batch(
        "CREATE TABLE ItemTable (key TEXT, value TEXT); INSERT INTO ItemTable VALUES ('cursorAuth/accessToken','app')",
    )
    .unwrap();
    let (state_path, agent_path) = paths();
    let found = discover_cursor_credentials(state_path, agent_path).unwrap().unwrap();
    assert_eq!(
        (found.origin, found.credentials.access_token.as_str()),
        (CursorCredentialOrigin::App, "app")
    );
}

#[test]
fn bridge_preserves_the_failing_source() {
    let folder = tempfile::tempdir().unwrap();
    let state = folder.path().join("state.vscdb");
    let agent = folder.path().join("auth.json");
    std::fs::write(&agent, b"{").unwrap();
    assert!(matches!(
        discover_cursor_credentials(
            state.to_string_lossy().into_owned(),
            agent.to_string_lossy().into_owned()
        ),
        Err(CursorCredentialError::AgentJson { .. })
    ));
    std::fs::write(&state, b"invalid database").unwrap();
    assert!(matches!(
        discover_cursor_credentials(
            state.to_string_lossy().into_owned(),
            agent.to_string_lossy().into_owned()
        ),
        Err(CursorCredentialError::StateQuery { .. })
    ));
}

#[test]
fn bridge_reports_unreadable_app_and_agent_sources() {
    let folder = tempfile::tempdir().unwrap();
    let agent = folder.path().join("auth.json");
    assert!(matches!(
        discover_cursor_credentials(
            folder.path().to_string_lossy().into_owned(),
            agent.to_string_lossy().into_owned()
        ),
        Err(CursorCredentialError::StateOpen { .. })
    ));
    let state = folder.path().join("state.vscdb");
    let db = rusqlite::Connection::open(&state).unwrap();
    db.execute_batch(
        "CREATE TABLE ItemTable (key TEXT, value INTEGER); INSERT INTO ItemTable VALUES ('cursorAuth/accessToken', 1)",
    )
    .unwrap();
    assert!(matches!(
        discover_cursor_credentials(
            state.to_string_lossy().into_owned(),
            agent.to_string_lossy().into_owned()
        ),
        Err(CursorCredentialError::StateRead { .. })
    ));
    std::fs::remove_file(&state).unwrap();
    assert!(matches!(
        discover_cursor_credentials(
            state.to_string_lossy().into_owned(),
            folder.path().to_string_lossy().into_owned()
        ),
        Err(CursorCredentialError::AgentFile { .. })
    ));
}
