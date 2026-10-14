use super::*;

#[test]
fn incomplete_tail_is_read_after_append() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    let message = line("one");
    let split = message.len() / 2;
    fs::write(&file, &message[..split]).unwrap();
    let mut reader = reader(directory.path(), None, Config::default());
    let partial = reader.refresh(now(), 60, false);
    assert_eq!(partial.state.message_count(), 0);
    assert!(!partial.projects_complete);
    assert_eq!(reader.workload().retained_partial_files, 1);
    fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(&message.as_bytes()[split..])
        .unwrap();
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 1);
    assert_eq!(reader.workload().retained_partial_files, 0);
}

#[test]
fn truncated_file_rewinds_from_zero() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("first-long-identifier")).unwrap();
    let mut reader = reader(directory.path(), None, Config::default());
    assert_eq!(reader.refresh(now(), 60, false).state.message_count(), 1);
    fs::write(&file, line("two")).unwrap();
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 2);
}

#[test]
fn cancel_restarts_discovery() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("session.jsonl"), line("one")).unwrap();
    let mut reader = reader(
        directory.path(),
        None,
        Config {
            entry_budget: 1,
            ..Config::default()
        },
    );
    reader.refresh(now(), 60, false);
    assert!(reader.pending());
    reader.cancel();
    assert!(!reader.pending());
    assert_eq!(reader.refresh(now(), 60, false).state.message_count(), 1);
    finish(&mut reader);
    assert_eq!(reader.workload().scans_started, 2);
}

#[test]
fn retained_partial_memory_evicts_other_files() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["a", "b", "c", "d"] {
        fs::write(directory.path().join(format!("{name}.jsonl")), b"1").unwrap();
    }
    let mut reader = reader(
        directory.path(),
        None,
        Config {
            maximum_line_bytes: 1,
            maximum_retained_partial_bytes: 3,
            ..Config::default()
        },
    );
    reader.refresh(now(), 60, false);
    assert_eq!(reader.workload().retained_partial_files, 3);
    assert_eq!(reader.workload().retained_partial_bytes, 3);
}

#[test]
fn incomplete_cold_file_does_not_retain_its_tail() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..=super::super::MAX_INDEXED_FILES {
        let file = directory.path().join(format!("session-{index:03}.jsonl"));
        fs::write(&file, b"partial").unwrap();
        fs::File::open(&file)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + Duration::from_secs(now() as u64)))
            .unwrap();
    }
    let mut reader = reader(
        directory.path(),
        None,
        Config {
            entry_budget: super::super::MAX_INDEXED_FILES,
            time_budget: Duration::from_secs(2),
            ..Config::default()
        },
    );
    let snapshot = reader.refresh(now(), 60, false);
    let cold = (0..=super::super::MAX_INDEXED_FILES)
        .map(|index| directory.path().join(format!("session-{index:03}.jsonl")))
        .find(|path| snapshot.state.offset(path.to_str().unwrap()).is_none())
        .unwrap();
    fs::File::open(cold)
        .unwrap()
        .set_times(
            fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + Duration::from_secs(now() as u64 - 86_400)),
        )
        .unwrap();
    finish(&mut reader);
    assert_eq!(
        reader.workload().retained_partial_files,
        super::super::MAX_INDEXED_FILES as u64
    );
}

fn paused_scan(path: &Path) -> Reader {
    let mut reader = reader(
        path,
        None,
        Config {
            byte_budget: 1,
            entry_budget: 1,
            ..Config::default()
        },
    );
    reader.refresh(now(), 60, false);
    assert!(reader.pending());
    reader
}

#[test]
fn file_removed_during_a_scan_is_skipped() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("one")).unwrap();
    let mut reader = paused_scan(directory.path());
    fs::remove_file(&file).unwrap();
    finish(&mut reader);
    assert_eq!(reader.workload().files_opened, 1);
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 0);
}

#[cfg(unix)]
#[test]
fn file_replaced_with_directory_during_a_scan_is_skipped() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("one")).unwrap();
    let mut reader = paused_scan(directory.path());
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    finish(&mut reader);
    assert_eq!(reader.workload().files_opened, 2);
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 0);
}

#[test]
fn file_truncated_during_a_scan_does_not_consume_stale_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("one")).unwrap();
    let mut reader = paused_scan(directory.path());
    fs::write(&file, b"").unwrap();
    finish(&mut reader);
    assert_eq!(reader.workload().bytes_read, 1);
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn file_replaced_with_pipe_during_a_scan_is_skipped() {
    use std::{ffi::CString, os::unix::fs::OpenOptionsExt};

    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("one")).unwrap();
    let mut reader = paused_scan(directory.path());
    fs::remove_file(&file).unwrap();
    let path = CString::new(file.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let _writer = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&file)
        .unwrap();
    finish(&mut reader);
    assert_eq!(reader.workload().files_opened, 2);
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 0);
}

#[test]
fn oversized_incomplete_line_is_discarded_after_newline() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, b"oversized").unwrap();
    let mut reader = reader(
        directory.path(),
        None,
        Config {
            maximum_line_bytes: 2,
            ..Config::default()
        },
    );
    let partial = reader.refresh(now(), 60, false);
    assert!(!partial.projects_complete);
    assert_eq!(reader.workload().retained_partial_files, 1);
    fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    assert!(reader.refresh(now() + 1.0, 60, false).projects_complete);
    assert_eq!(reader.workload().retained_partial_files, 0);
}

#[test]
fn complete_line_at_the_byte_budget_keeps_following_bytes_for_later() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    let first = line("one");
    fs::write(&file, format!("{first}{}", line("two"))).unwrap();
    let mut reader = reader(
        directory.path(),
        None,
        Config {
            byte_budget: first.len(),
            ..Config::default()
        },
    );
    assert_eq!(reader.refresh(now(), 60, false).state.message_count(), 1);
    assert!(reader.pending());
    finish(&mut reader);
    assert_eq!(reader.refresh(now() + 1.0, 60, false).state.message_count(), 2);
}
