//! Keeps exported history private and safe to open in a spreadsheet.

use std::os::unix::fs::PermissionsExt;

use super::*;

#[test]
fn exports_prefix_cells_that_spreadsheets_would_evaluate() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("history.csv");
    let store = Store::open(None, 60).unwrap();
    let mut row = sample(1000.0, 20.0, None);
    row.label = "=HYPERLINK(\"x\")".into();
    store.seed(&[row]).unwrap();
    store.export_samples(&path, 0.0, 2000.0, None, true).unwrap();
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("\"'=HYPERLINK(\"\"x\"\")\"")
    );
}

#[test]
fn exports_replace_existing_files_with_owner_only_access() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("history.csv");
    fs::write(&path, "old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    Store::open(None, 60).unwrap().export_all(&path, false).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn usage_dates_without_a_key_filter_cover_every_window() {
    let store = Store::open(None, 60).unwrap();
    let mut other = sample(1060.0, 5.0, None);
    other.sample.key = "codex:primary".into();
    store.seed(&[sample(1000.0, 10.0, None), other]).unwrap();
    let dates = store.last_usage_dates(None, 0.0, 2000.0).unwrap();
    assert_eq!(
        dates,
        [("claude:session".into(), 1000.0), ("codex:primary".into(), 1060.0)]
    );
}
