use super::*;

fn row() -> WidgetRow {
    WidgetRow {
        key: WidgetKey {
            provider: "claude".into(),
            window_id: "session".into(),
        },
        provider_name: "Claude".into(),
        label: "Current session".into(),
        used_percent: 42.0,
        resets_at: Some(1000.0),
    }
}

fn content() -> WidgetContent {
    WidgetContent {
        rows: vec![row()],
        attention: false,
        display: "Used".into(),
    }
}

#[test]
fn widget_projection_retains_selection_order_and_skips_missing_windows() {
    let projection = widget_projection(
        vec![WidgetProvider {
            id: "claude".into(),
            name: "Claude".into(),
            windows: vec![WidgetWindow {
                id: "session".into(),
                label: "Current session".into(),
                used_percent: 42.0,
                resets_at: Some(1000.0),
            }],
        }],
        &[
            WidgetKey {
                provider: "codex".into(),
                window_id: "weekly".into(),
            },
            WidgetKey {
                provider: "claude".into(),
                window_id: "session".into(),
            },
            WidgetKey {
                provider: "claude".into(),
                window_id: "missing".into(),
            },
        ],
        true,
    );
    assert_eq!(projection.rows, vec![row()]);
    assert!(projection.attention);
    assert!(!widget_projection(vec![], &[], false).attention);
}

#[test]
fn widget_publication_follows_content_and_freshness() {
    let current = content();
    assert!(widget_same_content(&current, &current));
    assert!(widget_should_publish(&current, 1000.0, None, 900.0));
    assert!(!widget_should_publish(
        &current,
        1100.0,
        Some((&current, 1000.0)),
        900.0
    ));
    assert!(widget_should_publish(&current, 1900.0, Some((&current, 1000.0)), 900.0));
    for changed in [
        WidgetContent {
            rows: vec![],
            ..current.clone()
        },
        WidgetContent {
            attention: true,
            ..current.clone()
        },
        WidgetContent {
            display: "Remaining".into(),
            ..current.clone()
        },
    ] {
        assert!(!widget_same_content(&current, &changed));
        assert!(widget_should_publish(&current, 1000.0, Some((&changed, 1000.0)), 900.0));
    }
}
