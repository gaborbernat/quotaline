use super::*;

#[test]
fn scanner_defaults_are_shared_with_swift() {
    let defaults = codex_rollout_defaults();
    assert_eq!(defaults.max_files, 8);
    assert_eq!(defaults.maximum_line_bytes, 1024 * 1024);
    assert_eq!(defaults.cache_seconds, 15.0 * 60.0);
    assert_eq!(defaults.byte_budget, 2 * 1024 * 1024);
    assert_eq!(defaults.entry_budget, 256);
    assert_eq!(defaults.time_seconds, 0.05);
}

#[test]
fn scanner_exposes_bounded_reading_and_workload() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("rollout-current.jsonl"),
        r#"{"rate_limits":{"primary":{"used_percent":7,"window_minutes":300}}}"#,
    )
    .unwrap();
    let scanner = CodexRolloutScanner::new(directory.path().to_string_lossy().into(), 300.0, 1_024, 256, 1.0);
    assert_eq!(
        scanner
            .latest(1_788_000_000.0, false)
            .unwrap()
            .provider(Default::default())
            .windows[0]
            .used,
        7.0
    );
    assert_eq!(scanner.workload().trees_scanned, 1);
    assert!(!scanner.pending());
    assert_eq!(scanner.candidate_paths().len(), 1);
    scanner.start_scan(1_788_000_000.0);
    assert!(scanner.tree_pending());
    scanner.run_slice();
    scanner.cancel();
    assert!(!scanner.pending());
}
