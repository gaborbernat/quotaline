use super::*;

#[test]
fn checkpoint_resumes_without_rereading_and_follows_appends() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("projects");
    fs::create_dir(&root).unwrap();
    let file = root.join("session.jsonl");
    let state = directory.path().join("state.json");
    fs::write(&file, line("one")).unwrap();
    let mut first = reader(&root, Some(&state), Config::default());
    assert_eq!(first.refresh(now(), 60, false).state.message_count(), 1);
    assert_eq!(first.workload().checkpoints, 1);

    let mut resumed = reader(&root, Some(&state), Config::default());
    assert_eq!(resumed.refresh(now(), 60, false).state.message_count(), 1);
    assert_eq!(resumed.workload().bytes_read, 0);
    fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(line("two").as_bytes())
        .unwrap();
    assert_eq!(resumed.refresh(now() + 1.0, 60, false).state.message_count(), 2);
}

#[test]
fn failed_checkpoint_retries_after_backoff() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("session.jsonl"), line("one")).unwrap();
    let blocker = directory.path().join("blocker");
    fs::write(&blocker, b"").unwrap();
    let mut reader = reader(directory.path(), Some(&blocker.join("state.json")), Config::default());
    reader.refresh(now(), 60, false);
    assert_eq!(reader.workload().checkpoint_attempts, 1);
    reader.refresh(now() + 1.0, 60, false);
    assert_eq!(reader.workload().checkpoint_attempts, 1);
    reader.refresh(now() + 61.0, 60, false);
    assert_eq!(reader.workload().checkpoint_attempts, 2);
}

#[test]
fn saved_offset_beyond_file_end_rewinds_and_persists_zero() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, b"").unwrap();
    let state_path = directory.path().join("state.json");
    let mut state = quotaline_claude::claude_transcripts::TranscriptState::default();
    state.set_offset(file.to_string_lossy().into_owned(), 100, 0);
    fs::write(&state_path, state.checkpoint().unwrap()).unwrap();
    let mut reader = reader(
        directory.path(),
        Some(&state_path),
        Config {
            checkpoint_interval: 0.0,
            ..Config::default()
        },
    );
    assert_eq!(reader.refresh(now(), 60, false).state.message_count(), 0);
    assert_eq!(reader.workload().files_opened, 0);
    let restored =
        quotaline_claude::claude_transcripts::TranscriptState::from_checkpoint(&fs::read(state_path).unwrap()).unwrap();
    assert_eq!(restored.offset(file.to_str().unwrap()).unwrap().bytes, 0);
}

#[test]
fn malformed_checkpoint_is_replaced_after_a_valid_read() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("session.jsonl"), line("one")).unwrap();
    let state_path = directory.path().join("state.json");
    fs::write(&state_path, b"{").unwrap();
    let mut reader = reader(directory.path(), Some(&state_path), Config::default());
    assert_eq!(reader.refresh(now(), 60, false).state.message_count(), 1);
    assert!(
        quotaline_claude::claude_transcripts::TranscriptState::from_checkpoint(&fs::read(state_path).unwrap()).is_ok()
    );
}

#[test]
fn checkpoint_waits_for_interval_or_byte_threshold() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("one")).unwrap();
    let state_path = directory.path().join("state.json");
    let mut reader = reader(
        directory.path(),
        Some(&state_path),
        Config {
            checkpoint_interval: 300.0,
            maximum_checkpoint_bytes: 10_000,
            ..Config::default()
        },
    );
    reader.refresh(now(), 60, false);
    fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(line("two").as_bytes())
        .unwrap();
    reader.refresh(now() + 1.0, 60, false);
    assert_eq!(reader.workload().checkpoints, 1);
    reader.refresh(now() + 301.0, 60, false);
    assert_eq!(reader.workload().checkpoints, 2);
}

#[test]
fn byte_threshold_overrides_checkpoint_interval() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("session.jsonl");
    fs::write(&file, line("one")).unwrap();
    let state_path = directory.path().join("state.json");
    let mut reader = reader(
        directory.path(),
        Some(&state_path),
        Config {
            checkpoint_interval: 300.0,
            maximum_checkpoint_bytes: 1,
            ..Config::default()
        },
    );
    reader.refresh(now(), 60, false);
    fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(line("two").as_bytes())
        .unwrap();
    reader.refresh(now() + 1.0, 60, false);
    assert_eq!(reader.workload().checkpoints, 2);
}

#[test]
fn old_project_checkpoint_is_migrated_and_saved() {
    let directory = tempfile::tempdir().unwrap();
    let state_path = directory.path().join("state.json");
    let old = serde_json::json!({
        "offsets": {}, "seenByDay": {}, "days": {}, "recent": {}
    });
    fs::write(&state_path, old.to_string()).unwrap();
    let mut reader = reader(directory.path(), Some(&state_path), Config::default());
    reader.refresh(now(), 60, false);
    let saved = fs::read(&state_path).unwrap();
    assert!(
        !quotaline_claude::claude_transcripts::TranscriptState::from_checkpoint(&saved)
            .unwrap()
            .requires_checkpoint()
    );
    assert_eq!(reader.workload().checkpoints, 1);
}
