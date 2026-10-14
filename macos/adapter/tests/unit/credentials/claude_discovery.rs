use super::*;
use crate::credentials::{CredentialSourceReader, CredentialSourceScan};

#[test]
fn bridge_returns_the_platform_keychain_label() {
    assert_eq!(
        claude_service_label("Claude Code-credentials-1234".into(), "Claude Code-credentials".into()),
        "configuration 1234"
    );
}

struct Reader(Vec<CredentialSourceScan>);

impl CredentialSourceReader for Reader {
    fn load(&self, index: u32) -> CredentialSourceScan {
        self.0[index as usize].clone()
    }
}

#[test]
fn claude_discovery_lists_scans_and_selects_configured_service() {
    let discovery = ClaudeServiceDiscovery::new("default".into());
    let names = discovery.service_names(vec!["other".into(), "default".into()]);
    assert_eq!(names, ["default", "other"]);
    let scan = discovery.scan(
        names,
        std::sync::Arc::new(Reader(vec![
            CredentialSourceScan::Present {
                access_token: "token".into(),
                refresh_token: None,
                expires_at: None,
            },
            CredentialSourceScan::Incomplete,
        ])),
    );
    assert_eq!(scan.available, [0]);
    assert_eq!(scan.account_error, None);
    assert!(scan.incomplete);
    assert_eq!(
        discovery.select(scan),
        ClaudeServiceSelection {
            selected: Some(0),
            error: None,
            incomplete: false,
            save_service: "default".into()
        }
    );
}

#[test]
fn claude_discovery_reports_read_failure_before_incomplete() {
    let discovery = ClaudeServiceDiscovery::new("default".into());
    let scan = discovery.scan(
        discovery.service_names(vec!["other".into()]),
        std::sync::Arc::new(Reader(vec![
            CredentialSourceScan::Failed,
            CredentialSourceScan::Incomplete,
        ])),
    );
    assert_eq!(scan.account_error, Some(0));
    assert_eq!(scan.first_error, Some(0));
    assert_eq!(
        discovery.select(scan),
        ClaudeServiceSelection {
            selected: None,
            error: Some(0),
            incomplete: false,
            save_service: "default".into()
        }
    );
}

#[test]
fn claude_discovery_reports_incomplete_without_read_error() {
    let discovery = ClaudeServiceDiscovery::new("default".into());
    let scan = discovery.scan(
        discovery.service_names(vec![]),
        std::sync::Arc::new(Reader(vec![CredentialSourceScan::Incomplete])),
    );
    assert_eq!(scan.account_error, None);
    assert_eq!(
        discovery.select(scan),
        ClaudeServiceSelection {
            selected: None,
            error: None,
            incomplete: true,
            save_service: "default".into()
        }
    );
}

#[test]
fn claude_discovery_returns_no_account_for_missing_item() {
    let discovery = ClaudeServiceDiscovery::new("default".into());
    let scan = discovery.scan(
        discovery.service_names(vec![]),
        std::sync::Arc::new(Reader(vec![CredentialSourceScan::Missing])),
    );
    assert!(scan.available.is_empty());
    assert_eq!(scan.account_error, None);
    assert_eq!(
        discovery.select(scan),
        ClaudeServiceSelection {
            selected: None,
            error: None,
            incomplete: false,
            save_service: "default".into()
        }
    );
}
