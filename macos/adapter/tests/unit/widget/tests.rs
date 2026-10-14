use super::*;
use crate::test_records as records;

#[test]
fn widget_bridge_projects_rows_and_publication_policy() {
    let projection = project_widget(
        vec![records::provider(
            "claude",
            vec![crate::provider_record::ProviderWindowRecord {
                label: "Current session".into(),
                ..records::window("session", 42.0, Some(1000.0))
            }],
        )],
        vec![WidgetKey {
            provider: "claude".into(),
            window_id: "session".into(),
        }],
        vec![Availability::AuthenticationRequired],
    );
    assert_eq!(projection.rows[0].key.window_id, "session");
    assert_eq!(projection.rows[0].provider_name, "Claude");
    assert_eq!(projection.rows[0].label, "Current session");
    assert_eq!(projection.rows[0].used_percent, 42.0);
    assert_eq!(projection.rows[0].resets_at, Some(1000.0));
    assert!(projection.attention);

    let content = WidgetContent {
        rows: projection.rows,
        attention: true,
        display: "Used".into(),
    };
    assert!(widget_same_content(content.clone(), content.clone()));
    assert!(widget_should_publish(content.clone(), 1000.0, None, 900.0));
    let previous = WidgetPreviousRecord {
        content: content.clone(),
        updated_at: 1000.0,
    };
    assert!(!widget_should_publish(
        content.clone(),
        1100.0,
        Some(previous.clone()),
        900.0
    ));
    assert!(widget_should_publish(content, 1900.0, Some(previous), 900.0));
}
