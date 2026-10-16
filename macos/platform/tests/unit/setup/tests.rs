use super::*;

#[test]
fn setup_catalog_describes_each_provider() {
    for (id, command, sources) in [
        ("claude", "claude auth login", 2),
        ("codex", "codex login", 2),
        ("gemini", "gemini", 2),
        ("antigravity", "agy", 1),
        ("cursor", "cursor-agent login", 2),
        ("copilot", "copilot login", 4),
    ] {
        let setup = metadata(id).unwrap();
        assert_eq!(setup.sign_in_command, command, "{id}");
        assert_eq!(setup.credential_sources.len(), sources, "{id}");
    }
}

#[test]
fn setup_catalog_rejects_unknown_provider() {
    assert_eq!(metadata("missing"), None);
}
