use super::*;

fn test_directory() -> tempfile::TempDir {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

#[test]
fn cache_bridge_writes_readings_it_reads_back() {
    let directory = test_directory();
    let path = directory.path().join("cache.json").display().to_string();
    let data = crate::reading_cache::encode_reading_cache(Vec::new());
    assert_eq!(
        write_snapshot_cache(path.clone(), data.clone()).unwrap(),
        data.len() as u64
    );
    assert!(crate::reading_cache::read_reading_cache(path).unwrap().is_empty());
}

#[test]
fn cache_bridge_reports_write_errors() {
    let directory = test_directory();
    assert!(matches!(
        write_snapshot_cache(directory.path().display().to_string(), b"[]".to_vec()),
        Err(SnapshotCacheError::Io { .. })
    ));
}

#[test]
fn secret_write_persists_bytes_and_exposes_errors() {
    let directory = test_directory();
    let path = directory.path().join("auth.json");
    write_secret_file(path.display().to_string(), b"secret".to_vec()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"secret");
    assert!(matches!(
        write_secret_file(
            directory.path().join("missing/auth.json").display().to_string(),
            b"secret".to_vec()
        ),
        Err(StorageFileError::Io { .. })
    ));
}

#[test]
fn optional_file_read_preserves_missing_and_present_values() {
    let directory = test_directory();
    let path = directory.path().join("auth.json");
    assert_eq!(read_optional_file(path.display().to_string()).unwrap(), None);
    std::fs::write(&path, b"auth").unwrap();
    assert_eq!(
        read_optional_file(path.display().to_string()).unwrap(),
        Some(b"auth".to_vec())
    );
    assert!(matches!(
        read_optional_file(directory.path().display().to_string()),
        Err(StorageFileError::Io { .. })
    ));
}

#[test]
fn private_file_bridge_round_trips_and_reports_io_errors() {
    let directory = test_directory();
    let path = directory.path().join("nested/snapshots.json");
    let path_string = path.display().to_string();
    assert_eq!(read_optional_file(path_string.clone()).unwrap(), None);
    assert_eq!(write_private_file(path_string.clone(), b"{}".to_vec()).unwrap(), 2);
    assert_eq!(read_optional_file(path_string).unwrap(), Some(b"{}".to_vec()));
    assert!(matches!(
        read_optional_file(directory.path().display().to_string()),
        Err(StorageFileError::Io { .. })
    ));
    assert!(matches!(
        write_private_file(directory.path().display().to_string(), b"{}".to_vec()),
        Err(StorageFileError::Io { .. })
    ));
}
