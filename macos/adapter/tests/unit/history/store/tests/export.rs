//! Checks CSV exports through the same API Swift calls.

use super::*;

#[test]
fn export_bridge_writes_samples_and_analytics() {
    let folder = tempfile::tempdir().unwrap();
    let database = folder.path().join("history.sqlite3");
    let store = RustHistoryStore::new(Some(database.to_string_lossy().into_owned()), 60).unwrap();
    let now = at("2026-08-29T12:00:00Z");
    store.seed(vec![sample(now, 20.0)]).unwrap();
    store
        .record_analytics(
            "codex".into(),
            None,
            now,
            vec![Point {
                provider: "codex".into(),
                day: "2026-08-29".into(),
                metric: "turns".into(),
                series: "total".into(),
                value: 3.0,
            }],
            vec![],
        )
        .unwrap();
    let path = folder.path().join("export.csv");
    let destination = path.to_string_lossy().into_owned();
    store.export_all(destination.clone(), false).unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().contains("claude:session"));
    store.export_samples(destination.clone(), now, now, None, true).unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().contains("claude:session"));
    store
        .export_analytics(
            destination,
            "turns".into(),
            vec!["codex".into()],
            now - 86_400.0,
            now + 86_400.0,
            false,
        )
        .unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().contains("codex"));
}
