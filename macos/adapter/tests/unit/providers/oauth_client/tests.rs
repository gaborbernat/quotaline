//! Checks installed OAuth client discovery across the Swift boundary.

use super::*;

#[test]
fn antigravity_bridge_returns_environment_client() {
    assert_eq!(
        antigravity_oauth_client(
            "/missing".into(),
            std::collections::HashMap::from([
                ("ANTIGRAVITY_OAUTH_CLIENT_ID".into(), "id".into()),
                ("ANTIGRAVITY_OAUTH_CLIENT_SECRET".into(), "secret".into()),
            ]),
        ),
        Some(OAuthClientRecord {
            id: "id".into(),
            secret: "secret".into()
        })
    );
}

#[test]
fn gemini_bridge_reads_home_installation() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder
        .path()
        .join(".npm-global/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        "const OAUTH_CLIENT_ID = 'id'; const OAUTH_CLIENT_SECRET = 'secret';",
    )
    .unwrap();
    assert_eq!(
        gemini_oauth_client(folder.path().to_str().unwrap().into(), Default::default()),
        Some(OAuthClientRecord {
            id: "id".into(),
            secret: "secret".into()
        })
    );
}
