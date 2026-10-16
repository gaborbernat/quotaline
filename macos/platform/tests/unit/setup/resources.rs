use super::*;

#[test]
fn descriptors_group_resources_by_provider() {
    assert_eq!(
        all()
            .iter()
            .map(|resource| (resource.id, resource.provider, resource.need))
            .collect::<Vec<_>>(),
        [
            ("claude.home", "claude", Need::Optional),
            ("codex.home", "codex", Need::Required),
            ("gemini.home", "gemini", Need::Required),
            ("cursor.support", "cursor", Need::OneOf("cursor")),
            ("cursor.home", "cursor", Need::OneOf("cursor")),
            ("copilot.home", "copilot", Need::OneOf("copilot")),
            ("copilot.config", "copilot", Need::OneOf("copilot")),
        ]
    );
    assert!(all().iter().all(|resource| !resource.explanation.is_empty()));
}

#[test]
fn configured_paths_follow_each_override_mode() {
    let environment = HashMap::from([
        ("CLAUDE_CONFIG_DIR".into(), "/cfg/claude".into()),
        ("GEMINI_CLI_HOME".into(), "/cfg".into()),
        ("XDG_CONFIG_HOME".into(), "/xdg".into()),
    ]);
    let home = Path::new("/home/user");
    for (id, expected) in [
        ("claude.home", "/cfg/claude"),
        ("gemini.home", "/cfg/.gemini"),
        ("copilot.config", "/xdg/github-copilot"),
        ("codex.home", "/home/user/.codex"),
        ("cursor.support", "/home/user/Library/Application Support/Cursor"),
    ] {
        let resource = all().iter().find(|resource| resource.id == id).unwrap();
        assert_eq!(
            configured_path(resource, &environment, home),
            Path::new(expected),
            "{id}"
        );
    }
}

#[test]
fn missing_overrides_use_home_directory() {
    for resource in all() {
        assert_eq!(
            configured_path(resource, &HashMap::new(), Path::new("/home/user")),
            Path::new("/home/user").join(resource.relative_path)
        );
    }
}

#[test]
fn environment_credentials_remove_only_copilot_folder_grants() {
    let all_ids = all().iter().map(|resource| resource.id).collect::<Vec<_>>();
    assert_eq!(required_ids(&HashMap::new()), all_ids);
    assert_eq!(
        required_ids(&HashMap::from([("COPILOT_GITHUB_TOKEN".into(), "token".into())])),
        all_ids[..5]
    );
    assert_eq!(
        required_ids(&HashMap::from([("COPILOT_GITHUB_TOKEN".into(), "".into())])),
        all_ids
    );
}

#[test]
fn only_copilot_environment_source_skips_sandbox_access() {
    for (provider, source, expected) in [
        ("copilot", Some("copilot.environment"), false),
        ("copilot", Some("copilot.keychain"), true),
        ("copilot", None, true),
        ("cursor", Some("copilot.environment"), true),
    ] {
        assert_eq!(needs_access(provider, source), expected);
    }
}

#[test]
fn configured_providers_include_explicit_intervals_and_resource_grants() {
    assert_eq!(
        configured_providers(
            &["claude".into()],
            &["codex".into()],
            &["gemini.home".into(), "unknown".into()],
        ),
        ["claude", "codex", "gemini"]
    );
    assert!(configured_providers(&[], &[], &[]).is_empty());
}

#[test]
fn missing_resources_preserve_descriptor_order() {
    assert_eq!(missing_ids("cursor", &["cursor.home".into()]), ["cursor.support"]);
    assert!(missing_ids("antigravity", &[]).is_empty());
}
