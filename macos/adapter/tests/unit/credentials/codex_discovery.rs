use super::*;
use crate::credentials::{CredentialSourceReader, CredentialSourceScan};

#[test]
fn bridge_returns_the_shared_codex_account_label() {
    assert_eq!(
        codex_account_label(None, "/home/user/.codex".into(), "/home/user".into()),
        "~/.codex"
    );
}

struct Reader(Vec<CredentialSourceScan>);

impl CredentialSourceReader for Reader {
    fn load(&self, index: u32) -> CredentialSourceScan {
        self.0[index as usize].clone()
    }
}

#[test]
fn account_discovery_scans_and_selects_a_sticky_home() {
    let discovery = CodexAccountDiscovery::new(vec!["first".into(), "second".into()]);
    let scan = discovery.scan(std::sync::Arc::new(Reader(vec![
        CredentialSourceScan::Failed,
        CredentialSourceScan::Present {
            access_token: "token".into(),
            refresh_token: None,
            expires_at: None,
        },
    ])));
    assert_eq!(scan.available, [1]);
    assert_eq!(scan.first_error, Some(0));
    assert_eq!(discovery.select(scan.available), Some(1));
    assert_eq!(discovery.selected_home().as_deref(), Some("second"));
    assert_eq!(discovery.save_target(vec![0, 1]), Some(1));
}
