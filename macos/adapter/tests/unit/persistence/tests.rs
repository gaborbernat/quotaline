use super::*;

fn path(directory: &tempfile::TempDir, name: &str) -> String {
    directory.path().join(name).display().to_string()
}

fn content() -> WidgetContent {
    WidgetContent {
        rows: Vec::new(),
        attention: false,
        display: "used".into(),
    }
}

#[test]
fn bridge_exposes_pending_writes_and_workload() {
    let directory = tempfile::tempdir().unwrap();
    let cache = path(&directory, "cache.json");
    let usage = path(&directory, "usage.json");
    let widget = path(&directory, "widget.json");
    let ledger = SnapshotWriteLedger::new(Some(cache.clone()), Some(usage.clone()), Some(widget.clone()));
    ledger.record_cache_load();
    ledger.submit_snapshots(b"cache".to_vec(), Some(b"usage".to_vec()));
    ledger.submit_widget(b"widget".to_vec(), content());
    assert!(ledger.has_pending_snapshots());
    assert!(ledger.has_pending_widget());
    assert_eq!(ledger.drain_snapshots(), []);
    assert_eq!(
        ledger.drain_widgets(),
        WidgetDrainResult {
            failures: Vec::new(),
            reloads: 1,
        }
    );
    assert!(!ledger.has_pending_snapshots());
    assert!(!ledger.has_pending_widget());
    assert_eq!(std::fs::read(cache).unwrap(), b"cache");
    assert_eq!(std::fs::read(usage).unwrap(), b"usage");
    assert_eq!(std::fs::read(widget).unwrap(), b"widget");
    assert_eq!(
        ledger.workload(),
        PersistenceWorkload {
            cache_loads: 1,
            cache_submissions: 1,
            cache_writes: 1,
            usage_file_writes: 1,
            widget_submissions: 1,
            widget_writes: 1,
            widget_reloads: 1,
            ..PersistenceWorkload::default()
        }
    );
}

#[test]
fn bridge_maps_each_write_failure() {
    let directory = tempfile::tempdir().unwrap();
    let blocker = path(&directory, "blocker");
    std::fs::write(&blocker, []).unwrap();
    let ledger = SnapshotWriteLedger::new(
        Some(format!("{blocker}/cache.json")),
        Some(format!("{blocker}/usage.json")),
        Some(format!("{blocker}/widget.json")),
    );
    ledger.submit_snapshots(b"cache".to_vec(), Some(b"usage".to_vec()));
    ledger.submit_widget(b"widget".to_vec(), content());
    assert!(matches!(
        ledger.drain_snapshots().as_slice(),
        [
            PersistenceFailure::CacheWrite { .. },
            PersistenceFailure::UsageFileWrite { .. }
        ]
    ));
    assert!(matches!(
        ledger.drain_widgets().failures.as_slice(),
        [PersistenceFailure::WidgetWrite { .. }]
    ));
}
