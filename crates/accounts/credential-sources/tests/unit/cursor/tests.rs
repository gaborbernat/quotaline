//! Checks Cursor files through the shared credential reader.

use super::*;
use std::fs;

mod discovery;

#[test]
fn missing_sources_have_no_credentials() {
    let folder = tempfile::tempdir().unwrap();
    assert_eq!(load_cursor_state(&folder.path().join("missing.vscdb")).unwrap(), None);
    assert_eq!(load_cursor_agent(&folder.path().join("missing.json")).unwrap(), None);
}

#[test]
fn state_reader_loads_wal_changes_and_optional_fields() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("state.vscdb");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
         PRAGMA wal_checkpoint(TRUNCATE);
         PRAGMA wal_autocheckpoint=0;",
    )
    .unwrap();
    assert_eq!(load_cursor_state(&path).unwrap(), None);
    for (key, value) in [
        ("cursorAuth/accessToken", "token"),
        ("cursorAuth/refreshToken", "refresh"),
        ("cursorAuth/cachedEmail", "user@example.com"),
        ("cursorAuth/stripeMembershipType", "pro"),
        ("unrelated", "ignored"),
    ] {
        db.execute("INSERT INTO ItemTable VALUES (?1,?2)", [key, value])
            .unwrap();
    }
    assert!(path.with_extension("vscdb-wal").exists());
    assert_eq!(
        load_cursor_state(&path).unwrap(),
        Some(CursorAuth {
            access_token: "token".into(),
            refresh_token: Some("refresh".into()),
            email: Some("user@example.com".into()),
            membership_type: Some("pro".into()),
        })
    );
    db.execute("UPDATE ItemTable SET value='' WHERE key='cursorAuth/accessToken'", [])
        .unwrap();
    assert_eq!(load_cursor_state(&path).unwrap(), None);
}

#[test]
fn state_reader_classifies_open_query_and_row_errors() {
    let folder = tempfile::tempdir().unwrap();
    assert!(matches!(
        load_cursor_state(folder.path()),
        Err(CursorSourceError::Open(_))
    ));
    let invalid = folder.path().join("invalid.vscdb");
    fs::write(&invalid, b"invalid database").unwrap();
    assert!(matches!(load_cursor_state(&invalid), Err(CursorSourceError::Query(_))));
    let wrong_type = folder.path().join("wrong.vscdb");
    let db = Connection::open(&wrong_type).unwrap();
    db.execute_batch(
        "CREATE TABLE ItemTable (key TEXT, value INTEGER); INSERT INTO ItemTable VALUES ('cursorAuth/accessToken',123)",
    )
    .unwrap();
    assert!(matches!(
        load_cursor_state(&wrong_type),
        Err(CursorSourceError::Read(_))
    ));
}

#[test]
fn state_reader_accepts_utf8_blob_values_and_rejects_invalid_bytes() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("state.vscdb");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE ItemTable (key TEXT, value BLOB)")
        .unwrap();
    db.execute(
        "INSERT INTO ItemTable VALUES (?1,?2)",
        rusqlite::params!["cursorAuth/accessToken", b"blob-token"],
    )
    .unwrap();
    assert_eq!(load_cursor_state(&path).unwrap().unwrap().access_token, "blob-token");
    db.execute("UPDATE ItemTable SET value=?1", [vec![0xff]]).unwrap();
    assert!(matches!(load_cursor_state(&path), Err(CursorSourceError::Read(_))));
}

#[test]
fn state_reader_keeps_the_last_duplicate_value() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("state.vscdb");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE ItemTable (key TEXT, value TEXT);
         INSERT INTO ItemTable VALUES ('cursorAuth/accessToken', 'first');
         INSERT INTO ItemTable VALUES ('cursorAuth/accessToken', 'second');
         INSERT INTO ItemTable VALUES ('cursorAuth/cachedEmail', 'you@example.com');",
    )
    .unwrap();
    let auth = load_cursor_state(&path).unwrap().unwrap();
    assert_eq!(
        (auth.access_token.as_str(), auth.email.as_deref()),
        ("second", Some("you@example.com"))
    );
}

#[test]
fn agent_reader_loads_tokens_and_optional_fields() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("auth.json");
    fs::write(
        &path,
        r#"{"accessToken":"token","refreshToken":"refresh","email":"user@example.com","membershipType":"pro"}"#,
    )
    .unwrap();
    assert_eq!(
        load_cursor_agent(&path).unwrap(),
        Some(CursorAuth {
            access_token: "token".into(),
            refresh_token: Some("refresh".into()),
            email: Some("user@example.com".into()),
            membership_type: Some("pro".into()),
        })
    );
    fs::write(&path, r#"{"other":true}"#).unwrap();
    assert_eq!(load_cursor_agent(&path).unwrap(), None);
}

#[test]
fn agent_reader_reports_io_and_json_errors() {
    let folder = tempfile::tempdir().unwrap();
    assert!(matches!(
        load_cursor_agent(folder.path()),
        Err(CursorSourceError::File(_))
    ));
    let path = folder.path().join("auth.json");
    fs::write(&path, b"{").unwrap();
    assert!(matches!(load_cursor_agent(&path), Err(CursorSourceError::Json(_))));
}
