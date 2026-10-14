use super::*;

#[test]
fn cursor_discovery_prefers_the_app_and_falls_back_to_the_agent() {
    let folder = tempfile::tempdir().unwrap();
    let state = folder.path().join("state.vscdb");
    let agent = folder.path().join("auth.json");
    assert!(discover_cursor(&state, &agent).unwrap().is_none());
    fs::write(&agent, r#"{"accessToken":"agent"}"#).unwrap();
    assert_eq!(
        discover_cursor(&state, &agent).unwrap().unwrap().origin,
        CursorOrigin::Agent
    );
    let db = Connection::open(&state).unwrap();
    db.execute_batch("CREATE TABLE ItemTable (key TEXT, value TEXT);")
        .unwrap();
    db.execute("INSERT INTO ItemTable VALUES ('cursorAuth/accessToken', 'app')", [])
        .unwrap();
    let found = discover_cursor(&state, &agent).unwrap().unwrap();
    assert_eq!(
        (found.origin, found.auth.access_token.as_str()),
        (CursorOrigin::App, "app")
    );
}

#[test]
fn cursor_discovery_retains_the_first_error_after_fallback() {
    let folder = tempfile::tempdir().unwrap();
    let state = folder.path().join("state.vscdb");
    let agent = folder.path().join("auth.json");
    fs::write(&state, b"not a database").unwrap();
    assert!(matches!(
        discover_cursor(&state, &agent),
        Err(CursorSourceError::Query(_))
    ));
    fs::write(&agent, b"{").unwrap();
    assert!(matches!(
        discover_cursor(&state, &agent),
        Err(CursorSourceError::Query(_))
    ));
    fs::write(&agent, r#"{"accessToken":"agent"}"#).unwrap();
    assert_eq!(
        discover_cursor(&state, &agent).unwrap().unwrap().origin,
        CursorOrigin::Agent
    );
}

#[test]
fn cursor_discovery_rejects_unreadable_and_empty_agent_credentials() {
    let folder = tempfile::tempdir().unwrap();
    let state = folder.path().join("state.vscdb");
    let agent = folder.path().join("auth.json");
    fs::write(&agent, b"{").unwrap();
    assert!(matches!(
        discover_cursor(&state, &agent),
        Err(CursorSourceError::Json(_))
    ));
    fs::write(&agent, r#"{"accessToken":""}"#).unwrap();
    assert!(discover_cursor(&state, &agent).unwrap().is_none());
}
