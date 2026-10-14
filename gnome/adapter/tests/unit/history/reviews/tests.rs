//! GNOME review storage cases.

use super::*;
use crate::history::{Point, Series};

#[test]
fn demo_review_counts_use_shared_aggregation() {
    let now = Utc::now();
    let store = Store::open(None, 60).unwrap();
    store
        .seed_demo(&["codex".into()], now.timestamp() as f64, false)
        .unwrap();
    let counts = review_counts(&store, &BTreeMap::new(), 60, now).unwrap();
    let counts = counts.unwrap();
    assert!(counts.today > 0);
    assert!(counts.week >= counts.today);
}

#[test]
fn history_review_counts_ignore_other_providers() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-10-13T12:00:00Z")
        .unwrap()
        .to_utc();
    let at = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let history = History {
        metric: "codeReviews".into(),
        start: at,
        end: at,
        series: vec![
            Series {
                id: "claude:reviews".into(),
                provider: "claude".into(),
                label: "reviews".into(),
                points: vec![Point(at, 20.0, None, false)],
            },
            Series {
                id: "codex:reviews".into(),
                provider: "codex".into(),
                label: "reviews".into(),
                points: vec![Point(at, 3.0, None, false)],
            },
        ],
        canonical_total: None,
    };
    assert_eq!(
        counts_from_history(&history, now),
        Some(analytics::ReviewCounts { today: 3, week: 3 })
    );
}

#[test]
fn history_without_codex_reviews_has_no_summary() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-10-13T12:00:00Z")
        .unwrap()
        .to_utc();
    let history = History {
        metric: "codeReviews".into(),
        start: 0,
        end: 0,
        series: Vec::new(),
        canonical_total: None,
    };
    assert_eq!(counts_from_history(&history, now), None);
}
