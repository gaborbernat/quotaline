use super::*;
use crate::test_records as records;

fn reading() -> ProviderReadingRecord {
    let mut reading = records::reading("claude", vec![records::window("session", 36.0, None)], 1_780_000_000.0);
    reading.provider.plan = "Max".into();
    reading
}

#[test]
fn bridge_exports_usage_and_writes_the_same_document() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/usage.json");
    let expected = usage_export_json(vec![reading()]);
    let value: serde_json::Value = serde_json::from_slice(&expected).unwrap();
    assert_eq!(value["providers"][0]["plan"], "Max");
    assert_eq!(value["providers"][0]["fetchedAt"], "2026-05-28T20:26:40Z");
    assert_eq!(
        usage_export_write(path.to_string_lossy().into_owned(), vec![reading()]).unwrap(),
        expected.len() as u64
    );
    assert_eq!(std::fs::read(path).unwrap(), expected);
}

#[test]
fn bridge_reports_write_errors() {
    assert!(matches!(
        usage_export_write(String::new(), vec![reading()]),
        Err(UsageExportFailure::Io { .. })
    ));
}
