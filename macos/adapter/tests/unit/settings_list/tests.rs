use super::{SettingsListRequestRecord, settings_model_groups};
use crate::test_records as records;

#[test]
fn bridge_projects_settings_rows() {
    let rows = settings_model_groups(SettingsListRequestRecord {
        readings: vec![records::reading(
            "claude",
            vec![
                records::window("session", 30.0, None),
                records::window("weekly", 0.0, None),
            ],
            100.0,
        )],
        last_used_at: [("claude:weekly".to_string(), 50.0)].into(),
        selected: vec!["claude:session".into()],
        overrides: Default::default(),
        provider_order: vec!["claude".into()],
        model_order: vec!["claude:session".into(), "claude:weekly".into()],
        query: "".into(),
        hide_unused: false,
        revealed: None,
    });

    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].selected_count, rows[0].total_count), (1, 2));
    assert_eq!(rows[0].rows[0].key, "claude:session");
    assert_eq!(rows[0].rows[0].last_used_at, Some(100.0));
    assert_eq!(rows[0].rows[1].last_used_at, Some(50.0));
}
