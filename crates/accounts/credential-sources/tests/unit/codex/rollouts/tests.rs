//! Checks codex rollouts behavior through the public API.

use super::*;
use std::fs::File;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde_json::json;

thread_local! {
    static CACHE: RolloutCache = RolloutCache::default();
}

fn latest(root: &Path, throttled: bool) -> io::Result<Option<Reading>> {
    CACHE.with(|cache| cache.latest(root, throttled))
}

fn latest_with_cache_age(root: &Path, throttled: bool, cache_age: Duration) -> io::Result<Option<Reading>> {
    CACHE.with(|cache| cache.latest_with_cache_age(root, throttled, cache_age))
}

#[test]
fn missing_sessions_have_no_local_reading() {
    let root = tempfile::tempdir().unwrap();
    assert!(latest(root.path(), false).unwrap().is_none());
}

#[test]
fn nested_rollout_parses_string_windows_and_observed_time() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions/2026/10/10");
    fs::create_dir_all(&sessions).unwrap();
    let event = json!({"timestamp":"2026-10-10T10:00:00Z","payload":{"events":[
        {"rate_limits":{"plan_type":"pro","primary":{"used_percent":"42","window_minutes":300,
            "reset_at":"1791630000"},"secondary":{"used_percent":7,"limit_window_seconds":604800}}}
    ]}});
    fs::write(sessions.join("rollout-one.jsonl"), format!("invalid\n{event}\n")).unwrap();
    let reading = latest(root.path(), false).unwrap().unwrap();
    assert_eq!(reading.data["rate_limit"]["primary_window"]["used_percent"], 42.0);
    assert_eq!(
        reading.data["rate_limit"]["primary_window"]["reset_at"],
        1_791_630_000.0
    );
    assert_eq!(
        reading.data["rate_limit"]["secondary_window"]["limit_window_seconds"],
        604_800.0
    );
    assert_eq!(reading.observed, Some(1_791_626_400));
    assert_eq!(latest(root.path(), false).unwrap().unwrap().path, reading.path);
}

#[test]
fn secondary_window_survives_an_invalid_primary_window() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("rollout-secondary.jsonl"),
        json!({"rate_limits":{"primary":{"used_percent":"invalid"},
                "secondary":{"used_percent":35,"reset_at":1791630000.0}}})
        .to_string(),
    )
    .unwrap();
    let reading = latest(root.path(), false).unwrap().unwrap();
    assert!(reading.data["rate_limit"].get("primary_window").is_none());
    assert_eq!(reading.data["rate_limit"]["secondary_window"]["used_percent"], 35.0);
    assert_eq!(
        reading.data["rate_limit"]["secondary_window"]["reset_at"],
        1_791_630_000.0
    );
}

#[test]
fn edited_rollout_replaces_a_cached_reading() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let path = sessions.join("rollout-live.jsonl");
    fs::write(
        &path,
        json!({"rate_limits":{"primary":{"used_percent":25}}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        latest(root.path(), false).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        25.0
    );
    let next_modified = fs::metadata(&path).unwrap().modified().unwrap() + Duration::from_secs(1);
    fs::write(
        &path,
        json!({"rate_limits":{"primary":{"used_percent":75}}}).to_string(),
    )
    .unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(next_modified)
        .unwrap();
    assert_eq!(
        latest(root.path(), false).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        75.0
    );
}

#[test]
fn resized_rollout_replaces_a_cached_reading_with_the_same_modified_time() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let path = sessions.join("rollout-live.jsonl");
    fs::write(&path, json!({"rate_limits":{"primary":{"used_percent":5}}}).to_string()).unwrap();
    assert_eq!(
        latest(root.path(), false).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        5.0
    );
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    fs::write(
        &path,
        json!({"rate_limits":{"primary":{"used_percent":100}}}).to_string(),
    )
    .unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    assert_eq!(
        latest(root.path(), false).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        100.0
    );
}

#[test]
fn removed_rollout_drops_a_cached_reading() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let path = sessions.join("rollout-live.jsonl");
    fs::write(
        &path,
        json!({"rate_limits":{"primary":{"used_percent":25}}}).to_string(),
    )
    .unwrap();
    assert!(latest(root.path(), false).unwrap().is_some());
    fs::remove_file(path).unwrap();
    assert!(latest(root.path(), false).unwrap().is_none());
}

#[test]
fn newest_unreadable_rollout_falls_back_to_older_file() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let older = sessions.join("rollout-older.jsonl");
    fs::write(
        &older,
        json!({"rate_limits":{"primary":{"used_percent":25}}}).to_string(),
    )
    .unwrap();
    File::options()
        .write(true)
        .open(&older)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH)
        .unwrap();
    fs::write(sessions.join("rollout-newer.jsonl"), "not a rollout\n").unwrap();
    let reading = latest(root.path(), false).unwrap().unwrap();
    assert_eq!(reading.path, older);
    assert_eq!(reading.data["rate_limit"]["primary_window"]["used_percent"], 25.0);
}

#[test]
fn oversized_rollout_uses_only_its_tail() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let event = json!({"rate_limits":{"primary":{"used_percent":80}}});
    fs::write(
        sessions.join("rollout-large.jsonl"),
        format!("{}\n{event}\n", "x".repeat(MAX_TAIL as usize)),
    )
    .unwrap();
    let reading = latest(root.path(), false).unwrap().unwrap();
    assert_eq!(reading.data["rate_limit"]["primary_window"]["used_percent"], 80.0);
}

#[test]
fn malformed_or_incomplete_rollouts_do_not_publish_readings() {
    for contents in [
        "{\"rate_limits\":",
        "{\"rate_limits\":{}}",
        "{\"rate_limits\":false}",
        "{\"payload\":{\"rate_limits\":[]}}",
    ] {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(sessions.join("rollout-invalid.jsonl"), contents).unwrap();
        assert!(latest(root.path(), false).unwrap().is_none(), "{contents}");
    }
}

#[test]
fn hidden_and_unrelated_session_files_are_ignored() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let event = json!({"rate_limits":{"primary":{"used_percent":50}}}).to_string();
    fs::write(sessions.join(".rollout-hidden.jsonl"), &event).unwrap();
    fs::write(sessions.join("other.jsonl"), &event).unwrap();
    fs::write(sessions.join("rollout-plain.txt"), event).unwrap();
    assert!(latest(root.path(), false).unwrap().is_none());
}

#[cfg(unix)]
#[test]
fn linked_rollout_outside_the_session_directory_is_ignored() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let outside = root.path().join("outside.jsonl");
    fs::write(
        &outside,
        json!({"rate_limits":{"primary":{"used_percent":50}}}).to_string(),
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, sessions.join("rollout-linked.jsonl")).unwrap();
    assert!(latest(root.path(), false).unwrap().is_none());
}

#[test]
fn unreadable_sessions_path_returns_an_error() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("sessions"), "not a directory").unwrap();
    assert_eq!(
        latest(root.path(), false).err().unwrap().kind(),
        io::ErrorKind::NotADirectory
    );
}

#[cfg(unix)]
#[test]
fn inaccessible_sessions_path_returns_a_permission_error() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("sessions")).unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o000)).unwrap();
    let result = latest(root.path(), false);
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn truncated_first_line_is_not_treated_as_a_complete_rollout() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let event = json!({"rate_limits":{"primary":{"used_percent":50}},"padding":"x".repeat(MAX_TAIL as usize)});
    fs::write(sessions.join("rollout-truncated.jsonl"), event.to_string()).unwrap();
    assert!(latest(root.path(), false).unwrap().is_none());
}

#[test]
fn cache_evicts_old_roots_before_their_absent_result_expires() {
    let first = tempfile::tempdir().unwrap();
    assert!(latest(first.path(), false).unwrap().is_none());
    let others: Vec<_> = (0..5).map(|_| tempfile::tempdir().unwrap()).collect();
    for root in &others {
        assert!(latest(root.path(), false).unwrap().is_none());
    }
    let sessions = first.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("rollout-after-eviction.jsonl"),
        json!({"rate_limits":{"primary":{"used_percent":55}}}).to_string(),
    )
    .unwrap();
    let reading = latest(first.path(), false).unwrap().unwrap();
    assert_eq!(reading.data["rate_limit"]["primary_window"]["used_percent"], 55.0);
}

#[test]
fn absent_rollout_is_discovered_after_the_cache_expires() {
    let root = tempfile::tempdir().unwrap();
    assert!(latest(root.path(), false).unwrap().is_none());
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("rollout-new.jsonl"),
        json!({"rate_limits":{"primary":{"used_percent":55}}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        latest_with_cache_age(root.path(), false, Duration::ZERO)
            .unwrap()
            .unwrap()
            .data["rate_limit"]["primary_window"]["used_percent"],
        55.0
    );
}

#[test]
fn power_saver_keeps_the_known_rollout_until_normal_scanning_resumes() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("rollout-known.jsonl"),
        json!({"rate_limits":{"primary":{"used_percent":7}}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        latest(root.path(), false).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        7.0
    );
    let newer = sessions.join("rollout-new.jsonl");
    fs::write(
        &newer,
        json!({"rate_limits":{"primary":{"used_percent":42}}}).to_string(),
    )
    .unwrap();
    File::options()
        .write(true)
        .open(&newer)
        .unwrap()
        .set_modified(SystemTime::now() + Duration::from_secs(10))
        .unwrap();
    assert_eq!(
        latest_with_cache_age(root.path(), true, Duration::ZERO)
            .unwrap()
            .unwrap()
            .data["rate_limit"]["primary_window"]["used_percent"],
        7.0
    );
    assert_eq!(
        latest_with_cache_age(root.path(), false, Duration::ZERO)
            .unwrap()
            .unwrap()
            .data["rate_limit"]["primary_window"]["used_percent"],
        42.0
    );
}

#[test]
fn power_saver_rereads_a_changed_known_rollout() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let path = sessions.join("rollout-known.jsonl");
    fs::write(&path, json!({"rate_limits":{"primary":{"used_percent":7}}}).to_string()).unwrap();
    assert!(latest(root.path(), true).unwrap().is_some());
    fs::write(
        &path,
        json!({"rate_limits":{"primary":{"used_percent":55}}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        latest(root.path(), true).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        55.0
    );
}

#[test]
fn power_saver_recovers_when_the_known_rollout_is_removed() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let path = sessions.join("rollout-known.jsonl");
    fs::write(&path, json!({"rate_limits":{"primary":{"used_percent":7}}}).to_string()).unwrap();
    assert!(latest(root.path(), true).unwrap().is_some());
    fs::remove_file(path).unwrap();
    fs::write(
        sessions.join("rollout-new.jsonl"),
        json!({"rate_limits":{"primary":{"used_percent":42}}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        latest(root.path(), true).unwrap().unwrap().data["rate_limit"]["primary_window"]["used_percent"],
        42.0
    );
}

#[cfg(unix)]
#[test]
fn power_saver_ignores_a_replaced_rollout_symlink() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let path = sessions.join("rollout-known.jsonl");
    let outside = root.path().join("outside.jsonl");
    fs::write(&path, json!({"rate_limits":{"primary":{"used_percent":7}}}).to_string()).unwrap();
    fs::write(
        &outside,
        json!({"rate_limits":{"primary":{"used_percent":90}}}).to_string(),
    )
    .unwrap();
    assert!(latest(root.path(), true).unwrap().is_some());
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(outside, path).unwrap();
    assert!(latest(root.path(), true).unwrap().is_none());
}

#[test]
fn power_saver_allows_discovery_after_an_empty_first_scan() {
    let root = tempfile::tempdir().unwrap();
    assert!(latest(root.path(), true).unwrap().is_none());
    let sessions = root.path().join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("rollout-new.jsonl"),
        json!({"rate_limits":{"primary":{"used_percent":42}}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        latest_with_cache_age(root.path(), true, Duration::ZERO)
            .unwrap()
            .unwrap()
            .data["rate_limit"]["primary_window"]["used_percent"],
        42.0
    );
}
