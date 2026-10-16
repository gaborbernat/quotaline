use super::*;

fn path(directory: &tempfile::TempDir, name: &str) -> String {
    directory.path().join(name).display().to_string()
}

fn content(attention: bool) -> WidgetContent {
    WidgetContent {
        rows: Vec::new(),
        attention,
        display: "used".into(),
    }
}

#[test]
fn snapshots_coalesce_and_write_only_changed_values() {
    let directory = tempfile::tempdir().unwrap();
    let cache = path(&directory, "cache.json");
    let usage = path(&directory, "usage.json");
    let ledger = SnapshotWriteLedger::new(Some(cache.clone()), Some(usage.clone()), None);
    assert!(!ledger.has_pending_snapshots());
    ledger.record_cache_load();
    ledger.submit_snapshots(b"old".to_vec(), None);
    ledger.submit_snapshots(b"new".to_vec(), Some(b"usage".to_vec()));
    assert!(ledger.has_pending_snapshots());
    assert_eq!(ledger.drain_snapshots(), []);
    assert!(!ledger.has_pending_snapshots());
    assert_eq!(std::fs::read(cache).unwrap(), b"new");
    assert_eq!(std::fs::read(usage).unwrap(), b"usage");
    ledger.submit_snapshots(b"new".to_vec(), Some(b"usage".to_vec()));
    assert_eq!(ledger.drain_snapshots(), []);
    assert_eq!(
        ledger.workload(),
        PersistenceWorkload {
            cache_loads: 1,
            cache_submissions: 3,
            cache_writes: 1,
            coalesced_cache_submissions: 1,
            usage_file_writes: 1,
            ..PersistenceWorkload::default()
        }
    );
}

#[test]
fn snapshots_skip_absent_destinations_and_retry_failed_writes() {
    let directory = tempfile::tempdir().unwrap();
    let no_path = SnapshotWriteLedger::new(None, None, None);
    no_path.submit_snapshots(b"cache".to_vec(), Some(b"usage".to_vec()));
    assert_eq!(no_path.drain_snapshots(), []);
    assert_eq!(no_path.workload().cache_writes, 1);
    assert_eq!(no_path.workload().usage_file_writes, 0);

    let cache_blocker = path(&directory, "cache-blocker");
    let usage_blocker = path(&directory, "usage-blocker");
    std::fs::write(&cache_blocker, []).unwrap();
    std::fs::write(&usage_blocker, []).unwrap();
    let cache = format!("{cache_blocker}/cache.json");
    let usage = format!("{usage_blocker}/usage.json");
    let ledger = SnapshotWriteLedger::new(Some(cache.clone()), Some(usage.clone()), None);
    ledger.submit_snapshots(b"cache".to_vec(), Some(b"usage".to_vec()));
    assert!(matches!(
        ledger.drain_snapshots().as_slice(),
        [
            PersistenceFailure::CacheWrite { .. },
            PersistenceFailure::UsageFileWrite { .. }
        ]
    ));
    assert_eq!(ledger.workload().cache_writes, 0);
    std::fs::remove_file(cache_blocker).unwrap();
    std::fs::remove_file(usage_blocker).unwrap();
    ledger.submit_snapshots(b"cache".to_vec(), Some(b"usage".to_vec()));
    assert_eq!(ledger.drain_snapshots(), []);
    assert_eq!(std::fs::read(cache).unwrap(), b"cache");
    assert_eq!(std::fs::read(usage).unwrap(), b"usage");
}

#[test]
fn widgets_write_freshness_without_reloading_unchanged_content() {
    let directory = tempfile::tempdir().unwrap();
    let widget = path(&directory, "widget.json");
    let ledger = SnapshotWriteLedger::new(None, None, Some(widget.clone()));
    assert!(!ledger.has_pending_widget());
    ledger.submit_widget(b"discarded".to_vec(), content(false));
    ledger.submit_widget(b"first".to_vec(), content(false));
    assert!(ledger.has_pending_widget());
    assert_eq!(ledger.drain_widgets().reloads, 1);
    assert_eq!(std::fs::read(&widget).unwrap(), b"first");
    assert!(!ledger.has_pending_widget());
    ledger.submit_widget(b"first".to_vec(), content(false));
    assert_eq!(ledger.drain_widgets().reloads, 0);
    ledger.submit_widget(b"heartbeat".to_vec(), content(false));
    assert_eq!(ledger.drain_widgets().reloads, 0);
    ledger.submit_widget(b"attention".to_vec(), content(true));
    assert_eq!(ledger.drain_widgets().reloads, 1);
    assert_eq!(std::fs::read(widget).unwrap(), b"attention");
    assert_eq!(
        ledger.workload(),
        PersistenceWorkload {
            widget_submissions: 5,
            widget_writes: 3,
            coalesced_widget_submissions: 1,
            widget_reloads: 2,
            ..PersistenceWorkload::default()
        }
    );
}

#[test]
fn widgets_skip_missing_store_and_retry_failed_write() {
    let no_path = SnapshotWriteLedger::new(None, None, None);
    no_path.submit_widget(b"ignored".to_vec(), content(false));
    assert!(!no_path.has_pending_widget());
    assert_eq!(no_path.workload().widget_submissions, 0);

    let directory = tempfile::tempdir().unwrap();
    let blocker = path(&directory, "blocker");
    std::fs::write(&blocker, []).unwrap();
    let widget = format!("{blocker}/widget.json");
    let ledger = SnapshotWriteLedger::new(None, None, Some(widget.clone()));
    ledger.submit_widget(b"widget".to_vec(), content(false));
    assert!(matches!(
        ledger.drain_widgets().failures.as_slice(),
        [PersistenceFailure::WidgetWrite { .. }]
    ));
    assert_eq!(ledger.workload().widget_reloads, 0);
    std::fs::remove_file(blocker).unwrap();
    ledger.submit_widget(b"widget".to_vec(), content(false));
    assert_eq!(ledger.drain_widgets().reloads, 1);
    assert_eq!(std::fs::read(widget).unwrap(), b"widget");
}
