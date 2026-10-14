//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn anonymous_project_uses_the_first_six_digest_bytes() {
    assert_eq!(anonymous("/work/repo"), "Project ddc5e473a09b");
}

#[test]
fn export_hides_only_project_metrics_when_requested() {
    let identity = "/work/repo";
    for (metric, hide, expected) in [
        ("projectCost", true, anonymous(identity)),
        ("projectMessages", true, anonymous(identity)),
        ("modelCost", true, identity.into()),
        ("projectCost", false, identity.into()),
    ] {
        assert_eq!(export_label(identity, metric, hide), expected, "{metric} {hide}");
    }
}

#[test]
fn project_labels_use_the_shortest_unique_suffix() {
    let identities = ["/a/one/repo", "/b/one/repo", "/a/other", "/repo", "repo", "/"].map(str::to_owned);
    assert_eq!(
        labels(&identities, false),
        BTreeMap::from([
            ("/a/one/repo".into(), "a/one/repo".into()),
            ("/b/one/repo".into(), "b/one/repo".into()),
            ("/a/other".into(), "other".into()),
            ("/repo".into(), "/repo".into()),
            ("repo".into(), "repo".into()),
            ("/".into(), "/".into()),
        ])
    );
}

#[test]
fn project_labels_deduplicate_and_hide_complete_paths() {
    let identities = ["/private/client/repo", "/private/other/repo", "/private/client/repo"].map(str::to_owned);
    assert_eq!(
        labels(&identities, true),
        BTreeMap::from([
            (identities[0].clone(), anonymous(&identities[0])),
            (identities[1].clone(), anonymous(&identities[1])),
        ])
    );
}

#[test]
fn empty_project_set_has_no_labels() {
    assert!(labels(&[], false).is_empty());
}
