//! Checks credential extraction through the shared readers.

use super::*;

fn binary(id: &str, secret: &str) -> Vec<u8> {
    [
        b"\xff\0prefix=".as_slice(),
        id.as_bytes(),
        b"\0secret=".as_slice(),
        secret.as_bytes(),
    ]
    .concat()
}

fn client(id: &str, secret: &str) -> OAuthClient {
    OAuthClient {
        id: id.into(),
        secret: secret.into(),
    }
}

#[test]
fn antigravity_extracts_credentials_from_binary_data() {
    let secret = format!("{}{}", "GOCSPX-", "abcdefghijklmnopqrstuvwxyz01");
    let id = "123-abc.apps.googleusercontent.com";
    assert_eq!(antigravity_bytes(&binary(id, &secret)), Some(client(id, &secret)));
}

#[test]
fn antigravity_extracts_credentials_across_read_boundaries() {
    let secret = format!("{}{}", "GOCSPX-", "abcdefghijklmnopqrstuvwxyz01");
    let id = "123-abc.apps.googleusercontent.com";
    let mut bytes = vec![0_u8; 65_780];
    bytes.extend(binary(id, &secret));
    assert_eq!(antigravity_reader(bytes.as_slice()), Some(client(id, &secret)));
}

#[test]
fn antigravity_rejects_missing_or_invalid_credentials() {
    let secret = format!("{}{}", "GOCSPX-", "abcdefghijklmnopqrstuvwxyz01");
    for bytes in [
        binary("abc-def.apps.googleusercontent.com", &secret),
        binary("123-abc.apps.googleusercontent.com", "GOCSPX-short"),
        binary(
            "123-abc.apps.googleusercontent.com",
            "GOCSPX-abcdefghijklmnopqrstuvwxyz0!",
        ),
        binary("123-abc.apps.googleusercontent.com", ""),
        binary("", &secret),
    ] {
        assert_eq!(antigravity_bytes(&bytes), None);
    }
}

#[test]
fn antigravity_reads_installed_file_without_loading_it_all() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("language-server");
    let secret = format!("{}{}", "GOCSPX-", "abcdefghijklmnopqrstuvwxyz01");
    let id = "123-abc.apps.googleusercontent.com";
    std::fs::write(&path, binary(id, &secret)).unwrap();
    assert_eq!(antigravity_binary(&path), Some(client(id, &secret)));
    assert_eq!(antigravity_binary(&folder.path().join("missing")), None);
}

#[test]
fn gemini_extracts_quoted_assignments() {
    assert_eq!(
        gemini_source("const OAUTH_CLIENT_ID = 'id'; const OAUTH_CLIENT_SECRET = \"secret\";"),
        Some(client("id", "secret"))
    );
}

#[test]
fn gemini_rejects_incomplete_assignments() {
    for source in [
        "const OAUTH_CLIENT_ID = 'id';",
        "const OAUTH_CLIENT_ID = ''; const OAUTH_CLIENT_SECRET = 'secret';",
        "const OAUTH_CLIENT_ID = 'id'; const OAUTH_CLIENT_SECRET = unquoted;",
        "const OAUTH_CLIENT_ID = 'id'; const OAUTH_CLIENT_SECRET = 'unclosed;",
        "const OAUTH_CLIENT_ID: 'id'; const OAUTH_CLIENT_SECRET = 'secret';",
    ] {
        assert_eq!(gemini_source(source), None, "{source}");
    }
}
