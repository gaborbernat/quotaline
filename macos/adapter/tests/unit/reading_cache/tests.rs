//! Checks that the launch cache keeps readings across launches.

use super::*;
use crate::provider_record::ProviderRecord;
use quotaline_quota::Provider;

fn cache_directory() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn read(contents: &[u8]) -> Result<Vec<ProviderReadingRecord>, SnapshotCacheError> {
    let directory = cache_directory();
    let path = directory.path().join("cache.json");
    std::fs::write(&path, contents).unwrap();
    read_reading_cache(path.to_string_lossy().into_owned())
}

#[test]
fn reading_cache_is_empty_without_a_file() {
    let directory = cache_directory();
    let path = directory.path().join("cache.json");
    assert!(
        read_reading_cache(path.to_string_lossy().into_owned())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reading_cache_reports_io_errors() {
    let directory = cache_directory();
    assert!(matches!(
        read_reading_cache(directory.path().to_string_lossy().into_owned()),
        Err(SnapshotCacheError::Io { .. })
    ));
}

#[test]
fn reading_cache_marks_stored_readings_as_cached() {
    let mut provider = Provider::empty("claude", "connected");
    provider.plan = "Max".into();
    let stored = ProviderReadingRecord {
        provider: ProviderRecord::from(provider),
        fetched_at: 1_780_000_000.0,
        source: ReadingSource::Network,
    };
    let restored = read(&encode_reading_cache(vec![stored.clone()])).unwrap();
    assert_eq!(
        restored,
        vec![ProviderReadingRecord {
            source: ReadingSource::Cache,
            ..stored
        }]
    );
}

#[test]
fn reading_cache_rejects_unreadable_contents() {
    for contents in [&b"{"[..], br#"["claude",{"provider":"claude"}]"#] {
        assert!(matches!(read(contents), Err(SnapshotCacheError::Malformed)));
    }
}
