//! Checks macOS application and CLI discovery through filesystem fixtures.

use super::*;

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn antigravity_prefers_environment_client() {
    let folder = tempfile::tempdir().unwrap();
    let environment = HashMap::from([
        ("ANTIGRAVITY_OAUTH_CLIENT_ID".into(), "env-id".into()),
        ("ANTIGRAVITY_OAUTH_CLIENT_SECRET".into(), "env-secret".into()),
    ]);
    assert_eq!(
        antigravity_client(folder.path(), folder.path(), &environment),
        Some(OAuthClient {
            id: "env-id".into(),
            secret: "env-secret".into()
        })
    );
}

#[test]
fn antigravity_reads_home_application_before_system_application() {
    let folder = tempfile::tempdir().unwrap();
    let home = folder.path().join("home");
    let system = folder.path().join("system");
    let secret = format!("{}{}", "GOCSPX-", "abcdefghijklmnopqrstuvwxyz01");
    write(
        &home,
        "Applications/Antigravity.app/Contents/Resources/bin/language_server_macos_arm",
        format!("123-home.apps.googleusercontent.com {secret}").as_bytes(),
    );
    write(
        &system,
        "Gemini.app/Contents/MacOS/Gemini",
        format!("123-system.apps.googleusercontent.com {secret}").as_bytes(),
    );
    assert_eq!(
        antigravity_client(&home, &system, &HashMap::new()).unwrap().id,
        "123-home.apps.googleusercontent.com"
    );
}

#[test]
fn antigravity_finds_system_application_after_missing_home() {
    let folder = tempfile::tempdir().unwrap();
    let secret = format!("{}{}", "GOCSPX-", "abcdefghijklmnopqrstuvwxyz01");
    write(
        folder.path(),
        "Gemini.app/Contents/Resources/app/out/main.js",
        format!("123-system.apps.googleusercontent.com {secret}").as_bytes(),
    );
    assert_eq!(
        antigravity_client(&folder.path().join("home"), folder.path(), &HashMap::new())
            .unwrap()
            .id,
        "123-system.apps.googleusercontent.com"
    );
}

#[test]
fn antigravity_reports_missing_installation() {
    let folder = tempfile::tempdir().unwrap();
    assert_eq!(antigravity_client(folder.path(), folder.path(), &HashMap::new()), None);
}

#[test]
fn gemini_prefers_environment_client() {
    let folder = tempfile::tempdir().unwrap();
    let environment = HashMap::from([
        ("GEMINI_OAUTH_CLIENT_ID".into(), "env-id".into()),
        ("GEMINI_OAUTH_CLIENT_SECRET".into(), "env-secret".into()),
    ]);
    assert_eq!(
        gemini_client(folder.path(), &[], &environment),
        Some(OAuthClient {
            id: "env-id".into(),
            secret: "env-secret".into()
        })
    );
}

#[test]
fn gemini_reads_configured_prefix_before_home() {
    let folder = tempfile::tempdir().unwrap();
    let prefix = folder.path().join("prefix");
    let home = folder.path().join("home");
    let source = "const OAUTH_CLIENT_ID = 'prefix-id'; const OAUTH_CLIENT_SECRET = 'prefix-secret';";
    write(&prefix, GEMINI_PATHS[0], source.as_bytes());
    write(
        &home.join(".npm-global"),
        GEMINI_PATHS[0],
        source.replace("prefix", "home").as_bytes(),
    );
    let environment = HashMap::from([("NPM_CONFIG_PREFIX".into(), prefix.to_string_lossy().into_owned())]);
    assert_eq!(gemini_client(&home, &[], &environment).unwrap().id, "prefix-id");
}

#[test]
fn gemini_reads_system_prefix_after_missing_home() {
    let folder = tempfile::tempdir().unwrap();
    let prefix = folder.path().join("system");
    write(
        &prefix,
        GEMINI_PATHS[2],
        b"const OAUTH_CLIENT_ID = 'id'; const OAUTH_CLIENT_SECRET = 'secret';",
    );
    assert_eq!(
        gemini_client(folder.path(), &[prefix], &HashMap::new()).unwrap().id,
        "id"
    );
}

#[test]
fn gemini_skips_incomplete_installation() {
    let folder = tempfile::tempdir().unwrap();
    write(
        &folder.path().join(".npm-global"),
        GEMINI_PATHS[0],
        b"const OAUTH_CLIENT_ID = 'id';",
    );
    assert_eq!(gemini_client(folder.path(), &[], &HashMap::new()), None);
}
