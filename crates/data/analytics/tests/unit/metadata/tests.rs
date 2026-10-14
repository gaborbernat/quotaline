//! Checks the shared metric catalog and chart policy.

use super::*;

#[test]
fn catalog_covers_every_metric_once() {
    let definitions = metric_definitions();
    assert_eq!(definitions.len(), METRICS.len() + 1);
    assert_eq!(definitions[0].id, "windowUsagePercent");
    assert_eq!(
        definitions[0].suppliers,
        ["antigravity", "claude", "codex", "copilot", "cursor", "gemini"]
    );
    let ids = definitions
        .iter()
        .map(|definition| definition.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), definitions.len());
    assert!(METRICS.iter().all(|(id, _, _, _)| ids.contains(id)));
}

#[test]
fn chart_policy_matches_metric_semantics() {
    let definitions = metric_definitions();
    let find = |id| definitions.iter().find(|definition| definition.id == id).unwrap();
    let usage = find("windowUsagePercent");
    assert_eq!(
        (&usage.title[..], &usage.mark[..], &usage.unit[..], &usage.summary[..]),
        ("Usage %", "stepLine", "percentage", "latest")
    );
    assert!(!usage.daily_utc);
    assert!(!usage.stackable);
    assert_eq!(usage.attribution, "Every model · step line · selected time zone");
    let shared = find("inputTokens");
    assert_eq!(shared.suppliers, ["claude", "codex"]);
    assert_eq!(shared.group, "Claude and Codex");
    assert!(shared.stackable);
    assert!(shared.daily_utc);
    assert_eq!(shared.unit, "tokens");
    assert_eq!(shared.summary, "sum");
    let balance = find("creditBalance");
    assert_eq!(
        (&balance.mark[..], &balance.summary[..], &balance.unit[..]),
        ("line", "latest", "credits")
    );
    assert!(!balance.stackable);
    let parallel = find("turns");
    assert!(parallel.parallel);
    assert!(!parallel.stackable);
    assert_eq!(find("costUSD").unit, "usd");
    assert_eq!(find("projectCost").group, "Projects");
    assert_eq!(find("projectCost").suppliers, ["claude"]);
    assert_eq!(find("surfaceUsagePercent").unit, "percentage");
    assert_eq!(find("surfaceUsagePercent").summary, "latest");
    assert_eq!(find("messages").attribution, "Claude · one series · daily UTC");
}
