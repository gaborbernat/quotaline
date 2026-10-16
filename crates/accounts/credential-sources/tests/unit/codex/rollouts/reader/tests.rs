use super::*;
use std::fs::File;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const NOW: f64 = 1_788_000_000.0;

fn reader(root: &Path, bytes: usize, entries: usize) -> RolloutReader {
    RolloutReader::new(
        root.to_path_buf(),
        Budget {
            bytes,
            entries,
            time: Duration::from_secs(1),
            cache: 300.0,
        },
    )
}

fn line(percent: u32) -> String {
    format!(r#"{{"rate_limits":{{"primary":{{"used_percent":{percent},"window_minutes":300}}}}}}"#)
}

fn write(root: &Path, name: &str, content: &str, modified: u64) -> PathBuf {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(modified)))
        .unwrap();
    path
}

fn finish(reader: &mut RolloutReader, now: f64) -> Option<Reading> {
    let mut reading = reader.latest(now, false);
    for _ in 0..10_000 {
        if !reader.pending() {
            break;
        }
        reading = reader.latest(now, false);
    }
    assert!(!reader.pending(), "rollout search did not finish");
    reading
}

fn percent(reading: Reading) -> u32 {
    reading.data["rate_limit"]["primary_window"]["used_percent"]
        .as_f64()
        .unwrap() as u32
}

#[test]
fn empty_and_missing_roots_have_no_reading() {
    let directory = tempfile::tempdir().unwrap();
    for root in [directory.path().to_path_buf(), directory.path().join("missing")] {
        let mut scanner = reader(&root, 1024, 256);
        assert!(finish(&mut scanner, NOW).is_none());
        assert_eq!(scanner.workload().trees_scanned, 1);
        assert_eq!(scanner.workload().searches_completed, 1);
    }
}

#[test]
fn newest_valid_line_wins_and_older_file_is_the_fallback() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "day/rollout-old.jsonl", &line(7), 10);
    let newer = write(
        directory.path(),
        "day/rollout-new.jsonl",
        &format!("{}\nnot json \"rate_limits\"\n{}\n", line(12), line(55)),
        20,
    );
    let mut scanner = reader(directory.path(), 1_024, 256);
    assert_eq!(percent(finish(&mut scanner, NOW).unwrap()), 55);
    fs::write(newer, "{}\n").unwrap();
    assert_eq!(percent(finish(&mut scanner, NOW + 1.0).unwrap()), 7);
    assert_eq!(scanner.workload().trees_scanned, 1);
}

#[test]
fn cache_rechecks_known_files_and_discovers_new_files_after_expiry() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(directory.path(), "rollout-current.jsonl", &line(7), 10);
    let mut scanner = reader(directory.path(), 1_024, 256);
    assert!(finish(&mut scanner, NOW).is_some());
    write(directory.path(), "rollout-new.jsonl", &line(42), 20);
    assert_eq!(percent(finish(&mut scanner, NOW + 60.0).unwrap()), 7);
    assert_eq!(scanner.workload().trees_scanned, 1);
    assert_eq!(percent(finish(&mut scanner, NOW + 301.0).unwrap()), 42);
    assert_eq!(scanner.workload().trees_scanned, 2);
    fs::remove_file(path).unwrap();
    assert!(finish(&mut scanner, NOW + 302.0).is_some());
    assert_eq!(scanner.workload().trees_scanned, 3);
}

#[test]
fn missing_cached_file_rebuilds_candidates() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(directory.path(), "rollout-current.jsonl", &line(7), 10);
    let mut scanner = reader(directory.path(), 1_024, 256);
    assert!(finish(&mut scanner, NOW).is_some());
    fs::remove_file(path).unwrap();
    assert!(finish(&mut scanner, NOW + 60.0).is_none());
    assert_eq!(scanner.workload().trees_scanned, 2);
}

#[test]
fn throttling_reuses_cached_candidates_but_rechecks_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(directory.path(), "rollout-current.jsonl", &line(7), 10);
    let mut scanner = reader(directory.path(), 1_024, 256);
    assert!(finish(&mut scanner, NOW).is_some());
    write(directory.path(), "rollout-new.jsonl", &line(42), 40);
    assert_eq!(percent(scanner.latest(NOW + 301.0, true).unwrap()), 7);
    write(directory.path(), "rollout-current.jsonl", &line(55), 30);
    assert_eq!(percent(scanner.latest(NOW + 302.0, true).unwrap()), 55);
    assert_eq!(scanner.workload().trees_scanned, 1);
    assert_eq!(percent(finish(&mut scanner, NOW + 303.0).unwrap()), 42);
    assert_eq!(scanner.workload().trees_scanned, 2);
    assert!(path.exists());
}

#[test]
fn file_and_entry_budgets_bound_each_slice() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..20 {
        write(
            directory.path(),
            &format!("rollout-{index}.jsonl"),
            &line(index),
            index as u64,
        );
    }
    let mut scanner = reader(directory.path(), 8, 1);
    assert!(scanner.latest(NOW, false).is_none());
    assert!(scanner.pending());
    assert_eq!(scanner.workload().tree_entries_examined, 1);
    assert_eq!(percent(finish(&mut scanner, NOW).unwrap()), 19);
    assert_eq!(scanner.workload().largest_tree_slice_entries, 1);
    assert!(scanner.workload().largest_slice_bytes_read <= 8);
    assert_eq!(scanner.candidate_paths().len(), 8);
}

#[cfg(unix)]
#[test]
fn scan_skips_hidden_paths_and_nonregular_candidates() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), ".hidden/rollout-secret.jsonl", &line(99), 100);
    write(directory.path(), ".rollout-hidden.jsonl", &line(99), 100);
    write(directory.path(), "rollout-invalid.txt", &line(99), 100);
    write(directory.path(), "other.jsonl", &line(99), 100);
    fs::create_dir(directory.path().join("rollout-directory.jsonl")).unwrap();
    let valid = write(directory.path(), "rollout-valid.jsonl", &line(7), 10);
    std::os::unix::fs::symlink(&valid, directory.path().join("rollout-link.jsonl")).unwrap();
    let mut scanner = reader(directory.path(), 1_024, 256);
    assert_eq!(percent(finish(&mut scanner, NOW).unwrap()), 7);
    assert_eq!(scanner.candidate_paths(), [valid]);
}

#[test]
fn older_candidates_do_not_displace_the_newest_eight() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..8 {
        write(
            directory.path(),
            &format!("rollout-new-{index}.jsonl"),
            &line(index),
            index as u64 + 10,
        );
    }
    write(directory.path(), "rollout-old.jsonl", &line(99), 1);
    let mut scanner = reader(directory.path(), 1024, 256);
    assert_eq!(percent(finish(&mut scanner, NOW).unwrap()), 7);
    assert_eq!(scanner.candidate_paths().len(), 8);
    assert!(
        scanner
            .candidate_paths()
            .iter()
            .all(|path| path.file_name().unwrap() != "rollout-old.jsonl")
    );
}

#[cfg(unix)]
#[test]
fn unreadable_newest_file_falls_back_to_an_older_one() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "rollout-old.jsonl", &line(7), 10);
    let newest = write(directory.path(), "rollout-new.jsonl", &line(99), 20);
    fs::set_permissions(&newest, fs::Permissions::from_mode(0o000)).unwrap();
    let mut scanner = reader(directory.path(), 1024, 256);
    let result = finish(&mut scanner, NOW);
    fs::set_permissions(&newest, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(percent(result.unwrap()), 7);
    assert_eq!(scanner.workload().files_opened, 1);
}

#[cfg(unix)]
#[test]
fn unsearchable_candidate_between_directory_iteration_and_stat_is_skipped() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "rollout-a.jsonl", &line(7), 10);
    write(directory.path(), "rollout-b.jsonl", &line(8), 20);
    let mut scanner = reader(directory.path(), 1024, 1);
    assert!(scanner.latest(NOW, false).is_none());
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o000)).unwrap();
    scanner.run_slice();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(scanner.workload().stat_checks >= 1);
}

#[test]
fn scan_orders_equal_modification_times_by_path() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["rollout-c.jsonl", "rollout-a.jsonl", "rollout-b.jsonl"] {
        write(directory.path(), name, "{}\n", 10);
    }
    let mut scanner = reader(directory.path(), 1_024, 256);
    scanner.start_scan(NOW);
    while scanner.tree_pending() {
        scanner.run_slice();
    }
    assert_eq!(
        scanner
            .candidate_paths()
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["rollout-a.jsonl", "rollout-b.jsonl", "rollout-c.jsonl"]
    );
}

#[test]
fn cancellation_retains_candidates_for_a_resumed_search() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "rollout-current.jsonl", &line(7), 10);
    let mut scanner = reader(directory.path(), 1, 256);
    assert!(scanner.latest(NOW, false).is_none());
    assert!(scanner.pending());
    scanner.cancel();
    assert!(!scanner.pending());
    assert!(scanner.latest(NOW + 301.0, true).is_none());
    assert_eq!(percent(finish(&mut scanner, NOW + 302.0).unwrap()), 7);
    assert_eq!(scanner.workload().trees_scanned, 1);
}

#[test]
fn expired_candidates_trigger_a_new_scan_after_cancellation() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "rollout-current.jsonl", &line(7), 10);
    let mut scanner = reader(directory.path(), 1, 256);
    assert!(scanner.latest(NOW, false).is_none());
    scanner.cancel();
    assert!(scanner.latest(NOW + 301.0, false).is_none());
    assert_eq!(scanner.workload().trees_scanned, 2);
    assert_eq!(percent(finish(&mut scanner, NOW + 301.0).unwrap()), 7);
}

#[test]
fn cache_age_can_change_during_a_pending_search() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "rollout-current.jsonl", &line(7), 10);
    let mut scanner = reader(directory.path(), 1, 256);
    assert!(
        scanner
            .latest_with_cache_age(NOW, false, Duration::from_secs(300))
            .is_none()
    );
    assert!(scanner.pending());
    assert!(
        scanner
            .latest_with_cache_age(NOW + 1.0, false, Duration::from_secs(60))
            .is_none()
    );
    assert_eq!(percent(finish(&mut scanner, NOW + 1.0).unwrap()), 7);
    assert_eq!(scanner.workload().trees_scanned, 1);
}

#[test]
fn oversized_and_invalid_lines_do_not_hide_a_valid_reading() {
    let directory = tempfile::tempdir().unwrap();
    let content = format!(
        "{}\n{}\n{}\n",
        line(7),
        "x".repeat(1024 * 1024 + 1),
        "not json \"rate_limits\""
    );
    write(directory.path(), "rollout-current.jsonl", &content, 10);
    let mut scanner = reader(directory.path(), 2 * 1024 * 1024, 256);
    assert_eq!(percent(finish(&mut scanner, NOW).unwrap()), 7);
}

#[test]
fn oversized_line_at_start_of_file_is_discarded() {
    let directory = tempfile::tempdir().unwrap();
    write(
        directory.path(),
        "rollout-current.jsonl",
        &"x".repeat(1024 * 1024 + 1),
        10,
    );
    let mut scanner = reader(directory.path(), 2 * 1024 * 1024, 256);
    assert!(finish(&mut scanner, NOW).is_none());
    assert_eq!(scanner.workload().searches_completed, 1);
}

#[test]
fn large_search_and_tree_respect_the_time_budget() {
    let directory = tempfile::tempdir().unwrap();
    let large = "x".repeat(8 * 1024 * 1024);
    for index in 0..1200 {
        write(
            directory.path(),
            &format!("rollout-{index}.jsonl"),
            if index == 1199 { &large } else { "{}" },
            index as u64,
        );
    }
    let mut scanner = RolloutReader::new(
        directory.path().to_path_buf(),
        Budget {
            bytes: 8 * 1024 * 1024,
            entries: 256,
            time: Duration::from_millis(1),
            cache: 300.0,
        },
    );
    scanner.run_slice();
    assert!(!scanner.pending());
    scanner.start_scan(NOW);
    scanner.run_slice();
    assert!(scanner.tree_pending());
    assert!(scanner.workload().largest_tree_slice_entries <= 256);
    while scanner.tree_pending() {
        scanner.run_slice();
    }
    scanner.run_slice();
    assert!(scanner.pending());
}

#[test]
fn zero_time_budget_defers_the_tree_scan() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "rollout-current.jsonl", &line(42), 10);
    let mut scanner = RolloutReader::new(
        directory.path().to_path_buf(),
        Budget {
            bytes: 1024,
            entries: 256,
            time: Duration::ZERO,
            cache: 300.0,
        },
    );
    scanner.start_scan(NOW);
    scanner.run_slice();
    assert!(scanner.tree_pending());
    assert_eq!(scanner.workload().tree_entries_examined, 0);
}

#[test]
fn shrinking_file_during_a_search_completes_without_a_reading() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(directory.path(), "rollout-current.jsonl", &"x".repeat(4096), 10);
    let mut scanner = reader(directory.path(), 1, 256);
    assert!(scanner.latest(NOW, false).is_none());
    fs::write(path, "").unwrap();
    assert!(finish(&mut scanner, NOW).is_none());
    assert_eq!(scanner.workload().searches_completed, 1);
}

#[test]
fn empty_cache_ignores_new_files_until_expiry() {
    let directory = tempfile::tempdir().unwrap();
    let mut scanner = reader(directory.path(), 1_024, 256);
    assert!(finish(&mut scanner, NOW).is_none());
    write(directory.path(), "rollout-new.jsonl", &line(7), 10);
    assert!(scanner.latest(NOW + 60.0, false).is_none());
    assert_eq!(scanner.workload().trees_scanned, 1);
    assert!(finish(&mut scanner, NOW + 301.0).is_some());
}
