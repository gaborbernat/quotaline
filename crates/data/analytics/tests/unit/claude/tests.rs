//! Verifies shared Claude history metrics, ordering, and missing costs.

use super::*;

#[test]
fn projects_all_claude_metrics_in_day_order() {
    let points = claude_points(
        &[
            ModelTotal {
                day: "2026-10-13".into(),
                model: "sonnet".into(),
                input: 2,
                cache_read: 3,
                output: 4,
                cache_write: 5,
                cost: Some(1.25),
            },
            ModelTotal {
                day: "2026-10-12".into(),
                model: "opus".into(),
                input: 6,
                cache_read: 7,
                output: 8,
                cache_write: 9,
                cost: None,
            },
        ],
        &[DayTotal {
            day: "2026-10-13".into(),
            messages: 10,
            sessions: 11,
            tool_calls: 12,
        }],
        &[ProjectTotal {
            day: "2026-10-13".into(),
            project: "app".into(),
            messages: 13,
            cost: Some(14.0),
        }],
    );
    assert_eq!(points.len(), 14);
    assert!(points.iter().all(|point| point.provider == "claude"));
    assert_eq!(points.first().unwrap().day, "2026-10-12");
    assert_eq!(points.last().unwrap().metric, "toolCalls");
    assert!(
        points
            .iter()
            .any(|point| point.metric == "costUSD" && point.value == 1.25)
    );
    assert!(
        points
            .iter()
            .any(|point| point.metric == "projectCost" && point.value == 14.0)
    );
    assert!(
        points
            .iter()
            .any(|point| point.metric == "projectMessages" && point.value == 13.0)
    );
    assert!(
        points
            .iter()
            .any(|point| point.metric == "sessions" && point.value == 11.0)
    );
}

#[test]
fn missing_costs_do_not_create_zero_cost_metrics() {
    let points = claude_points(
        &[],
        &[],
        &[ProjectTotal {
            day: "2026-10-13".into(),
            project: "app".into(),
            messages: 1,
            cost: None,
        }],
    );
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].metric, "projectMessages");
}
