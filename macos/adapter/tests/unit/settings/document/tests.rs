//! Keeps the macOS settings document valid across launches.

use super::*;

fn document(value: serde_json::Value) -> Option<Vec<u8>> {
    Some(serde_json::to_vec(&value).unwrap())
}

#[test]
fn missing_or_unreadable_documents_open_with_the_defaults() {
    for stored in [None, Some(b"not json".to_vec())] {
        let record = RustSettings::new(stored).record();
        assert_eq!(record.format, "stacked");
        assert_eq!(record.retention_days, 60);
    }
}

#[test]
fn stored_documents_reopen_with_their_values() {
    let settings = RustSettings::new(document(serde_json::json!({"format": "inline", "decimals": 1})));
    let reopened = RustSettings::new(Some(settings.document())).record();
    assert_eq!(reopened.format, "inline");
    assert_eq!(reopened.decimals, 1);
}

#[test]
fn updates_return_the_normalized_record() {
    let settings = RustSettings::new(None);
    let record = SettingsRecord {
        retention_days: 1,
        notify_thresholds: vec![100, 75, 75],
        ..settings.record()
    };
    let updated = settings.update(record).unwrap();
    assert_eq!(updated.retention_days, 7);
    assert_eq!(updated.notify_thresholds, [75, 100]);
    assert_eq!(settings.record(), updated);
}

#[test]
fn invalid_updates_leave_the_document_unchanged() {
    let settings = RustSettings::new(None);
    let record = SettingsRecord {
        format: "unknown".into(),
        ..settings.record()
    };
    assert_eq!(
        settings.update(record),
        Err(SettingsError::Invalid {
            message: "Invalid format".into()
        })
    );
    assert_eq!(settings.record().format, "stacked");
}

#[test]
fn updates_keep_values_only_gnome_edits() {
    let settings = RustSettings::new(document(serde_json::json!({"widget_size": "large"})));
    settings
        .update(SettingsRecord {
            hide_zero: false,
            ..settings.record()
        })
        .unwrap();
    let stored: serde_json::Value = serde_json::from_slice(&settings.document()).unwrap();
    assert_eq!(stored["widget_size"], "large");
    assert_eq!(stored["hide_zero"], false);
}

#[test]
fn reset_restores_defaults_but_keeps_the_open_tab() {
    let settings = RustSettings::new(document(serde_json::json!({"last_tab": "History", "decimals": 2})));
    let record = settings.reset();
    assert_eq!(record.last_tab, "History");
    assert_eq!(record.decimals, 0);
}
