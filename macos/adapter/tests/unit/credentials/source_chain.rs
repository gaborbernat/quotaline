use super::*;

struct Reader(Vec<CredentialSourceScan>);

impl CredentialSourceReader for Reader {
    fn load(&self, index: u32) -> CredentialSourceScan {
        self.0[index as usize].clone()
    }
}

#[test]
fn first_policy_selects_first_valid_after_failure() {
    let selected = select_credential_source(
        CredentialSourcePolicy::First,
        3,
        std::sync::Arc::new(Reader(vec![
            CredentialSourceScan::Failed,
            CredentialSourceScan::Present {
                access_token: "first".into(),
                refresh_token: None,
                expires_at: None,
            },
            CredentialSourceScan::Missing,
        ])),
    );
    assert_eq!(selected.selected, Some(1));
    assert_eq!(selected.error, Some(0));
}

#[test]
fn save_policy_uses_first_source_when_none_are_valid() {
    let selected = select_credential_source(
        CredentialSourcePolicy::Save,
        2,
        std::sync::Arc::new(Reader(vec![
            CredentialSourceScan::Failed,
            CredentialSourceScan::Missing,
        ])),
    );
    assert_eq!(selected.selected, Some(0));
    assert_eq!(selected.error, None);
}

#[test]
fn claude_policy_prefers_newer_same_account_and_retains_last_error() {
    let selected = select_credential_source(
        CredentialSourcePolicy::Claude,
        4,
        std::sync::Arc::new(Reader(vec![
            CredentialSourceScan::Present {
                access_token: "same".into(),
                refresh_token: None,
                expires_at: Some(1.0),
            },
            CredentialSourceScan::Missing,
            CredentialSourceScan::Present {
                access_token: "same".into(),
                refresh_token: None,
                expires_at: Some(2.0),
            },
            CredentialSourceScan::Failed,
        ])),
    );
    assert_eq!(selected.selected, Some(2));
    assert_eq!(selected.error, Some(3));
}

#[test]
fn empty_chain_has_no_selection() {
    let reader = std::sync::Arc::new(Reader(vec![]));
    for policy in [
        CredentialSourcePolicy::First,
        CredentialSourcePolicy::Save,
        CredentialSourcePolicy::Claude,
    ] {
        assert_eq!(
            select_credential_source(policy, 0, reader.clone()),
            CredentialSourceSelection {
                selected: None,
                error: None
            }
        );
    }
}
