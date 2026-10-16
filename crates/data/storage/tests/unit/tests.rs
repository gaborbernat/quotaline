use super::*;

#[test]
fn optional_file_read_distinguishes_missing_and_unreadable_paths() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    assert_eq!(read_optional_file(&path).unwrap(), None);
    fs::write(&path, b"data").unwrap();
    assert_eq!(read_optional_file(&path).unwrap(), Some(b"data".to_vec()));
    assert!(read_optional_file(directory.path()).is_err());
}

#[test]
fn secret_write_replaces_file_with_private_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("credentials.json");
    write_secret_file(&path, b"old").unwrap();
    write_secret_file(&path, b"new").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[cfg(unix)]
#[test]
fn secret_write_follows_destination_symlink() {
    let directory = tempfile::tempdir().unwrap();
    let actual = directory.path().join("actual.json");
    let linked = directory.path().join("linked.json");
    fs::write(&actual, b"old").unwrap();
    std::os::unix::fs::symlink(&actual, &linked).unwrap();
    write_secret_file(&linked, b"new").unwrap();
    assert_eq!(fs::read(&actual).unwrap(), b"new");
    assert_eq!(fs::read_link(&linked).unwrap(), actual);
}

#[test]
fn secret_write_does_not_create_parent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing/credentials.json");
    assert_eq!(
        write_secret_file(&path, b"data").unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn secret_write_requires_a_filename() {
    assert_eq!(
        write_secret_file(Path::new(""), b"data").unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn failed_replacement_cleans_temporary_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("occupied");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("child"), b"keep").unwrap();
    assert!(write_secret_file(&path, b"data").is_err());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    assert_eq!(fs::read(path.join("child")).unwrap(), b"keep");
}

#[test]
fn atomic_json_creates_parent_and_appends_newline() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/state.json");
    atomic_json(&path, &serde_json::json!({"active":true})).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"{\"active\":true}\n");
}

#[test]
fn atomic_json_rejects_unserializable_values_without_writing() {
    struct Unserializable;
    impl Serialize for Unserializable {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("cannot encode"))
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.json");
    assert_eq!(
        atomic_json(&path, &Unserializable).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(!path.exists());
}

#[test]
fn atomic_json_reports_invalid_parent() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("file");
    fs::write(&parent, b"occupied").unwrap();
    assert_eq!(
        atomic_json(&parent.join("state.json"), &0).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
}

#[test]
fn private_file_write_creates_parent_and_replaces_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/state.json");
    assert_eq!(write_private_file(&path, b"old").unwrap(), 3);
    assert_eq!(write_private_file(&path, b"new value").unwrap(), 9);
    assert_eq!(read_optional_file(&path).unwrap(), Some(b"new value".to_vec()));
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn private_file_write_reports_invalid_destination() {
    assert_eq!(
        write_private_file(Path::new(""), b"data").unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}
