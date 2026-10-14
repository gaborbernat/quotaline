//! Checks credential replacement against concurrent sign-in changes.

use super::*;

#[test]
fn credential_guard_replaces_unchanged_document() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    fs::write(&path, b"{\"token\":\"old\"}").unwrap();
    CredentialFileGuard::observe(&path, &serde_json::json!({"token":"old"}))
        .unwrap()
        .replace(&serde_json::json!({"token":"new"}))
        .unwrap();
    assert_eq!(fs::read(path).unwrap(), b"{\"token\":\"new\"}\n");
}

#[test]
fn credential_guard_rejects_changed_document_before_refresh() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    fs::write(&path, b"{\"token\":\"new\"}").unwrap();
    assert!(matches!(
        CredentialFileGuard::observe(&path, &serde_json::json!({"token":"old"})),
        Err(CredentialFileError::Changed)
    ));
    assert_eq!(fs::read(path).unwrap(), b"{\"token\":\"new\"}");
}

#[test]
fn credential_guard_rejects_changed_bytes_after_refresh() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    fs::write(&path, b"{\"token\":\"old\"}").unwrap();
    let guard = CredentialFileGuard::observe(&path, &serde_json::json!({"token":"old"})).unwrap();
    fs::write(&path, b"{ \"token\":\"old\" }").unwrap();
    assert!(matches!(
        guard.replace(&serde_json::json!({"token":"new"})),
        Err(CredentialFileError::Changed)
    ));
    assert_eq!(fs::read(path).unwrap(), b"{ \"token\":\"old\" }");
}

#[test]
fn credential_guard_reports_missing_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    assert!(matches!(
        CredentialFileGuard::observe(&path, &serde_json::json!({})),
        Err(CredentialFileError::Read(_))
    ));
}

#[test]
fn credential_guard_reports_invalid_json() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    fs::write(&path, b"{").unwrap();
    assert!(matches!(
        CredentialFileGuard::observe(&path, &serde_json::json!({})),
        Err(CredentialFileError::InvalidJson(_))
    ));
}

#[test]
fn credential_guard_reports_disappearing_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    fs::write(&path, b"{}").unwrap();
    let guard = CredentialFileGuard::observe(&path, &serde_json::json!({})).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(matches!(
        guard.replace(&serde_json::json!({"token":"new"})),
        Err(CredentialFileError::Read(_))
    ));
}

#[cfg(unix)]
#[test]
fn credential_guard_reports_write_error() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("auth.json");
    fs::write(&path, b"{}").unwrap();
    let guard = CredentialFileGuard::observe(&path, &serde_json::json!({})).unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o500)).unwrap();
    let result = guard.replace(&serde_json::json!({"token":"new"}));
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(result, Err(CredentialFileError::Write(_))));
    assert_eq!(fs::read(path).unwrap(), b"{}");
}
