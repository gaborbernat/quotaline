use super::*;

#[test]
fn report_redaction_applies_account_masking_when_enabled() {
    let identities = vec![DiagnosticIdentity {
        email: Some("person@example.com".into()),
        organization: Some("Acme Corp".into()),
    }];
    assert_eq!(
        redact_diagnostic_report(
            "Acme Corp Bearer secret person@example.com /home/me/private".into(),
            "/home/me".into(),
            identities.clone(),
            true,
        ),
        "workspace Bearer <redacted> account ~/private"
    );
    assert_eq!(
        redact_diagnostic_report("Acme Corp".into(), String::new(), identities, false),
        "Acme Corp"
    );
}

#[test]
fn bridge_formats_structured_diagnostics() {
    assert_eq!(
        diagnostic_event_message(
            "request.finished".into(),
            vec![
                DiagnosticField {
                    key: "operation".into(),
                    value: Some("profile lookup".into())
                },
                DiagnosticField {
                    key: "status".into(),
                    value: None
                },
            ],
            String::new(),
        ),
        "request.finished operation=\"profile lookup\""
    );
}

#[test]
fn bridge_redacts_secrets_and_home_without_losing_report_lines() {
    assert_eq!(
        log_redact("first body=secret\nsecond /home/me/private".into(), "/home/me".into()),
        "first body=<redacted>\nsecond ~/private"
    );
}

#[test]
fn bridge_bounds_and_escapes_diagnostic_messages() {
    let value = log_message(format!("Bearer secret\n{}", "é".repeat(2_000)), "/home/me".into());
    assert!(value.starts_with("Bearer <redacted>\\n"));
    assert!(value.ends_with('…'));
    assert!(value.len() <= quotaline_logging::MAX_MESSAGE_BYTES);
}

#[test]
fn bridge_loads_rotates_rewrites_and_clears_private_logs() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("app.log").to_string_lossy().to_string();
    log_append(
        path.clone(),
        vec![
            "[2026-08-29 19:00:00.000] [warning] [tabs] first".into(),
            "followup".into(),
        ],
        100,
        2,
    )
    .unwrap();
    let loaded = log_load(path.clone(), 2, 100, 10, 0.0, "/home/me".into(), Vec::new());
    assert_eq!(loaded.entries.len(), 2);
    assert_eq!(loaded.entries[0].category, "tabs");
    assert_eq!(loaded.entries[1].timestamp, loaded.entries[0].timestamp);
    log_rewrite(path.clone(), vec!["replacement".into()], 100, 2).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "replacement\n");
    log_clear(path.clone(), 2).unwrap();
    assert!(!std::path::Path::new(&path).exists());
}

#[test]
fn bridge_creates_log_entry_and_keeps_live_identity() {
    let record = log_make_entry(
        1_700_000_000.123,
        "warning".into(),
        "tabs".into(),
        "Bearer abc".into(),
        String::new(),
    );
    assert_eq!(record.message, "Bearer <redacted>");
    assert_eq!(
        record.line,
        "[2023-11-14 22:13:20.123] [warning] [tabs] Bearer <redacted>"
    );

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("app.log").to_string_lossy().to_string();
    log_append(path.clone(), vec![record.line.clone()], 200, 2).unwrap();
    let loaded = log_load(path, 2, 200, 10, 0.0, String::new(), vec![record.clone()]);
    assert_eq!(loaded.entries[0].sequence_id, record.sequence_id);
    assert_eq!(loaded.entries[0].line, record.line);
}

#[test]
fn bridge_reports_file_write_failures() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("blocked");
    std::fs::write(&parent, "not a directory").unwrap();
    let error = log_append(
        parent.join("app.log").to_string_lossy().into(),
        vec!["entry".into()],
        100,
        1,
    )
    .unwrap_err();
    assert!(error.to_string().contains("Log storage failed"));
    assert!(
        log_rewrite(
            parent.join("app.log").to_string_lossy().into(),
            vec!["entry".into()],
            100,
            1,
        )
        .is_err()
    );
}
