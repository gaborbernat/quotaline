use serde_json::json;
use std::collections::HashMap;

use super::*;

#[test]
fn environment_credentials_follow_token_priority() {
    for (environment, expected) in [
        (HashMap::new(), None),
        (
            HashMap::from([("GH_TOKEN".into(), "gh".into())]),
            Some(("gh", "github.com")),
        ),
        (
            HashMap::from([
                ("COPILOT_GITHUB_TOKEN".into(), "copilot".into()),
                ("GH_TOKEN".into(), "gh".into()),
                ("GH_HOST".into(), "ghe.example".into()),
            ]),
            Some(("copilot", "ghe.example")),
        ),
        (
            HashMap::from([
                ("COPILOT_GITHUB_TOKEN".into(), "".into()),
                ("GITHUB_TOKEN".into(), "github".into()),
            ]),
            Some(("github", "github.com")),
        ),
    ] {
        let actual = copilot_environment_credential(&environment);
        assert_eq!(
            actual
                .as_ref()
                .map(|credential| (credential.token.as_str(), credential.host.as_str())),
            expected
        );
    }
}

#[test]
fn cli_accounts_accept_object_array_and_unknown_shapes() {
    let object = copilot_cli_accounts(&json!({"loggedInUsers": {
        "b.example": {"login": "bob", "oauth_token": "second"},
        "a.example": {"username": "ada", "oauthToken": "first"},
        "c.example": "carol",
        "d.example": {}
    }}));
    assert_eq!(
        object.iter().map(|entry| entry.account.as_deref()).collect::<Vec<_>>(),
        [
            Some("https://a.example:ada"),
            Some("https://b.example:bob"),
            Some("https://c.example:carol"),
            Some("d.example"),
        ]
    );
    assert_eq!(
        object[0].credential.as_ref().map(|value| value.token.as_str()),
        Some("first")
    );
    assert_eq!(
        object[1].credential.as_ref().map(|value| value.token.as_str()),
        Some("second")
    );
    assert_eq!(object[2].credential, None);
    let array = copilot_cli_accounts(&json!({"loggedInUsers": [
        {"keychainAccount": "known", "token": "direct", "host": "corp.example"},
        {"token": "anonymous"}
    ]}));
    assert_eq!(array[0].account.as_deref(), Some("known"));
    assert_eq!(array[1].account, None);
    assert_eq!(
        array[1].credential.as_ref().map(|value| value.token.as_str()),
        Some("anonymous")
    );
    assert!(copilot_cli_accounts(&json!({"loggedInUsers": 1})).is_empty());
}

#[test]
fn legacy_credential_prefers_github_and_skips_empty_tokens() {
    let result = copilot_legacy_credential(&json!({
        "z.example": {"oauth_token": "other"},
        "github.com:ada": {"oauth_token": "preferred", "user": "ada"},
        "github.com:empty": {"oauth_token": ""}
    }));
    assert_eq!(
        result,
        Some(CopilotCredential {
            token: "preferred".into(),
            user: Some("ada".into()),
            host: "github.com".into(),
        })
    );
    assert_eq!(
        copilot_legacy_credential(&json!({"": {"oauth_token": "token"}}))
            .unwrap()
            .host,
        "github.com"
    );
    assert_eq!(copilot_legacy_credential(&json!({"x": {}})), None);
    assert_eq!(copilot_legacy_credential(&json!(1)), None);
}

#[test]
fn keyring_credential_reads_json_and_plain_tokens() {
    assert_eq!(
        copilot_keyring_credential(br#"{"access_token":"access","login":"ada"}"#, None).unwrap(),
        Some(CopilotCredential {
            token: "access".into(),
            user: Some("ada".into()),
            host: "github.com".into()
        })
    );
    assert_eq!(
        copilot_keyring_credential(b" token ", Some("http://corp.example:ada")).unwrap(),
        Some(CopilotCredential {
            token: "token".into(),
            user: Some("ada".into()),
            host: "corp.example".into()
        })
    );
    assert_eq!(
        copilot_keyring_credential(br#""quoted""#, Some("https://corp.example:bob")).unwrap(),
        Some(CopilotCredential {
            token: "quoted".into(),
            user: Some("bob".into()),
            host: "corp.example".into()
        })
    );
    assert_eq!(copilot_keyring_credential(br#"{}"#, Some("octocat")).unwrap(), None);
    assert_eq!(copilot_keyring_credential(b"  ", None).unwrap(), None);
}

#[test]
fn keyring_credential_rejects_invalid_data_and_host() {
    for (bytes, account, expected) in [
        (b"\xff".as_slice(), None, CopilotSecretError::InvalidText),
        (b"{".as_slice(), None, CopilotSecretError::InvalidJson),
        (
            br#"{"token":"valid","host":"bad/path"}"#.as_slice(),
            None,
            CopilotSecretError::InvalidHost,
        ),
    ] {
        assert_eq!(copilot_keyring_credential(bytes, account), Err(expected));
    }
    assert_eq!(
        copilot_keyring_credential(b"token", Some("https://bad/path:ada"))
            .unwrap()
            .unwrap()
            .host,
        "github.com"
    );
    assert_eq!(
        copilot_keyring_credential(b"token", Some("octocat"))
            .unwrap()
            .unwrap()
            .user
            .as_deref(),
        Some("octocat")
    );
    assert_eq!(
        copilot_keyring_credential(b"token", Some("https://no-user"))
            .unwrap()
            .unwrap()
            .user,
        None
    );
    assert_eq!(
        copilot_keyring_credential(b"token", Some("https://github.com:ada/bob"))
            .unwrap()
            .unwrap()
            .user,
        None
    );
    assert_eq!(
        copilot_keyring_credential(b"token", Some("bad:"))
            .unwrap()
            .unwrap()
            .user
            .as_deref(),
        Some("bad:")
    );
}
