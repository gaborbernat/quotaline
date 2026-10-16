//! Snapshot widget projection cases.

use super::*;
use crate::Window;

fn provider(id: &str, status: &str) -> Provider {
    let mut provider = Provider::connected(
        id,
        String::new(),
        String::new(),
        String::new(),
        vec![Window::new(
            "session",
            "Current session",
            87.0,
            Some("tomorrow".into()),
            "session",
        )],
    );
    provider.status = status.into();
    provider.updated = Some(12);
    provider
}

#[test]
fn snapshot_attention_ignores_disabled_providers() {
    let providers = BTreeMap::from([("claude".into(), provider("claude", "sign-in"))]);
    assert!(!snapshot_attention(
        &providers,
        &BTreeMap::from([("claude".into(), false)])
    ));
}

#[test]
fn snapshot_attention_accepts_enabled_authentication_failure() {
    for status in ["sign-in", "auth"] {
        let providers = BTreeMap::from([("claude".into(), provider("claude", status))]);
        assert!(snapshot_attention(
            &providers,
            &BTreeMap::from([("claude".into(), true)])
        ));
    }
}

#[test]
fn snapshot_attention_accepts_unconfigured_authentication_failure() {
    let providers = BTreeMap::from([("claude".into(), provider("claude", "sign-in"))]);
    assert!(snapshot_attention(&providers, &BTreeMap::new()));
}

#[test]
fn snapshot_attention_accepts_missing_sign_in_notice() {
    let mut claude = provider("claude", "connected");
    claude.notice = "No sign-in found. Open settings".into();
    assert!(snapshot_attention(
        &BTreeMap::from([("claude".into(), claude)]),
        &BTreeMap::from([("claude".into(), true)])
    ));
}

#[test]
fn snapshot_attention_ignores_healthy_providers() {
    let providers = BTreeMap::from([("claude".into(), provider("claude", "connected"))]);
    assert!(!snapshot_attention(
        &providers,
        &BTreeMap::from([("claude".into(), true)])
    ));
}
